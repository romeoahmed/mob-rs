// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Archive extraction tool supporting multiple formats via 7z.
//!
//! ```text
//! .7z | .zip | .tar.gz | .tar --> 7z x ... --> output_dir
//! ```

use std::path::{Path, PathBuf};

use crate::error::Result;
use anyhow::Context;
use tokio::fs;
use tracing::{debug, info};

use super::{BoxFuture, Tool, ToolContext};
use crate::core::process::builder::ProcessBuilder;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveFormat {
    SevenZip,
    Zip,
    TarGz,
    Tar,
}

impl ArchiveFormat {
    /// Detects archive format from file extension.
    fn from_extension(path: &Path) -> Option<Self> {
        let filename = path.file_name()?.to_str()?;

        // Helper for case-insensitive suffix matching
        let ends_with_ci = |s: &str, suffix: &str| {
            s.len() >= suffix.len() && s[s.len() - suffix.len()..].eq_ignore_ascii_case(suffix)
        };

        // Check compound extensions first
        if ends_with_ci(filename, ".tar.gz") || ends_with_ci(filename, ".tgz") {
            return Some(Self::TarGz);
        }

        // Check simple extensions
        let ext = path.extension()?.to_str()?;
        if ext.eq_ignore_ascii_case("7z") {
            Some(Self::SevenZip)
        } else if ext.eq_ignore_ascii_case("zip") {
            Some(Self::Zip)
        } else if ext.eq_ignore_ascii_case("tar") {
            Some(Self::Tar)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExtractorOperation {
    #[default]
    Extract,
    Clean,
}

#[derive(Debug, Clone)]
pub struct ExtractorTool {
    archive: Option<PathBuf>,
    output_dir: Option<PathBuf>,
    format: Option<ArchiveFormat>,
    force: bool,
    operation: ExtractorOperation,
}

impl ExtractorTool {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            archive: None,
            output_dir: None,
            format: None,
            force: false,
            operation: ExtractorOperation::Extract,
        }
    }

    #[must_use]
    pub fn archive(mut self, path: impl AsRef<Path>) -> Self {
        self.archive = Some(path.as_ref().to_path_buf());
        self
    }

    #[must_use]
    pub fn output(mut self, path: impl AsRef<Path>) -> Self {
        self.output_dir = Some(path.as_ref().to_path_buf());
        self
    }

    /// Explicitly sets the archive format (auto-detection used if not specified).
    #[must_use]
    pub const fn format(mut self, format: ArchiveFormat) -> Self {
        self.format = Some(format);
        self
    }

    #[must_use]
    pub const fn force(mut self, force: bool) -> Self {
        self.force = force;
        self
    }

    #[must_use]
    pub const fn extract_op(mut self) -> Self {
        self.operation = ExtractorOperation::Extract;
        self
    }

    #[must_use]
    pub const fn clean_op(mut self) -> Self {
        self.operation = ExtractorOperation::Clean;
        self
    }

    fn archive_required(&self) -> Result<&Path> {
        self.archive
            .as_deref()
            .context("ExtractorTool: archive path is required")
    }

    fn output_dir_required(&self) -> Result<&Path> {
        self.output_dir
            .as_deref()
            .context("ExtractorTool: output directory is required")
    }

    fn detect_format(&self) -> Result<ArchiveFormat> {
        if let Some(format) = self.format {
            return Ok(format);
        }

        let archive = self.archive_required()?;
        ArchiveFormat::from_extension(archive).context(
            "Failed to detect archive format from extension. Supported: .7z, .zip, .tar.gz, .tgz, .tar",
        )
    }

    async fn do_extract(&self, ctx: &ToolContext) -> Result<()> {
        let archive = self.archive_required()?;
        let output_dir = self.output_dir_required()?;
        let format = self.detect_format()?;

        // Skip if output exists and not forcing
        if output_dir.exists() && !self.force {
            info!(
                output = %output_dir.display(),
                "Output directory exists, skipping extraction"
            );
            return Ok(());
        }

        if ctx.is_dry_run() {
            info!(
                archive = %archive.display(),
                output = %output_dir.display(),
                format = ?format,
                force = self.force,
                "[dry-run] Would extract archive"
            );
            return Ok(());
        }

        // Create output directory if it doesn't exist
        if !output_dir.exists() {
            fs::create_dir_all(output_dir).await.with_context(|| {
                format!(
                    "Failed to create output directory: {}",
                    output_dir.display()
                )
            })?;
        }

        debug!(
            archive = %archive.display(),
            output = %output_dir.display(),
            format = ?format,
            "Extracting archive"
        );

        match format {
            ArchiveFormat::TarGz => self.extract_tar_gz(ctx, archive, output_dir).await?,
            _ => self.extract_with_7z(ctx, archive, output_dir).await?,
        }

        info!(
            archive = %archive.display(),
            output = %output_dir.display(),
            "Archive extracted successfully"
        );
        Ok(())
    }

    async fn extract_with_7z(
        &self,
        ctx: &ToolContext,
        archive: &Path,
        output_dir: &Path,
    ) -> Result<()> {
        let mut builder = ProcessBuilder::new(&ctx.config().tools.sevenz);

        builder = builder
            .arg("x")
            .arg("-aoa")
            .arg("-bd")
            .arg("-bb0")
            .arg(format!("-o{}", output_dir.display()))
            .arg(archive);

        let output = builder
            .run_with_cancellation(ctx.cancel_token().clone())
            .await
            .context("Failed to run 7z extraction")?;

        if output.is_interrupted() {
            anyhow::bail!("Archive extraction was interrupted");
        }

        if output.exit_code() != 0 {
            anyhow::bail!(
                "7z extraction failed with exit code: {}",
                output.exit_code()
            );
        }

        Ok(())
    }

    async fn extract_tar_gz(
        &self,
        ctx: &ToolContext,
        archive: &Path,
        output_dir: &Path,
    ) -> Result<()> {
        // For tar.gz, use 7z directly which handles both decompression and extraction
        let mut cmd = ProcessBuilder::new(&ctx.config().tools.sevenz);
        cmd = cmd
            .arg("x")
            .arg("-aoa")
            .arg("-bd")
            .arg("-bb0")
            .arg(format!("-o{}", output_dir.display()))
            .arg(archive);

        let output = cmd
            .run_with_cancellation(ctx.cancel_token().clone())
            .await
            .context("Failed to run 7z extraction for tar.gz")?;

        if output.is_interrupted() {
            anyhow::bail!("Archive extraction was interrupted");
        }

        if output.exit_code() != 0 {
            anyhow::bail!(
                "7z extraction failed with exit code: {}",
                output.exit_code()
            );
        }

        Ok(())
    }

    async fn do_clean(&self, ctx: &ToolContext) -> Result<()> {
        let output_dir = self.output_dir_required()?;

        if ctx.is_dry_run() {
            info!(
                output = %output_dir.display(),
                "[dry-run] Would clean output directory"
            );
            return Ok(());
        }

        if output_dir.exists() {
            fs::remove_dir_all(output_dir).await.with_context(|| {
                format!("Failed to clean output directory: {}", output_dir.display())
            })?;
            info!(output = %output_dir.display(), "Output directory cleaned");
        } else {
            info!(output = %output_dir.display(), "Output directory does not exist");
        }

        Ok(())
    }
}

impl Default for ExtractorTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for ExtractorTool {
    fn name(&self) -> &'static str {
        "extractor"
    }

    fn run<'a>(&'a self, ctx: &'a ToolContext) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            match self.operation {
                ExtractorOperation::Extract => self.do_extract(ctx).await,
                ExtractorOperation::Clean => self.do_clean(ctx).await,
            }
        })
    }
}

#[cfg(test)]
mod tests;
