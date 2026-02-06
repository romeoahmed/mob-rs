// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Installer task implementation.
//!
//! This task builds the MO2 installer using Inno Setup.
//!
//! # Process
//!
//! 1. **Fetch**: Clone the modorganizer-Installer repository
//! 2. **Build**: Compile the installer using ISCC (Inno Setup Compiler)
//!
//! # Directory Structure
//!
//! ```text
//! build/
//!   modorganizer_super/
//!     installer/        # Cloned from modorganizer-Installer
//!       dist/
//!         MO2-Installer.iss
//! install/
//!   installer/          # Output directory for the compiled installer
//! ```

use std::path::PathBuf;

use crate::error::Result;
use anyhow::Context;
use futures_util::future::BoxFuture;
use tracing::{debug, info, warn};

use crate::config::Config;
use crate::task::tools::git::{GitTool, remote_branch_exists_ctx};
use crate::task::tools::{Tool, ToolContext};
use crate::task::{CleanFlags, TaskContext, Taskable};

#[cfg(windows)]
use crate::task::tools::iscc::IsccTool;

/// Installer task for building the MO2 installer.
#[derive(Debug, Clone)]
pub struct InstallerTask {
    /// Task name
    name: String,
}

impl Default for InstallerTask {
    fn default() -> Self {
        Self::new()
    }
}

impl InstallerTask {
    /// Create a new installer task.
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: "installer".to_string(),
        }
    }

    /// Returns the task name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the source path for the installer repository.
    ///
    /// This is `build/modorganizer_super/installer`.
    fn source_path(config: &Config) -> Result<PathBuf> {
        let build = config
            .paths
            .build
            .as_ref()
            .context("paths.build not configured")?;
        Ok(build.join("modorganizer_super").join("installer"))
    }

    /// Get the install path for the compiled installer.
    fn install_path(config: &Config) -> Result<PathBuf> {
        config
            .paths
            .install_installer
            .clone()
            .context("paths.install_installer not configured")
    }

    /// Build the git URL for the installer repository.
    fn git_url(config: &Config) -> String {
        format!(
            "{}{}modorganizer-Installer.git",
            config.task.git_url_prefix, config.task.mo_org
        )
    }

    /// Select the first existing branch from a list of candidates.
    ///
    /// Checks each branch candidate in order and returns the first one that exists
    /// on the remote. If no branches exist or cannot be checked, falls back to the
    /// first candidate (primary branch).
    ///
    /// # Arguments
    ///
    /// * `tool_ctx` - Tool context with cancellation token
    /// * `git_url` - The remote repository URL
    /// * `candidates` - List of branch names to try, in order of preference
    ///
    /// # Returns
    ///
    /// The first existing branch, or the primary branch if none can be verified.
    async fn select_branch(
        &self,
        tool_ctx: &ToolContext,
        git_url: &str,
        candidates: &[String],
    ) -> Result<String> {
        if candidates.is_empty() {
            return Err(anyhow::anyhow!("No branch candidates provided"));
        }

        debug!(
            candidates = ?candidates,
            "Selecting branch from candidates"
        );

        // Try each candidate
        for (index, candidate) in candidates.iter().enumerate() {
            match remote_branch_exists_ctx(tool_ctx, git_url, candidate).await {
                Ok(true) => {
                    debug!(
                        branch = %candidate,
                        index,
                        "Remote branch exists"
                    );
                    return Ok(candidate.clone());
                }
                Ok(false) => {
                    debug!(
                        branch = %candidate,
                        index,
                        "Remote branch does not exist"
                    );
                    // Continue to next candidate
                }
                Err(e) => {
                    debug!(
                        branch = %candidate,
                        error = %e,
                        "Could not verify remote branch, continuing to next candidate"
                    );
                    // Continue to next candidate on error
                }
            }
        }

        // If no branch could be verified, use the primary (first) branch
        // This ensures we always attempt to clone with a valid branch name
        let selected = candidates[0].clone();
        info!(
            selected = %selected,
            candidates = ?candidates,
            "Using primary branch (could not verify remote branches)"
        );
        Ok(selected)
    }

    /// Execute the clean phase.
    ///
    /// # Errors
    ///
    /// Returns an error if the installer source directory or the output directory
    /// cannot be removed.
    pub async fn do_clean(&self, ctx: &TaskContext, flags: CleanFlags) -> Result<()> {
        let config = &ctx.config;

        // Reextract/Reclone: delete the source directory
        if flags.contains(CleanFlags::REEXTRACT) {
            let source = Self::source_path(config)?;
            if source.exists() {
                if ctx.dry_run {
                    info!(
                        path = %source.display(),
                        "[DRY-RUN] would delete installer source directory"
                    );
                } else {
                    info!(path = %source.display(), "Deleting installer source directory");
                    tokio::fs::remove_dir_all(&source)
                        .await
                        .with_context(|| format!("failed to delete {}", source.display()))?;
                }
            }
        }

        // Rebuild: delete the installer output directory
        if flags.contains(CleanFlags::REBUILD) {
            let install = Self::install_path(config)?;
            if install.exists() {
                if ctx.dry_run {
                    info!(
                        path = %install.display(),
                        "[DRY-RUN] would delete installer output directory"
                    );
                } else {
                    info!(path = %install.display(), "Deleting installer output directory");
                    tokio::fs::remove_dir_all(&install)
                        .await
                        .with_context(|| format!("failed to delete {}", install.display()))?;
                }
            }
        }

        Ok(())
    }

    /// Execute the fetch phase.
    ///
    /// # Errors
    ///
    /// Returns an error if the repository cannot be cloned or pulled, or if
    /// the branch selection fails.
    pub async fn do_fetch(&self, ctx: &TaskContext) -> Result<()> {
        let config = &ctx.config;
        let task_config = config.task_config(&self.name);
        let tool_ctx = ctx.tool_context();

        let source_path = Self::source_path(config)?;
        let git_url = Self::git_url(config);

        // Build candidate branch list: [mo_branch] + mo_fallback (if non-empty)
        let mut branch_candidates = vec![task_config.mo_branch.clone()];
        if !task_config.mo_fallback.is_empty() {
            branch_candidates.push(task_config.mo_fallback.clone());
        }

        // Select the first existing branch
        let branch = self
            .select_branch(&tool_ctx, &git_url, &branch_candidates)
            .await?;

        if source_path.exists() {
            // Pull existing repo
            if task_config.git_behavior.no_pull {
                debug!(path = %source_path.display(), "Skipping pull (no_pull=true)");
                return Ok(());
            }

            info!(
                repo = "modorganizer-Installer",
                branch = %branch,
                "Pulling updates"
            );

            let git = GitTool::new().path(&source_path).branch(&branch).pull_op();

            git.run(&tool_ctx)
                .await
                .context("failed to pull modorganizer-Installer")?;
        } else {
            // Clone new repo
            info!(
                repo = "modorganizer-Installer",
                url = %git_url,
                branch = %branch,
                "Cloning repository"
            );

            // Create parent directory if needed
            if let Some(parent) = source_path.parent()
                && !parent.exists()
            {
                if ctx.dry_run {
                    info!(
                        path = %parent.display(),
                        "[DRY-RUN] would create parent directory"
                    );
                } else {
                    tokio::fs::create_dir_all(parent)
                        .await
                        .with_context(|| format!("failed to create {}", parent.display()))?;
                }
            }

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
                .context("failed to clone modorganizer-Installer")?;
        }

        Ok(())
    }

    /// Execute the build and install phase.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The installer output directory cannot be created.
    /// - The Inno Setup compiler (ISCC) fails to build the installer.
    #[cfg(windows)]
    pub async fn do_build_and_install(&self, ctx: &TaskContext) -> Result<()> {
        let config = &ctx.config;
        let tool_ctx = ctx.tool_context();

        let source_path = Self::source_path(config)?;
        let install_path = Self::install_path(config)?;

        // The ISS file is at source_path/dist/MO2-Installer.iss
        let iss_file = source_path.join("dist").join("MO2-Installer.iss");

        if !iss_file.exists() {
            warn!(
                path = %iss_file.display(),
                "Installer script not found. Run fetch first."
            );
            return Ok(());
        }

        // Create install directory if needed
        if !install_path.exists() {
            if ctx.dry_run {
                info!(
                    path = %install_path.display(),
                    "[DRY-RUN] would create installer output directory"
                );
            } else {
                tokio::fs::create_dir_all(&install_path)
                    .await
                    .with_context(|| format!("failed to create {}", install_path.display()))?;
            }
        }

        info!(
            iss = %iss_file.display(),
            output = %install_path.display(),
            "Building installer"
        );

        let iscc = IsccTool::new().iss(&iss_file).output_dir(&install_path);

        iscc.run(&tool_ctx)
            .await
            .context("failed to build installer")?;

        info!("Installer built successfully");

        Ok(())
    }

    /// Execute the build and install phase (non-Windows stub).
    #[cfg(not(windows))]
    pub async fn do_build_and_install(&self, _ctx: &TaskContext) -> Result<()> {
        warn!("Installer task is only available on Windows");
        Ok(())
    }
}

impl Taskable for InstallerTask {
    fn name(&self) -> &str {
        &self.name
    }

    fn enabled(&self, _ctx: &TaskContext) -> bool {
        // Installer is only available on Windows
        cfg!(windows)
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
