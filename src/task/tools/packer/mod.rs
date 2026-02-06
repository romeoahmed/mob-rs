// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Archive packing tool supporting 7z format.
//!
//! ```text
//! PackerTool
//! Operations: PackDir | PackFiles
//! 7z: 7z a -t7z -mx9 -bd -bb0 <output> <source> [-xr!pattern]...
//! Builder: archive/base_dir/exclude_patterns/files + pack_dir/pack_files
//! Uses: config.tools.sevenz
//! ```
//!
//! Provides capabilities for creating 7z archives from directories or explicit file lists.
//! Supports exclusion patterns for directory-based packing and file list-based packing.

use std::path::{Path, PathBuf};

use crate::error::Result;
use anyhow::Context;
use tempfile::NamedTempFile;
use tokio::io::AsyncWriteExt;
use tracing::{debug, info};

use super::{BoxFuture, Tool, ToolContext};
use crate::core::process::builder::ProcessBuilder;

/// Packer operation to perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PackOperation {
    /// Archive entire directory with exclusion patterns.
    #[default]
    PackDir,
    /// Archive specific files from list.
    PackFiles,
}

/// Packer tool for creating archives using 7z.
///
/// Supports creating 7z archives from either:
/// - A directory with optional exclusion patterns
/// - An explicit list of files
///
/// # Example
///
/// ```ignore
/// let tool = PackerTool::new()
///     .archive("output.7z")
///     .base_dir("source")
///     .exclude_patterns(vec!["*.tmp", "*.log"])
///     .pack_dir_op();
///
/// tool.run(&ctx).await?;
/// ```
#[derive(Debug, Clone)]
pub struct PackerTool {
    archive: Option<PathBuf>,
    base_dir: Option<PathBuf>,
    exclude_patterns: Vec<String>,
    files: Vec<PathBuf>,
    operation: PackOperation,
}

impl PackerTool {
    /// Creates a new `PackerTool` with default settings.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            archive: None,
            base_dir: None,
            exclude_patterns: Vec::new(),
            files: Vec::new(),
            operation: PackOperation::PackDir,
        }
    }

    #[must_use]
    pub fn archive(mut self, path: impl AsRef<Path>) -> Self {
        self.archive = Some(path.as_ref().to_path_buf());
        self
    }

    #[must_use]
    pub fn base_dir(mut self, path: impl AsRef<Path>) -> Self {
        self.base_dir = Some(path.as_ref().to_path_buf());
        self
    }

    #[must_use]
    pub fn exclude_patterns(mut self, patterns: impl IntoIterator<Item = impl AsRef<str>>) -> Self {
        self.exclude_patterns = patterns
            .into_iter()
            .map(|p| p.as_ref().to_string())
            .collect();
        self
    }

    #[must_use]
    pub fn files(mut self, files: impl IntoIterator<Item = impl AsRef<Path>>) -> Self {
        self.files = files
            .into_iter()
            .map(|p| p.as_ref().to_path_buf())
            .collect();
        self
    }

    #[must_use]
    pub const fn pack_dir_op(mut self) -> Self {
        self.operation = PackOperation::PackDir;
        self
    }

    #[must_use]
    pub const fn pack_files_op(mut self) -> Self {
        self.operation = PackOperation::PackFiles;
        self
    }

    fn archive_required(&self) -> Result<&Path> {
        self.archive
            .as_deref()
            .context("PackerTool: archive path is required")
    }

    fn base_dir_required(&self) -> Result<&Path> {
        self.base_dir
            .as_deref()
            .context("PackerTool: base directory is required for PackDir operation")
    }

    async fn pack_dir(&self, ctx: &ToolContext) -> Result<()> {
        let archive = self.archive_required()?;
        let base_dir = self.base_dir_required()?;

        if ctx.is_dry_run() {
            info!(
                archive = %archive.display(),
                base_dir = %base_dir.display(),
                exclude_patterns = ?self.exclude_patterns,
                "[dry-run] Would create archive from directory"
            );
            return Ok(());
        }

        debug!(
            archive = %archive.display(),
            base_dir = %base_dir.display(),
            exclude_patterns = ?self.exclude_patterns,
            "Creating archive from directory"
        );

        archive_from_glob(ctx, base_dir, archive, &self.exclude_patterns).await?;

        info!(
            archive = %archive.display(),
            base_dir = %base_dir.display(),
            "Archive created successfully"
        );
        Ok(())
    }

    async fn pack_files(&self, ctx: &ToolContext) -> Result<()> {
        let archive = self.archive_required()?;
        let base_dir = self.base_dir_required()?;

        if ctx.is_dry_run() {
            info!(
                archive = %archive.display(),
                base_dir = %base_dir.display(),
                file_count = self.files.len(),
                "[dry-run] Would create archive from file list"
            );
            return Ok(());
        }

        debug!(
            archive = %archive.display(),
            base_dir = %base_dir.display(),
            file_count = self.files.len(),
            "Creating archive from file list"
        );

        archive_from_files(ctx, &self.files, base_dir, archive).await?;

        info!(
            archive = %archive.display(),
            file_count = self.files.len(),
            "Archive created successfully"
        );
        Ok(())
    }
}

impl Default for PackerTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for PackerTool {
    fn name(&self) -> &'static str {
        "packer"
    }

    fn run<'a>(&'a self, ctx: &'a ToolContext) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            match self.operation {
                PackOperation::PackDir => self.pack_dir(ctx).await,
                PackOperation::PackFiles => self.pack_files(ctx).await,
            }
        })
    }
}

/// Creates a 7z archive from a directory with glob exclusion patterns.
///
/// # Arguments
/// * `ctx` - Tool context with configuration and cancellation token
/// * `base_dir` - Directory to archive
/// * `output` - Output archive path
/// * `excludes` - List of glob patterns to exclude (e.g., `["*.tmp", "*.log"]`)
///
/// # 7z Command Format
/// `7z a -t7z -mx9 -bd -bb0 <output> <base_dir>/* -xr!<pattern1> -xr!<pattern2> ...`
///
/// # Errors
///
/// Returns an error if the 7z command fails or is interrupted.
pub async fn archive_from_glob(
    ctx: &ToolContext,
    base_dir: &Path,
    output: &Path,
    excludes: &[String],
) -> Result<()> {
    let mut builder = ProcessBuilder::new(&ctx.config().tools.sevenz);

    builder = builder
        .arg("a")
        .arg("-t7z")
        .arg("-mx9")
        .arg("-bd")
        .arg("-bb0")
        .arg(output);

    let glob_pattern = format!("{}/*", base_dir.display());
    builder = builder.arg(&glob_pattern);

    for pattern in excludes {
        builder = builder.arg(format!("-xr!{pattern}"));
    }

    let output_result = builder
        .run_with_cancellation(ctx.cancel_token().clone())
        .await
        .context("Failed to run 7z archive creation")?;

    if output_result.is_interrupted() {
        anyhow::bail!("Archive creation was interrupted");
    }

    if output_result.exit_code() != 0 {
        anyhow::bail!(
            "7z archive creation failed with exit code: {}",
            output_result.exit_code()
        );
    }

    Ok(())
}

/// Creates a 7z archive from an explicit list of files.
///
/// # Arguments
/// * `ctx` - Tool context with configuration and cancellation token
/// * `files` - List of files to archive
/// * `base_dir` - Base directory for relative path resolution
/// * `output` - Output archive path
///
/// # 7z Command Format
/// `7z a -t7z -mx9 -bd -bb0 <output> @<listfile>`
///
/// The listfile contains one file path per line.
///
/// # Errors
///
/// Returns an error if:
/// - The temporary list file cannot be created or written.
/// - The 7z command fails or is interrupted.
pub async fn archive_from_files(
    ctx: &ToolContext,
    files: &[PathBuf],
    base_dir: &Path,
    output: &Path,
) -> Result<()> {
    // Use NamedTempFile for RAII cleanup - automatically deleted on drop
    let list_file = NamedTempFile::new_in(base_dir)
        .with_context(|| format!("Failed to create temp file in {}", base_dir.display()))?;

    let file_list = files
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join("\n");

    // Write using tokio for async compatibility
    let mut async_file =
        tokio::fs::File::from_std(list_file.reopen().with_context(|| {
            format!("Failed to reopen temp file {}", list_file.path().display())
        })?);
    async_file
        .write_all(file_list.as_bytes())
        .await
        .with_context(|| {
            format!(
                "Failed to write file list to {}",
                list_file.path().display()
            )
        })?;
    async_file.flush().await.with_context(|| {
        format!(
            "Failed to flush file list to {}",
            list_file.path().display()
        )
    })?;
    // Drop async_file to release the handle before 7z reads it
    drop(async_file);

    let mut builder = ProcessBuilder::new(&ctx.config().tools.sevenz);

    builder = builder
        .arg("a")
        .arg("-t7z")
        .arg("-mx9")
        .arg("-bd")
        .arg("-bb0")
        .arg(output)
        .arg(format!("@{}", list_file.path().display()));

    let output_result = builder
        .run_with_cancellation(ctx.cancel_token().clone())
        .await
        .context("Failed to run 7z archive creation")?;

    // NamedTempFile automatically cleans up on drop here

    if output_result.is_interrupted() {
        anyhow::bail!("Archive creation was interrupted");
    }

    if output_result.exit_code() != 0 {
        anyhow::bail!(
            "7z archive creation failed with exit code: {}",
            output_result.exit_code()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests;
