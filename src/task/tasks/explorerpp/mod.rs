// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Explorer++ task implementation.
//!
//! ```text
//! ExplorerPPTask
//! URL → cache/explorerpp_x64.zip → build/explorer++ → install/bin/explorer++
//! ```
//!
//! This task downloads and installs Explorer++ - a file manager replacement
//! that MO2 can launch for browsing mod files.
//!
//! # Process
//!
//! 1. **Fetch**: Download zip from explorerplusplus.com
//! 2. **Extract**: Unpack to build/explorer++/
//! 3. **Install**: Copy to install/bin/explorer++/

use std::path::PathBuf;

use crate::error::Result;
use anyhow::Context;
use futures_util::future::BoxFuture;
use tokio::fs;
use tracing::info;

use crate::config::Config;
use crate::task::tools::Tool;
use crate::task::tools::downloader::DownloaderTool;
use crate::task::tools::extractor::ExtractorTool;
use crate::task::{CleanFlags, TaskContext, Taskable};
use crate::utility::fs::copy::copy_files_async;

/// Explorer++ task for downloading prebuilt Explorer++.
#[derive(Debug, Clone)]
pub struct ExplorerPPTask {
    /// Task name
    name: String,
}

impl Default for ExplorerPPTask {
    fn default() -> Self {
        Self::new()
    }
}

impl ExplorerPPTask {
    /// Create a new Explorer++ task.
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: "explorerpp".to_string(),
        }
    }

    /// Returns the task name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the version from config.
    fn version(config: &Config) -> String {
        config.versions.explorerpp.clone()
    }

    /// Get the download URL.
    fn download_url(config: &Config) -> String {
        let version = Self::version(config);
        format!("https://download.explorerplusplus.com/stable/{version}/explorerpp_x64.zip")
    }

    /// Get the cache file path.
    fn cache_file(config: &Config) -> Result<PathBuf> {
        let cache = config
            .paths
            .cache
            .as_ref()
            .context("paths.cache not configured")?;
        Ok(cache.join("explorerpp_x64.zip"))
    }

    /// Get the source/build path (where it's extracted).
    fn source_path(config: &Config) -> Result<PathBuf> {
        let build = config
            .paths
            .build
            .as_ref()
            .context("paths.build not configured")?;
        Ok(build.join("explorer++"))
    }

    /// Get the install path.
    fn install_path(config: &Config) -> Result<PathBuf> {
        let install_bin = config
            .paths
            .install_bin
            .as_ref()
            .context("paths.install_bin not configured")?;
        Ok(install_bin.join("explorer++"))
    }

    /// Execute the clean phase.
    ///
    /// # Errors
    ///
    /// Returns an error if the cached archive or the extracted directory
    /// cannot be removed.
    pub async fn do_clean(&self, ctx: &TaskContext, flags: CleanFlags) -> Result<()> {
        let config = &ctx.config;
        let tool_ctx = ctx.tool_context();

        // Redownload: delete cached archive
        if flags.contains(CleanFlags::REDOWNLOAD) {
            let cache_file = Self::cache_file(config)?;

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

        // Reextract: delete extracted directory
        if flags.contains(CleanFlags::REEXTRACT) {
            let source_path = Self::source_path(config)?;

            if source_path.exists() {
                if ctx.dry_run {
                    info!(
                        path = %source_path.display(),
                        "[DRY-RUN] would delete extracted directory"
                    );
                } else {
                    info!(path = %source_path.display(), "Deleting extracted directory");
                    fs::remove_dir_all(&source_path)
                        .await
                        .with_context(|| format!("failed to delete {}", source_path.display()))?;
                }
            }
        }

        Ok(())
    }

    /// Execute the fetch phase (download and extract).
    ///
    /// # Errors
    ///
    /// Returns an error if the download or extraction fails.
    pub async fn do_fetch(&self, ctx: &TaskContext) -> Result<()> {
        let config = &ctx.config;
        let tool_ctx = ctx.tool_context();

        let url = Self::download_url(config);
        let cache_file = Self::cache_file(config)?;
        let source_path = Self::source_path(config)?;

        info!(
            version = %Self::version(config),
            "Fetching Explorer++"
        );

        // Download
        let downloader = DownloaderTool::new()
            .url(&url)
            .file(&cache_file)
            .force(config.global.clean_download_actions.redownload);

        downloader
            .run(&tool_ctx)
            .await
            .context("failed to download Explorer++")?;

        // Extract
        let extractor = ExtractorTool::new()
            .archive(&cache_file)
            .output(&source_path)
            .force(config.global.clean_download_actions.reextract);

        extractor
            .run(&tool_ctx)
            .await
            .context("failed to extract Explorer++")?;

        Ok(())
    }

    /// Execute the build and install phase.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The install directory cannot be created.
    /// - Files cannot be copied to the install directory.
    pub async fn do_build_and_install(&self, ctx: &TaskContext) -> Result<()> {
        let config = &ctx.config;
        let source_path = Self::source_path(config)?;
        let install_path = Self::install_path(config)?;

        if !source_path.exists() {
            info!(
                path = %source_path.display(),
                "Explorer++ source not found, skipping install"
            );
            return Ok(());
        }

        // Create install directory if needed
        if !install_path.exists() {
            if ctx.dry_run {
                info!(
                    path = %install_path.display(),
                    "[DRY-RUN] would create explorer++ directory"
                );
            } else {
                fs::create_dir_all(&install_path)
                    .await
                    .with_context(|| format!("failed to create {}", install_path.display()))?;
            }
        }

        info!("Installing Explorer++");

        // Copy all files from source to install
        if ctx.dry_run {
            info!(
                src = %source_path.display(),
                dst = %install_path.display(),
                "[DRY-RUN] would copy Explorer++ files"
            );
        } else {
            copy_files_async(&source_path, &install_path, None).await?;
        }

        Ok(())
    }
}

impl Taskable for ExplorerPPTask {
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
