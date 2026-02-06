// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Downloader tool for HTTP file downloads.
//!
//! ```text
//! URLs --> HTTP GET --> progress --> local file
//! Features: fallback URLs, cache skip, force re-download, cancel
//! Uses: crate::net::Downloader + ProgressDisplay::Bar
//! ```

use std::path::PathBuf;

use crate::error::Result;
use anyhow::Context;
use tracing::{debug, info};

use super::{BoxFuture, Tool, ToolContext};
use crate::net::{Downloader, ProgressDisplay};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DownloaderOperation {
    #[default]
    Download,
    Clean,
}

#[derive(Debug, Clone)]
pub struct DownloaderTool {
    urls: Vec<String>,
    output_file: Option<PathBuf>,
    force: bool,
    operation: DownloaderOperation,
}

impl Default for DownloaderTool {
    fn default() -> Self {
        Self::new()
    }
}

impl DownloaderTool {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            urls: Vec::new(),
            output_file: None,
            force: false,
            operation: DownloaderOperation::Download,
        }
    }

    /// Add a URL to download from. Multiple URLs will be tried in order.
    #[must_use]
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.urls.push(url.into());
        self
    }

    #[must_use]
    pub fn urls(mut self, urls: Vec<String>) -> Self {
        self.urls = urls;
        self
    }

    #[must_use]
    pub fn file(mut self, path: impl Into<PathBuf>) -> Self {
        self.output_file = Some(path.into());
        self
    }

    #[must_use]
    pub const fn force(mut self, force: bool) -> Self {
        self.force = force;
        self
    }

    #[must_use]
    pub const fn download_op(mut self) -> Self {
        self.operation = DownloaderOperation::Download;
        self
    }

    #[must_use]
    pub const fn clean_op(mut self) -> Self {
        self.operation = DownloaderOperation::Clean;
        self
    }

    async fn execute_download(&self, ctx: &ToolContext) -> Result<()> {
        let output_file = self
            .output_file
            .as_ref()
            .context("no output file specified")?;

        // Check if cancellation was requested
        if ctx.is_cancelled() {
            return Err(anyhow::anyhow!("download cancelled"));
        }

        // Check if file already exists and we're not forcing re-download
        if !self.force && output_file.exists() {
            info!(
                path = %output_file.display(),
                "file already exists, skipping download"
            );
            return Ok(());
        }

        if self.urls.is_empty() {
            return Err(anyhow::anyhow!("no URLs provided for download"));
        }

        // Try each URL in order
        let mut last_error = None;
        for (idx, url) in self.urls.iter().enumerate() {
            // Check for cancellation before each attempt
            if ctx.is_cancelled() {
                return Err(anyhow::anyhow!("download cancelled"));
            }

            debug!(
                url = %url,
                attempt = idx + 1,
                total = self.urls.len(),
                "attempting download"
            );

            if ctx.is_dry_run() {
                info!(
                    url = %url,
                    file = %output_file.display(),
                    "[DRY-RUN] would download"
                );
                return Ok(());
            }

            let downloader = Downloader::new()
                .url(url)
                .file(output_file)
                .progress(ProgressDisplay::Bar);

            match downloader.download().await {
                Ok(()) => {
                    info!(
                        url = %url,
                        file = %output_file.display(),
                        "download completed successfully"
                    );
                    return Ok(());
                }
                Err(e) => {
                    debug!(
                        url = %url,
                        error = %e,
                        "download attempt failed, trying next URL"
                    );
                    last_error = Some(e);
                    // Continue to next URL
                }
            }
        }

        // All URLs failed
        last_error.map_or_else(
            || Err(anyhow::anyhow!("no URLs provided for download")),
            |error| Err(error).context("all download URLs failed"),
        )
    }

    async fn execute_clean(&self, ctx: &ToolContext) -> Result<()> {
        let output_file = self
            .output_file
            .as_ref()
            .context("no output file specified")?;

        if ctx.is_dry_run() {
            info!(
                file = %output_file.display(),
                "[DRY-RUN] would delete"
            );
            return Ok(());
        }

        if output_file.exists() {
            tokio::fs::remove_file(output_file)
                .await
                .with_context(|| format!("failed to delete {}", output_file.display()))?;
            info!(file = %output_file.display(), "file deleted");
        } else {
            debug!(file = %output_file.display(), "file does not exist, nothing to clean");
        }

        Ok(())
    }
}

impl Tool for DownloaderTool {
    fn name(&self) -> &'static str {
        "downloader"
    }

    fn run<'a>(&'a self, ctx: &'a ToolContext) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            match self.operation {
                DownloaderOperation::Download => self.execute_download(ctx).await,
                DownloaderOperation::Clean => self.execute_clean(ctx).await,
            }
        })
    }
}

#[cfg(test)]
mod tests;
