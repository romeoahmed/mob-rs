// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Stylesheets task implementation.
//!
//! ```text
//! StylesheetsTask
//! 9 theme releases (6788-00 + Trosski)
//! Pipeline: GitHub .7z → cache → build/stylesheets → install/bin/stylesheets
//! ```

use std::path::PathBuf;

use crate::error::Result;
use anyhow::Context;
use futures_util::future::BoxFuture;
use tracing::info;

use crate::config::Config;
use crate::task::tools::Tool;
use crate::task::tools::downloader::DownloaderTool;
use crate::task::tools::extractor::ExtractorTool;
use crate::task::{CleanFlags, TaskContext, Taskable};
use crate::utility::fs::copy::copy_dir_contents_async;

/// A stylesheet release definition.
#[derive(Debug, Clone)]
struct StylesheetRelease {
    /// GitHub username
    user: &'static str,
    /// GitHub repository name
    repo: &'static str,
    /// Version key in config (e.g., "`ss_paper_lad_6788`")
    version_key: &'static str,
    /// Filename in release (without .7z extension)
    file: &'static str,
    /// Top-level folder inside archive (empty if files are at root)
    top_level_folder: &'static str,
}

impl StylesheetRelease {
    const fn new(
        user: &'static str,
        repo: &'static str,
        version_key: &'static str,
        file: &'static str,
        top_level_folder: &'static str,
    ) -> Self {
        Self {
            user,
            repo,
            version_key,
            file,
            top_level_folder,
        }
    }
}

/// All stylesheet releases.
const RELEASES: &[StylesheetRelease] = &[
    StylesheetRelease::new(
        "6788-00",
        "paper-light-and-dark",
        "ss_paper_lad_6788",
        "paper-light-and-dark",
        "",
    ),
    StylesheetRelease::new(
        "6788-00",
        "paper-automata",
        "ss_paper_automata_6788",
        "paper-automata",
        "",
    ),
    StylesheetRelease::new(
        "6788-00",
        "paper-mono",
        "ss_paper_mono_6788",
        "paper-mono",
        "",
    ),
    StylesheetRelease::new(
        "6788-00",
        "1809-dark-mode",
        "ss_dark_mode_1809_6788",
        "1809",
        "",
    ),
    StylesheetRelease::new(
        "Trosski",
        "ModOrganizer_Style_Morrowind",
        "ss_morrowind_trosski",
        "Morrowind-MO2-Stylesheet",
        "",
    ),
    StylesheetRelease::new(
        "Trosski",
        "Mod-Organizer-2-Skyrim-Stylesheet",
        "ss_skyrim_trosski",
        "Skyrim-MO2-Stylesheet",
        "",
    ),
    StylesheetRelease::new(
        "Trosski",
        "ModOrganizer_Style_Fallout3",
        "ss_fallout3_trosski",
        "Fallout3-MO2-Stylesheet",
        "",
    ),
    StylesheetRelease::new(
        "Trosski",
        "Mod-Organizer2-Fallout-4-Stylesheet",
        "ss_fallout4_trosski",
        "Fallout4-MO2-Stylesheet",
        "",
    ),
    StylesheetRelease::new(
        "Trosski",
        "Starfield_MO2_Stylesheet",
        "ss_starfield_trosski",
        "Starfield.MO2.Stylsheet",
        "",
    ),
];

/// Stylesheets task for downloading and installing MO2 themes.
#[derive(Debug, Clone)]
pub struct StylesheetsTask {
    /// Task name
    name: String,
}

impl Default for StylesheetsTask {
    fn default() -> Self {
        Self::new()
    }
}

impl StylesheetsTask {
    /// Create a new stylesheets task.
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: "stylesheets".to_string(),
        }
    }

    /// Returns the task name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the version for a release from config.
    fn get_version(config: &Config, release: &StylesheetRelease) -> String {
        config
            .versions
            .stylesheets
            .get(release.version_key)
            .cloned()
            .unwrap_or_else(|| "latest".to_string())
    }

    /// Get the download URL for a release.
    fn download_url(config: &Config, release: &StylesheetRelease) -> String {
        let version = Self::get_version(config, release);
        format!(
            "https://github.com/{}/{}/releases/download/{}/{}.7z",
            release.user, release.repo, version, release.file
        )
    }

    /// Get the cache file path for a release.
    fn cache_file(config: &Config, release: &StylesheetRelease) -> Result<PathBuf> {
        let cache = config
            .paths
            .cache
            .as_ref()
            .context("paths.cache not configured")?;
        Ok(cache.join(format!("{}.7z", release.repo)))
    }

    /// Get the build path for a release (where it's extracted).
    fn build_path(config: &Config, release: &StylesheetRelease) -> Result<PathBuf> {
        let build = config
            .paths
            .build
            .as_ref()
            .context("paths.build not configured")?;
        let version = Self::get_version(config, release);
        Ok(build
            .join("stylesheets")
            .join(format!("{}-{}", release.repo, version)))
    }

    /// Get the install directory for stylesheets.
    fn install_path(config: &Config) -> Result<PathBuf> {
        config
            .paths
            .install_stylesheets
            .clone()
            .context("paths.install_stylesheets not configured")
    }

    /// Execute the clean phase.
    ///
    /// # Errors
    ///
    /// Returns an error if any cached archive or extracted directory cannot be removed.
    pub async fn do_clean(&self, ctx: &TaskContext, flags: CleanFlags) -> Result<()> {
        let config = &ctx.config;
        let tool_ctx = ctx.tool_context();

        // Redownload: delete cached archives
        if flags.contains(CleanFlags::REDOWNLOAD) {
            for release in RELEASES {
                let cache_file = Self::cache_file(config, release)?;

                if cache_file.exists() {
                    let downloader = DownloaderTool::new().file(&cache_file).clean_op();

                    if ctx.dry_run {
                        info!(
                            file = %cache_file.display(),
                            "[DRY-RUN] would delete cached archive"
                        );
                    } else {
                        downloader.run(&tool_ctx).await.with_context(|| {
                            format!("failed to clean cache file: {}", cache_file.display())
                        })?;
                    }
                }
            }
        }

        // Reextract: delete extracted directories
        if flags.contains(CleanFlags::REEXTRACT) {
            for release in RELEASES {
                let build_path = Self::build_path(config, release)?;

                if build_path.exists() {
                    if ctx.dry_run {
                        info!(
                            path = %build_path.display(),
                            "[DRY-RUN] would delete extracted directory"
                        );
                    } else {
                        info!(path = %build_path.display(), "Deleting extracted directory");
                        tokio::fs::remove_dir_all(&build_path)
                            .await
                            .with_context(|| {
                                format!("failed to delete {}", build_path.display())
                            })?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Execute the fetch phase (download and extract).
    ///
    /// # Errors
    ///
    /// Returns an error if any download or extraction fails.
    pub async fn do_fetch(&self, ctx: &TaskContext) -> Result<()> {
        let config = &ctx.config;
        let tool_ctx = ctx.tool_context();

        for release in RELEASES {
            let url = Self::download_url(config, release);
            let cache_file = Self::cache_file(config, release)?;
            let build_path = Self::build_path(config, release)?;

            info!(
                repo = release.repo,
                version = %Self::get_version(config, release),
                "Fetching stylesheet"
            );

            // Download
            let downloader = DownloaderTool::new()
                .url(&url)
                .file(&cache_file)
                .force(config.global.clean_download_actions.redownload);

            downloader
                .run(&tool_ctx)
                .await
                .with_context(|| format!("failed to download {}", release.repo))?;

            // Extract
            let extractor = ExtractorTool::new()
                .archive(&cache_file)
                .output(&build_path)
                .force(config.global.clean_download_actions.reextract);

            extractor
                .run(&tool_ctx)
                .await
                .with_context(|| format!("failed to extract {}", release.repo))?;
        }

        Ok(())
    }

    /// Execute the build and install phase.
    ///
    /// # Errors
    ///
    /// Returns an error if the install directory cannot be created or if
    /// any stylesheet files cannot be copied.
    pub async fn do_build_and_install(&self, ctx: &TaskContext) -> Result<()> {
        let config = &ctx.config;
        let install_path = Self::install_path(config)?;

        // Create install directory if needed
        if !install_path.exists() {
            if ctx.dry_run {
                info!(
                    path = %install_path.display(),
                    "[DRY-RUN] would create stylesheets directory"
                );
            } else {
                tokio::fs::create_dir_all(&install_path)
                    .await
                    .with_context(|| format!("failed to create {}", install_path.display()))?;
            }
        }

        for release in RELEASES {
            let build_path = Self::build_path(config, release)?;

            // Determine source directory (with or without top-level folder)
            let source_path = if release.top_level_folder.is_empty() {
                build_path.clone()
            } else {
                build_path.join(release.top_level_folder)
            };

            if !source_path.exists() {
                info!(
                    path = %source_path.display(),
                    "Stylesheet source not found, skipping"
                );
                continue;
            }

            info!(repo = release.repo, "Installing stylesheet");

            // Copy all files and directories from source to install
            if ctx.dry_run {
                info!(
                    src = %source_path.display(),
                    dst = %install_path.display(),
                    "[DRY-RUN] would copy stylesheet files"
                );
            } else {
                copy_dir_contents_async(&source_path, &install_path).await?;
            }
        }

        Ok(())
    }
}

impl Taskable for StylesheetsTask {
    fn name(&self) -> &str {
        &self.name
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

#[cfg(test)]
mod tests;
