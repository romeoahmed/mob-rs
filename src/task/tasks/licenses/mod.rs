// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Licenses task implementation.
//!
//! ```text
//! LicensesTask
//! paths.licenses/ → install/bin/licenses/
//! Phases: Clean (no-op) → Fetch (no-op) → BuildAndInstall (copy)
//! ```
//!
//! This task copies license files from the licenses directory to the
//! installation directory, making them available in the final MO2 distribution.
//!
//! # Process
//!
//! 1. **`BuildAndInstall`**: Copy all files from licenses/ to install/bin/licenses/
//!
//! This task has no clean or fetch phases - it only copies files during install.

use std::path::PathBuf;

use crate::error::Result;
use anyhow::Context;
use futures_util::future::BoxFuture;
use tokio::fs;
use tracing::info;

use crate::config::Config;
use crate::task::{CleanFlags, TaskContext, Taskable};
use crate::utility::fs::copy::copy_dir_contents_async;

/// Licenses task for copying license files.
#[derive(Debug, Clone)]
pub struct LicensesTask {
    /// Task name
    name: String,
}

impl Default for LicensesTask {
    fn default() -> Self {
        Self::new()
    }
}

impl LicensesTask {
    /// Create a new licenses task.
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: "licenses".to_string(),
        }
    }

    /// Returns the task name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the source licenses directory.
    fn source_path(config: &Config) -> Result<PathBuf> {
        config
            .paths
            .licenses
            .clone()
            .context("paths.licenses not configured")
    }

    /// Get the install path for licenses.
    fn install_path(config: &Config) -> Result<PathBuf> {
        config
            .paths
            .install_licenses
            .clone()
            .context("paths.install_licenses not configured")
    }

    /// Execute the clean phase (no-op for licenses).
    ///
    /// # Errors
    ///
    /// This function is currently infallible and always returns `Ok(())`.
    pub fn do_clean(
        &self,
        _ctx: &TaskContext,
        _flags: CleanFlags,
    ) -> impl std::future::Future<Output = Result<()>> {
        // Licenses task has no clean phase - nothing to clean
        std::future::ready(Ok(()))
    }

    /// Execute the fetch phase (no-op for licenses).
    ///
    /// # Errors
    ///
    /// This function is currently infallible and always returns `Ok(())`.
    pub fn do_fetch(&self, _ctx: &TaskContext) -> impl std::future::Future<Output = Result<()>> {
        // Licenses task has no fetch phase - no downloads
        std::future::ready(Ok(()))
    }

    /// Execute the build and install phase.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The install licenses directory is not configured.
    /// - The install directory cannot be created.
    /// - License files cannot be copied.
    pub async fn do_build_and_install(&self, ctx: &TaskContext) -> Result<()> {
        let config = &ctx.config;

        let Ok(source_path) = Self::source_path(config) else {
            info!("paths.licenses not configured, skipping licenses task");
            return Ok(());
        };

        let install_path = Self::install_path(config)?;

        if !source_path.exists() {
            info!(
                path = %source_path.display(),
                "Licenses source directory not found, skipping"
            );
            return Ok(());
        }

        // Create install directory if needed
        if !install_path.exists() {
            if ctx.dry_run {
                info!(
                    path = %install_path.display(),
                    "[DRY-RUN] would create licenses directory"
                );
            } else {
                fs::create_dir_all(&install_path)
                    .await
                    .with_context(|| format!("failed to create {}", install_path.display()))?;
            }
        }

        info!("Copying license files");

        // Copy all files and directories from source to install
        if ctx.dry_run {
            info!(
                src = %source_path.display(),
                dst = %install_path.display(),
                "[DRY-RUN] would copy license files"
            );
        } else {
            copy_dir_contents_async(&source_path, &install_path).await?;
        }

        Ok(())
    }
}

impl Taskable for LicensesTask {
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
