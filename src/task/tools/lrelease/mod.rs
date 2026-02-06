// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Qt lrelease tool for compiling translation files.
//!
//! ```text
//! Sources (.ts) --> lrelease --> {project}_{lang}.qm
//! Uses Qt lrelease (qt_bin or PATH)
//! ```
//!
//! This module provides the `LreleaseTool` struct for compiling Qt translation
//! files (.ts) into binary format (.qm) with cancellation support.
//!
//! # Architecture
//!
//! The lrelease tool (part of Qt) compiles `.ts` translation files into `.qm`
//! binary files that can be loaded by Qt applications at runtime.
//!
//! Multiple source files can be combined into a single output file, which is
//! useful for projects that share translations (like gamebryo plugins).

use std::path::{Path, PathBuf};

use crate::error::Result;
use anyhow::Context;
use tracing::{debug, info};

use super::{BoxFuture, Tool, ToolContext};
use crate::core::process::builder::ProcessBuilder;

/// Qt lrelease tool for compiling translation files.
///
/// # Example
///
/// ```ignore
/// // Compile a single translation file
/// let tool = LreleaseTool::new()
///     .project("modorganizer")
///     .add_source("fr.ts")
///     .output_dir("./install/bin/translations");
/// tool.run(&ctx).await?;
///
/// // Compile multiple source files into one output
/// let tool = LreleaseTool::new()
///     .project("game_skyrim")
///     .add_source("skyrim/fr.ts")
///     .add_source("gamebryo/fr.ts")
///     .output_dir("./install/bin/translations");
/// tool.run(&ctx).await?;
/// ```
#[derive(Debug, Clone)]
pub struct LreleaseTool {
    project: Option<String>,
    sources: Vec<PathBuf>,
    output_dir: Option<PathBuf>,
    lrelease_binary: Option<PathBuf>,
}

impl Default for LreleaseTool {
    fn default() -> Self {
        Self::new()
    }
}

impl LreleaseTool {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            project: None,
            sources: Vec::new(),
            output_dir: None,
            lrelease_binary: None,
        }
    }

    /// Sets the project name used to generate the output filename.
    ///
    /// The output filename will be `{project}_{lang}.qm` where `lang` is
    /// derived from the first source file's stem (e.g., "fr" from "fr.ts").
    #[must_use]
    pub fn project(mut self, name: impl Into<String>) -> Self {
        self.project = Some(name.into());
        self
    }

    #[must_use]
    pub fn add_source(mut self, path: impl AsRef<Path>) -> Self {
        self.sources.push(path.as_ref().to_path_buf());
        self
    }

    #[must_use]
    pub fn sources(mut self, paths: impl IntoIterator<Item = impl AsRef<Path>>) -> Self {
        self.sources = paths
            .into_iter()
            .map(|p| p.as_ref().to_path_buf())
            .collect();
        self
    }

    #[must_use]
    pub fn output_dir(mut self, path: impl AsRef<Path>) -> Self {
        self.output_dir = Some(path.as_ref().to_path_buf());
        self
    }

    #[must_use]
    pub fn lrelease_binary(mut self, path: impl AsRef<Path>) -> Self {
        self.lrelease_binary = Some(path.as_ref().to_path_buf());
        self
    }

    /// Gets the lrelease binary path, falling back to config or PATH.
    fn get_lrelease_binary(&self, ctx: &ToolContext) -> Result<PathBuf> {
        if let Some(ref binary) = self.lrelease_binary {
            return Ok(binary.clone());
        }

        let config_path = &ctx.config().tools.lrelease;
        if config_path.is_absolute() && config_path.exists() {
            return Ok(config_path.clone());
        }

        if let Some(ref qt_bin) = ctx.config().paths.qt_bin {
            let qt_lrelease = qt_bin.join("lrelease.exe");
            if qt_lrelease.exists() {
                return Ok(qt_lrelease);
            }
            let qt_lrelease = qt_bin.join("lrelease");
            if qt_lrelease.exists() {
                return Ok(qt_lrelease);
            }
        }

        ProcessBuilder::find("lrelease")
            .or_else(|| ProcessBuilder::find("lrelease.exe"))
            .context("lrelease executable not found in PATH or config")
    }

    /// Generates the output .qm filename based on project and first source.
    ///
    /// Returns `{project}_{lang}.qm` where lang is the stem of the first source file.
    fn qm_filename(&self) -> Result<String> {
        let project = self
            .project
            .as_ref()
            .context("LreleaseTool: project name is required")?;

        let first_source = self
            .sources
            .first()
            .context("LreleaseTool: at least one source file is required")?;

        let lang = first_source
            .file_stem()
            .and_then(|s| s.to_str())
            .context("LreleaseTool: could not determine language from source filename")?;

        Ok(format!("{project}_{lang}.qm"))
    }

    /// Returns the full path to the output .qm file.
    ///
    /// # Errors
    ///
    /// Returns an error if the output directory is not set or if the filename cannot be generated.
    pub fn qm_path(&self) -> Result<PathBuf> {
        let output_dir = self
            .output_dir
            .as_ref()
            .context("LreleaseTool: output_dir is required")?;

        Ok(output_dir.join(self.qm_filename()?))
    }
}

impl Tool for LreleaseTool {
    fn name(&self) -> &'static str {
        "lrelease"
    }

    fn run<'a>(&'a self, ctx: &'a ToolContext) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            let output_dir = self
                .output_dir
                .as_ref()
                .context("LreleaseTool: output_dir is required")?;

            if self.sources.is_empty() {
                anyhow::bail!("LreleaseTool: at least one source file is required");
            }

            let qm_filename = self.qm_filename()?;
            let qm_path = output_dir.join(&qm_filename);

            if ctx.is_dry_run() {
                info!(
                    sources = ?self.sources,
                    output = %qm_path.display(),
                    "[dry-run] Would compile translation files"
                );
                return Ok(());
            }

            if !output_dir.exists() {
                tokio::fs::create_dir_all(output_dir)
                    .await
                    .with_context(|| {
                        format!("Failed to create directory: {}", output_dir.display())
                    })?;
            }

            let lrelease_binary = self.get_lrelease_binary(ctx)?;

            let mut builder = ProcessBuilder::new(&lrelease_binary).arg("-silent");

            for source in &self.sources {
                builder = builder.arg(source);
            }

            builder = builder.arg("-qm").arg(&qm_path);

            debug!(
                sources = ?self.sources,
                output = %qm_path.display(),
                "Compiling translation files"
            );

            let output = builder
                .run_with_cancellation(ctx.cancel_token().clone())
                .await
                .context("Failed to run lrelease")?;

            if output.is_interrupted() {
                anyhow::bail!("lrelease was interrupted");
            }

            info!(
                output = %qm_path.display(),
                "Translation compiled successfully"
            );

            Ok(())
        })
    }
}

#[cfg(test)]
mod tests;
