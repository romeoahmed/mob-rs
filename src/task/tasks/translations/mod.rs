// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Translations task implementation.
//!
//! This task manages MO2 translations using Transifex and Qt's lrelease tool.
//!
//! # Process
//!
//! 1. **Fetch**: Initialize transifex, configure, and pull translations from Transifex
//! 2. **Build**: Compile .ts files to .qm using lrelease
//! 3. **Install**: Copy Qt builtin translations
//!
//! # Directory Structure
//!
//! ```text
//! build/transifex-translations/
//!   .tx/                 # Transifex config
//!   translations/        # Downloaded translations
//!     mod-organizer-2.bsa_extractor/
//!     mod-organizer-2.bsa_packer/
//!     ...
//!       de.ts
//!       fr.ts
//!       ...
//! ```

use std::path::{Path, PathBuf};

use crate::error::Result;
use crate::task::helpers::{copy_file_if_newer, ensure_dir};
use crate::task::tools::Tool;
use crate::task::tools::lrelease::LreleaseTool;
use crate::task::tools::transifex::TransifexTool;
use crate::task::{CleanFlags, TaskContext, Taskable};
use anyhow::Context;
use futures_util::future::BoxFuture;
use tracing::{debug, info, warn};

/// Translations task for managing MO2 translations.
#[derive(Debug, Clone)]
pub struct TranslationsTask {
    /// Task name
    name: String,
}

impl Default for TranslationsTask {
    fn default() -> Self {
        Self::new()
    }
}

impl TranslationsTask {
    /// Create a new translations task.
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: "translations".to_string(),
        }
    }

    /// Returns the task name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the source path for transifex directory.
    fn source_path(ctx: &TaskContext) -> Result<PathBuf> {
        let build = ctx
            .config
            .paths
            .build
            .as_ref()
            .context("paths.build not configured")?;
        Ok(build.join("transifex-translations"))
    }

    /// Get the translations subdirectory inside `source_path`.
    fn translations_path(ctx: &TaskContext) -> Result<PathBuf> {
        Ok(Self::source_path(ctx)?.join("translations"))
    }

    /// Get the install path for translations (.qm files).
    fn install_path(ctx: &TaskContext) -> Result<PathBuf> {
        ctx.config()
            .paths
            .install_translations
            .clone()
            .context("paths.install_translations not configured")
    }

    /// Get the Qt translations path for builtin translations.
    fn qt_translations_path(ctx: &TaskContext) -> Option<PathBuf> {
        ctx.config().paths.qt_translations.clone()
    }

    /// Execute the clean phase.
    ///
    /// # Errors
    ///
    /// Returns an error if the transifex directory or .qm files cannot be removed.
    pub async fn do_clean(&self, ctx: &TaskContext, flags: CleanFlags) -> Result<()> {
        // Redownload: delete entire transifex directory
        if flags.contains(CleanFlags::REDOWNLOAD) {
            let source = Self::source_path(ctx)?;
            if source.exists() {
                if ctx.dry_run {
                    info!(
                        path = %source.display(),
                        "[DRY-RUN] would delete transifex directory"
                    );
                } else {
                    info!(path = %source.display(), "Deleting transifex directory");
                    tokio::fs::remove_dir_all(&source)
                        .await
                        .with_context(|| format!("failed to delete {}", source.display()))?;
                }
            }
        }

        // Rebuild: delete .qm files in install directory
        if flags.contains(CleanFlags::REBUILD) {
            let install = Self::install_path(ctx)?;
            if install.exists() {
                if ctx.dry_run {
                    info!(
                        path = %install.display(),
                        "[DRY-RUN] would delete .qm files"
                    );
                } else {
                    delete_qm_files(&install).await?;
                }
            }
        }

        Ok(())
    }

    /// Execute the fetch phase.
    ///
    /// # Errors
    ///
    /// Returns an error if Transifex initialization, configuration, or pulling fails.
    pub async fn do_fetch(&self, ctx: &TaskContext) -> Result<()> {
        let config = &ctx.config;
        let tool_ctx = ctx.tool_context();
        let source = Self::source_path(ctx)?;

        // Check for API key
        let api_key = if !config.transifex.key.is_empty() {
            config.transifex.key.clone()
        } else if let Ok(key) = std::env::var("TX_TOKEN") {
            key
        } else {
            warn!(
                "No Transifex API key found in config or TX_TOKEN environment variable. \
                 This will probably fail."
            );
            String::new()
        };

        // Build the Transifex URL
        let tx_url = format!(
            "{}/{}/{}/dashboard",
            config.transifex.url, config.transifex.team, config.transifex.project
        );

        // 1. Initialize transifex directory
        info!("Initializing transifex directory");
        let init_tool = TransifexTool::new().root(&source).init_op();
        init_tool
            .run(&tool_ctx)
            .await
            .context("failed to initialize transifex")?;

        // 2. Configure (if enabled)
        if config.transifex.actions.configure {
            info!("Configuring transifex remote");
            let config_tool = TransifexTool::new()
                .root(&source)
                .api_key(&api_key)
                .url(&tx_url)
                .config_op();
            config_tool
                .run(&tool_ctx)
                .await
                .context("failed to configure transifex")?;
        } else {
            debug!("Skipping transifex configuration");
        }

        // 3. Pull translations (if enabled)
        if config.transifex.actions.pull {
            info!("Pulling translations from Transifex");
            let pull_tool = TransifexTool::new()
                .root(&source)
                .api_key(&api_key)
                .minimum(config.transifex.minimum)
                .force(config.transifex.actions.force)
                .pull_op();
            pull_tool
                .run(&tool_ctx)
                .await
                .context("failed to pull translations")?;
        } else {
            debug!("Skipping transifex pull");
        }

        Ok(())
    }

    /// Execute the build and install phase.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The install directory cannot be created.
    /// - Translation projects cannot be discovered.
    /// - Translation compilation (`lrelease`) fails.
    /// - Builtin Qt translations cannot be copied.
    pub async fn do_build_and_install(&self, ctx: &TaskContext) -> Result<()> {
        let tool_ctx = ctx.tool_context();
        let translations = Self::translations_path(ctx)?;
        let install = Self::install_path(ctx)?;

        // Create install directory if needed
        ensure_dir(ctx, &install, "translations directory").await?;

        // Walk project directories
        if !translations.exists() {
            warn!(
                path = %translations.display(),
                "Translations directory not found. Run fetch first."
            );
            return Ok(());
        }

        let projects = discover_projects(&translations).await?;

        if projects.is_empty() {
            warn!("No translation projects found");
            return Ok(());
        }

        info!(count = projects.len(), "Found translation projects");

        // Compile each project's translations
        for project in &projects {
            for ts_file in project.ts_files() {
                let lang = ts_file
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown");

                debug!(
                    project = %project.name(),
                    lang,
                    "Compiling translation"
                );

                let lrelease = LreleaseTool::new()
                    .project(project.name())
                    .add_source(ts_file)
                    .output_dir(&install);

                lrelease
                    .run(&tool_ctx)
                    .await
                    .with_context(|| format!("failed to compile {}_{}", project.name(), lang))?;
            }
        }

        // Copy Qt builtin translations
        if let Some(project) = projects.iter().find(|p| p.name() == "organizer") {
            self.copy_builtin_qt_translations(ctx, project, &install)
                .await?;
        } else {
            warn!("Organizer project not found, skipping Qt builtin translations");
        }

        Ok(())
    }

    /// Copy Qt builtin translations (qt_*.qm, qtbase_*.qm).
    async fn copy_builtin_qt_translations(
        &self,
        ctx: &TaskContext,
        organizer_project: &TranslationProject,
        install: &Path,
    ) -> Result<()> {
        let Some(qt_translations) = Self::qt_translations_path(ctx) else {
            warn!("Qt translations path not configured, skipping builtin translations");
            return Ok(());
        };

        if !qt_translations.exists() {
            warn!(
                path = %qt_translations.display(),
                "Qt translations directory not found"
            );
            return Ok(());
        }

        let prefixes = ["qt", "qtbase"];

        for prefix in &prefixes {
            debug!(prefix, "Copying builtin Qt translations");

            for ts_file in organizer_project.ts_files() {
                let Some(lang) = ts_file.file_stem().and_then(|s| s.to_str()) else {
                    continue;
                };

                // Try full language code first (e.g., "zh_CN")
                let qm_file = format!("{prefix}_{lang}.qm");
                let src_path = qt_translations.join(&qm_file);

                if src_path.exists() {
                    copy_file_if_newer(ctx, &src_path, &install.join(&qm_file), "Qt translation")
                        .await?;
                    continue;
                }

                // Try just the language part (e.g., "zh" from "zh_CN")
                if let Some(short_lang) = lang.split('_').next()
                    && short_lang != lang
                {
                    let qm_file = format!("{prefix}_{short_lang}.qm");
                    let src_path = qt_translations.join(&qm_file);

                    if src_path.exists() {
                        copy_file_if_newer(
                            ctx,
                            &src_path,
                            &install.join(&qm_file),
                            "Qt translation",
                        )
                        .await?;
                        continue;
                    }
                }

                debug!(prefix, lang, "Missing builtin Qt translation");
            }
        }

        Ok(())
    }
}

impl Taskable for TranslationsTask {
    fn name(&self) -> &str {
        &self.name
    }

    fn enabled(&self, ctx: &TaskContext) -> bool {
        ctx.config().transifex.enabled
    }

    fn do_clean<'a>(&'a self, ctx: &'a TaskContext) -> BoxFuture<'a, Result<()>> {
        Box::pin(self.do_clean(ctx, ctx.clean_flags))
    }

    fn do_fetch<'a>(&'a self, ctx: &'a TaskContext) -> BoxFuture<'a, Result<()>> {
        Box::pin(self.do_fetch(ctx))
    }

    fn do_build_and_install<'a>(&'a self, ctx: &'a TaskContext) -> BoxFuture<'a, Result<()>> {
        Box::pin(self.do_build_and_install(ctx))
    }
}

/// A translation project discovered in the translations directory.
#[derive(Debug)]
pub struct TranslationProject {
    /// Project name (e.g., "`bsa_extractor`")
    name: String,
    /// List of .ts files
    ts_files: Vec<PathBuf>,
}

impl TranslationProject {
    /// Creates a new `TranslationProject`.
    #[must_use]
    pub const fn new(name: String, ts_files: Vec<PathBuf>) -> Self {
        Self { name, ts_files }
    }

    /// Returns the project name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the list of .ts files.
    #[must_use]
    pub fn ts_files(&self) -> &[PathBuf] {
        &self.ts_files
    }
}

/// Discover translation projects in the translations directory.
///
/// # Errors
///
/// Returns an error if the translations directory cannot be read.
pub async fn discover_projects(translations_dir: &Path) -> Result<Vec<TranslationProject>> {
    let mut projects = Vec::new();

    let mut entries = tokio::fs::read_dir(translations_dir)
        .await
        .with_context(|| format!("failed to read {}", translations_dir.display()))?;

    while let Some(entry) = entries
        .next_entry()
        .await
        .with_context(|| format!("failed to read entry in {}", translations_dir.display()))?
    {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        // Directory name is like "mod-organizer-2.bsa_extractor"
        let Some(dir_name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };

        // Parse project name from directory name
        let Some(project_name) = parse_project_name(dir_name) else {
            warn!(dir = dir_name, "Invalid project directory name, skipping");
            continue;
        };

        // Find all .ts files in the project
        let ts_files = find_ts_files(&path).await?;

        if ts_files.is_empty() {
            debug!(project = project_name, "No .ts files found, skipping");
            continue;
        }

        projects.push(TranslationProject::new(project_name, ts_files));
    }

    // Sort for deterministic order
    projects.sort_by(|a, b| a.name().cmp(b.name()));

    Ok(projects)
}

/// Parse project name from transifex directory name.
///
/// Directory names are like "mod-organizer-2.bsa_extractor", we want "`bsa_extractor`".
fn parse_project_name(dir_name: &str) -> Option<String> {
    let parts: Vec<&str> = dir_name.splitn(2, '.').collect();
    if parts.len() != 2 {
        return None;
    }

    let project = parts[1].trim();
    if project.is_empty() {
        return None;
    }

    Some(project.to_string())
}

/// Find all .ts files in a directory.
async fn find_ts_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut ts_files = Vec::new();

    let mut entries = tokio::fs::read_dir(dir)
        .await
        .with_context(|| format!("failed to read {}", dir.display()))?;

    while let Some(entry) = entries
        .next_entry()
        .await
        .with_context(|| format!("failed to read entry in {}", dir.display()))?
    {
        let path = entry.path();
        if path.is_file()
            && let Some(ext) = path.extension()
            && ext == "ts"
        {
            ts_files.push(path);
        }
    }

    // Sort for deterministic order
    ts_files.sort();

    Ok(ts_files)
}

/// Delete all .qm files in a directory.
async fn delete_qm_files(dir: &Path) -> Result<()> {
    let mut entries = tokio::fs::read_dir(dir)
        .await
        .with_context(|| format!("failed to read {}", dir.display()))?;

    while let Some(entry) = entries
        .next_entry()
        .await
        .with_context(|| format!("failed to read entry in {}", dir.display()))?
    {
        let path = entry.path();
        if path.is_file()
            && let Some(ext) = path.extension()
            && ext == "qm"
        {
            info!(file = %path.display(), "Deleting .qm file");
            tokio::fs::remove_file(&path)
                .await
                .with_context(|| format!("failed to delete {}", path.display()))?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests;
