// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! `ModOrganizer` task implementation.
//!
//! ```text
//! ModOrganizerTask
//! Super repo (build/): cmake_common + modorganizer-* subprojects
//! Pipeline: Fetch → CMake configure → Build & Install
//! CMAKE_PREFIX_PATH: Qt + cmake_common + install/lib/cmake
//! ```

use std::path::{Path, PathBuf};

use crate::error::Result;
use anyhow::Context;
use futures_util::future::BoxFuture;
use tokio::fs;
use tokio::sync::OnceCell;
use tracing::{debug, info};

use crate::config::Config;
use crate::git::cmd::init_repo;
use crate::git::query::is_git_repo;
use crate::task::helpers::safe_remove_source;
use crate::task::tools::Tool;
use crate::task::tools::cmake::{CmakeArchitecture, CmakeGenerator, CmakeTool};
use crate::task::tools::git::GitTool;
use crate::task::{CleanFlags, TaskContext, Taskable};

/// Static initializer for the super repository.
/// Ensures the super repo is initialized only once across all `ModOrganizer` tasks.
static SUPER_INIT: OnceCell<()> = OnceCell::const_new();

/// `ModOrganizer` task for building MO2 projects.
///
/// Each `ModOrganizer` task builds a single subproject (e.g., modorganizer-archive,
/// modorganizer-uibase, etc.). The tasks share a common "super" repository
/// that contains all subprojects as git submodules.
///
/// # Example
///
/// ```ignore
/// let task = ModOrganizerTask::new("archive");
/// task.run(&ctx).await?;
/// ```
#[derive(Debug, Clone)]
pub struct ModOrganizerTask {
    /// The project name (e.g., "archive", "uibase", "`game_features`")
    name: String,

    /// Full repository name (e.g., "modorganizer-archive")
    repo_name: String,
}

impl ModOrganizerTask {
    /// Create a new `ModOrganizer` task for the given project.
    ///
    /// The project name should be the short name (e.g., "archive" not "modorganizer-archive").
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        let repo_name = if name == "modorganizer" || name.starts_with("modorganizer-") {
            name.clone()
        } else {
            format!("modorganizer-{name}")
        };

        Self { name, repo_name }
    }

    /// Returns the task name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the full repository name.
    #[must_use]
    pub fn repo_name(&self) -> &str {
        &self.repo_name
    }

    /// Returns the git URL for this project.
    fn git_url(&self, config: &Config) -> String {
        format!(
            "{}{}/{}.git",
            config.task.git_url_prefix, config.task.mo_org, self.repo_name
        )
    }

    /// Returns the source directory path.
    fn source_path(&self, config: &Config) -> Result<PathBuf> {
        let build_dir = config
            .paths
            .build
            .as_ref()
            .context("paths.build not configured")?;
        Ok(build_dir.join(&self.repo_name))
    }

    /// Returns the super repository path (parent of all modorganizer repos).
    fn super_path(config: &Config) -> Result<PathBuf> {
        config
            .paths
            .build
            .clone()
            .context("paths.build not configured")
    }

    /// Initialize the super repository if not already done.
    ///
    /// This creates an empty git repository in the build directory that will
    /// contain all modorganizer projects as submodules.
    async fn initialize_super(&self, ctx: &TaskContext) -> Result<()> {
        let config = ctx.config();
        let super_path = Self::super_path(config)?;

        SUPER_INIT
            .get_or_try_init(|| async {
                if !super_path.exists() {
                    if ctx.is_dry_run() {
                        info!(
                            path = %super_path.display(),
                            "[DRY-RUN] would create super directory"
                        );
                    } else {
                        fs::create_dir_all(&super_path).await.with_context(|| {
                            format!("failed to create super directory: {}", super_path.display())
                        })?;
                    }
                }

                // Check if already a git repo
                if !is_git_repo(&super_path) {
                    debug!(path = %super_path.display(), "Initializing super repository");

                    if ctx.is_dry_run() {
                        info!(
                            path = %super_path.display(),
                            "[DRY-RUN] would init git repository"
                        );
                    } else {
                        init_repo(&super_path).context("failed to init super repository")?;
                    }
                }

                Ok::<(), anyhow::Error>(())
            })
            .await?;

        Ok(())
    }

    /// Build the `CMAKE_PREFIX_PATH` for this project.
    ///
    /// Includes:
    /// - Qt installation directory
    /// - Super repo `cmake_common` directory
    /// - Install lib/cmake directory
    fn cmake_prefix_path(config: &Config) -> Result<String> {
        let super_path = Self::super_path(config)?;
        let install_path = config
            .paths
            .install
            .as_ref()
            .context("paths.install not configured")?;

        let separator = if cfg!(windows) { ";" } else { ":" };
        let mut paths = Vec::new();

        // Qt installation
        if let Some(qt_install) = &config.paths.qt_install {
            paths.push(qt_install.display().to_string());
        }

        // cmake_common in super repo
        let cmake_common = super_path.join("cmake_common");
        if cmake_common.exists() || cfg!(test) {
            paths.push(cmake_common.display().to_string());
        }

        // install/lib/cmake
        let lib_cmake = install_path.join("lib").join("cmake");
        paths.push(lib_cmake.display().to_string());

        Ok(paths.join(separator))
    }

    /// Check if the source directory has CMakeLists.txt.
    fn has_cmake(source_path: &Path) -> bool {
        source_path.join("CMakeLists.txt").exists()
    }

    /// Check if the source directory has CMakePresets.json.
    fn has_cmake_presets(source_path: &Path) -> bool {
        source_path.join("CMakePresets.json").exists()
    }

    /// Execute the clean phase.
    ///
    /// # Errors
    ///
    /// Returns an error if the source directory cannot be removed, or if the
    /// `CMake` clean operation fails.
    pub async fn do_clean(&self, ctx: &TaskContext, flags: CleanFlags) -> Result<()> {
        let config = ctx.config();
        let source_path = self.source_path(config)?;

        if flags.contains(CleanFlags::REEXTRACT) {
            // Remove source directory (reclone)
            safe_remove_source(ctx, &source_path, "source directory").await?;
            return Ok(());
        }

        if flags.contains(CleanFlags::RECONFIGURE) && source_path.exists() {
            // Clean CMake cache
            let tool_ctx = ctx.tool_context();
            let cmake = CmakeTool::new()
                .source_dir(&source_path)
                .build_dir(&source_path)
                .clean_op();

            if ctx.is_dry_run() {
                info!(
                    path = %source_path.display(),
                    "[DRY-RUN] would clean cmake cache"
                );
            } else {
                cmake
                    .run(&tool_ctx)
                    .await
                    .context("failed to clean cmake cache")?;
            }
        }

        Ok(())
    }

    /// Execute the fetch phase.
    ///
    /// # Errors
    ///
    /// Returns an error if the super repository cannot be initialized,
    /// if the repository cannot be cloned or pulled, or if submodules
    /// cannot be updated.
    pub async fn do_fetch(&self, ctx: &TaskContext) -> Result<()> {
        let config = ctx.config();
        let task_config = config.task_config(&self.name);

        // Initialize super repo first
        self.initialize_super(ctx).await?;

        let source_path = self.source_path(config)?;
        let git_url = self.git_url(config);

        // Use configured branch (fallback logic to be implemented when remote_branch_exists is available)
        let branch = task_config.mo_branch.clone();

        let tool_ctx = ctx.tool_context();

        if source_path.exists() {
            // Pull existing repo
            if task_config.git_behavior.no_pull {
                debug!(path = %source_path.display(), "Skipping pull (no_pull=true)");
                return Ok(());
            }

            info!(
                repo = %self.repo_name,
                branch = %branch,
                "Pulling updates"
            );

            let git = GitTool::new().path(&source_path).branch(&branch).pull_op();

            git.run(&tool_ctx)
                .await
                .with_context(|| format!("failed to pull {}", self.repo_name))?;
        } else {
            // Clone new repo
            info!(
                repo = %self.repo_name,
                url = %git_url,
                branch = %branch,
                "Cloning repository"
            );

            let mut git = GitTool::new()
                .url(&git_url)
                .path(&source_path)
                .branch(&branch)
                .clone_op();

            if task_config.git_clone.git_shallow {
                git = git.shallow(true);
            }

            git.run(&tool_ctx)
                .await
                .with_context(|| format!("failed to clone {}", self.repo_name))?;
        }

        // Update submodules if present
        let gitmodules = source_path.join(".gitmodules");
        if gitmodules.exists() {
            debug!(repo = %self.repo_name, "Updating submodules");

            let git = GitTool::new().path(&source_path).submodule_update_op();

            git.run(&tool_ctx)
                .await
                .with_context(|| format!("failed to update submodules for {}", self.repo_name))?;
        }

        Ok(())
    }

    /// Execute the build and install phase.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - `CMakePresets.json` is missing but `CMakeLists.txt` exists.
    /// - The install prefix is not configured.
    /// - The `CMake` configuration, build, or install operations fail.
    pub async fn do_build_and_install(&self, ctx: &TaskContext) -> Result<()> {
        let config = ctx.config();
        let task_config = config.task_config(&self.name);
        let source_path = self.source_path(config)?;

        // Skip if no CMakeLists.txt
        if !Self::has_cmake(&source_path) {
            debug!(
                repo = %self.repo_name,
                "No CMakeLists.txt, skipping build"
            );
            return Ok(());
        }

        // Require CMakePresets.json for MO projects
        if !Self::has_cmake_presets(&source_path) {
            anyhow::bail!(
                "{} has CMakeLists.txt but no CMakePresets.json. \
                 MO2 projects require CMakePresets.json for configuration.",
                self.repo_name
            );
        }

        let install_prefix = config
            .paths
            .install
            .as_ref()
            .context("paths.install not configured")?;

        let cmake_prefix_path = Self::cmake_prefix_path(config)?;
        let configuration = task_config.configuration;

        let tool_ctx = ctx.tool_context();

        // CMake configure
        info!(
            repo = %self.repo_name,
            config = %configuration,
            "Configuring with CMake"
        );

        let cmake_configure = CmakeTool::new()
            .source_dir(&source_path)
            .build_dir(&source_path)
            .generator(CmakeGenerator::VisualStudio)
            .architecture(CmakeArchitecture::X64)
            .definition("CMAKE_INSTALL_PREFIX", install_prefix.display().to_string())
            .definition("CMAKE_PREFIX_PATH", &cmake_prefix_path)
            .configuration(configuration)
            .configure_op();

        cmake_configure
            .run(&tool_ctx)
            .await
            .with_context(|| format!("failed to configure {}", self.repo_name))?;

        // CMake build
        info!(
            repo = %self.repo_name,
            config = %configuration,
            "Building"
        );

        let cmake_build = CmakeTool::new()
            .source_dir(&source_path)
            .build_dir(&source_path)
            .configuration(configuration)
            .build_op();

        cmake_build
            .run(&tool_ctx)
            .await
            .with_context(|| format!("failed to build {}", self.repo_name))?;

        // CMake install
        info!(
            repo = %self.repo_name,
            prefix = %install_prefix.display(),
            "Installing"
        );

        let cmake_install = CmakeTool::new()
            .source_dir(&source_path)
            .build_dir(&source_path)
            .configuration(configuration)
            .install_op();

        cmake_install
            .run(&tool_ctx)
            .await
            .with_context(|| format!("failed to install {}", self.repo_name))?;

        Ok(())
    }
}

impl Taskable for ModOrganizerTask {
    fn name(&self) -> &str {
        &self.name
    }

    fn enabled(&self, ctx: &TaskContext) -> bool {
        ctx.config().task_config(&self.name).enabled
    }

    fn do_clean<'a>(&'a self, ctx: &'a TaskContext) -> BoxFuture<'a, Result<()>> {
        Box::pin(self.do_clean(ctx, ctx.clean_flags()))
    }

    fn do_fetch<'a>(&'a self, ctx: &'a TaskContext) -> BoxFuture<'a, Result<()>> {
        Box::pin(self.do_fetch(ctx))
    }

    fn do_build_and_install<'a>(&'a self, ctx: &'a TaskContext) -> BoxFuture<'a, Result<()>> {
        Box::pin(self.do_build_and_install(ctx))
    }
}

#[cfg(test)]
mod tests;
