// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Inno Setup Compiler (ISCC) tool for creating installers.
//!
//! ```text
//! script.iss --> ISCC.exe --> Setup.exe
//! e.g. iscc /DVERSION=2.5.0 /DARCH=x64 /Odist /FMO2-Setup
//! ```
//!
//! This module provides the `IsccTool` struct for compiling Inno Setup
//! scripts (.iss) into Windows installers with cancellation support.
//!
//! # Architecture
//!
//! The Inno Setup Compiler (`iscc.exe`) compiles `.iss` script files into
//! executable Windows installers. This is used to create the final MO2 installer.

use std::path::{Path, PathBuf};

use crate::error::Result;
use anyhow::Context;
use tracing::{debug, info};

use super::{BoxFuture, Tool, ToolContext};
use crate::core::process::builder::ProcessBuilder;

/// Inno Setup Compiler tool for creating installers.
///
/// Compiles Inno Setup scripts (.iss) into Windows installers.
///
/// # Example
///
/// ```ignore
/// let tool = IsccTool::new()
///     .iss("./installer/MO2-Installer.iss");
/// tool.run(&ctx).await?;
/// ```
#[derive(Debug, Clone)]
pub struct IsccTool {
    /// Path to the .iss script file.
    iss: Option<PathBuf>,

    /// Path to the iscc executable.
    iscc_binary: Option<PathBuf>,

    /// Additional defines to pass to iscc (/D).
    defines: Vec<(String, String)>,

    /// Output directory override (/O).
    output_dir: Option<PathBuf>,

    /// Output filename override (/F).
    output_name: Option<String>,
}

impl Default for IsccTool {
    fn default() -> Self {
        Self::new()
    }
}

impl IsccTool {
    /// Creates a new `IsccTool` with default settings.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            iss: None,
            iscc_binary: None,
            defines: Vec::new(),
            output_dir: None,
            output_name: None,
        }
    }

    #[must_use]
    pub fn iss(mut self, path: impl AsRef<Path>) -> Self {
        self.iss = Some(path.as_ref().to_path_buf());
        self
    }

    #[must_use]
    pub fn iscc_binary(mut self, path: impl AsRef<Path>) -> Self {
        self.iscc_binary = Some(path.as_ref().to_path_buf());
        self
    }

    #[must_use]
    pub fn define(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.defines.push((name.into(), value.into()));
        self
    }

    #[must_use]
    pub fn output_dir(mut self, path: impl AsRef<Path>) -> Self {
        self.output_dir = Some(path.as_ref().to_path_buf());
        self
    }

    #[must_use]
    pub fn output_name(mut self, name: impl Into<String>) -> Self {
        self.output_name = Some(name.into());
        self
    }

    /// Gets the iscc binary path, falling back to config or PATH.
    fn get_iscc_binary(&self, ctx: &ToolContext) -> Result<PathBuf> {
        if let Some(ref binary) = self.iscc_binary {
            return Ok(binary.clone());
        }

        // Try config path first
        let config_path = &ctx.config().tools.iscc;
        if config_path.is_absolute() && config_path.exists() {
            return Ok(config_path.clone());
        }

        let common_paths = [
            r"C:\Program Files (x86)\Inno Setup 6\ISCC.exe",
            r"C:\Program Files\Inno Setup 6\ISCC.exe",
            r"C:\Program Files (x86)\Inno Setup 5\ISCC.exe",
            r"C:\Program Files\Inno Setup 5\ISCC.exe",
        ];

        for path in &common_paths {
            let p = PathBuf::from(path);
            if p.exists() {
                return Ok(p);
            }
        }

        ProcessBuilder::find("iscc")
            .or_else(|| ProcessBuilder::find("ISCC.exe"))
            .context("iscc executable not found in PATH, config, or common locations")
    }
}

impl Tool for IsccTool {
    fn name(&self) -> &'static str {
        "iscc"
    }

    fn run<'a>(&'a self, ctx: &'a ToolContext) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            let iss = self
                .iss
                .as_ref()
                .context("IsccTool: iss script path is required")?;

            if ctx.is_dry_run() {
                info!(
                    iss = %iss.display(),
                    output_dir = ?self.output_dir,
                    output_name = ?self.output_name,
                    "[dry-run] Would compile Inno Setup script"
                );
                return Ok(());
            }

            let iscc_binary = self.get_iscc_binary(ctx)?;

            let mut builder = ProcessBuilder::new(&iscc_binary);

            for (name, value) in &self.defines {
                builder = builder.arg(format!("/D{name}={value}"));
            }

            if let Some(ref output_dir) = self.output_dir {
                builder = builder.arg(format!("/O{}", output_dir.display()));
            }

            if let Some(ref output_name) = self.output_name {
                builder = builder.arg(format!("/F{output_name}"));
            }

            builder = builder.arg(iss);

            debug!(
                iss = %iss.display(),
                "Compiling Inno Setup script"
            );

            let output = builder
                .run_with_cancellation(ctx.cancel_token().clone())
                .await
                .with_context(|| format!("Failed to compile {}", iss.display()))?;

            if output.is_interrupted() {
                anyhow::bail!("iscc was interrupted");
            }

            info!(
                iss = %iss.display(),
                "Installer compiled successfully"
            );

            Ok(())
        })
    }
}

#[cfg(test)]
mod tests;
