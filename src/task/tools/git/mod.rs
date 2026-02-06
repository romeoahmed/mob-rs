// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Git tool for repository operations.
//!
//! ```text
//! GitTool
//! Operations: Clone | Pull | Fetch | Checkout | SubmoduleUpdate | Reset
//! Builder: url/path/branch/remote/target/shallow/force/recursive
//! Safety: warn on uncommitted checkout, cancellation support
//! ```
//!
//! This module provides the `GitTool` struct for executing git operations
//! like clone, pull, fetch, checkout, and submodule update with cancellation support.
//!
//! # Architecture
//!
//! This module uses shell git commands via `ProcessBuilder::run_with_cancellation()`
//! for all operations to ensure:
//! - Consistent cancellation support via `CancellationToken`
//! - `PuTTY` SSH key compatibility on Windows
//! - Real-time output streaming
//!
//! For read-only queries (like checking for uncommitted changes), use `crate::git`.

use std::path::{Path, PathBuf};

use crate::error::Result;
use anyhow::Context;
use tracing::{debug, info, warn};

use super::{BoxFuture, Tool, ToolContext};
use crate::core::process::builder::ProcessBuilder;
use crate::git::query::{has_uncommitted_changes, is_git_repo};

/// Git tool for repository operations.
///
/// Supports cloning, pulling, fetching, checking out, and submodule updates with:
/// - Shallow clones (`--depth 1`)
/// - Branch/tag/commit specification
/// - Remote specification
/// - Cancellation support
/// - Pre-operation safety checks
///
/// # Example
///
/// ```ignore
/// // Clone a repository
/// let tool = GitTool::new()
///     .url("https://github.com/example/repo.git")
///     .path("./repo")
///     .shallow(true)
///     .branch("main");
/// tool.run(&ctx).await?;
///
/// // Fetch from a remote
/// let tool = GitTool::new()
///     .path("./repo")
///     .remote("origin")
///     .fetch_op();
/// tool.run(&ctx).await?;
///
/// // Checkout a branch
/// let tool = GitTool::new()
///     .path("./repo")
///     .target("feature-branch")
///     .checkout_op();
/// tool.run(&ctx).await?;
/// ```
#[derive(Debug, Clone)]
pub struct GitTool {
    url: Option<String>,
    path: Option<PathBuf>,
    branch: Option<String>,
    remote: Option<String>,
    target: Option<String>,
    shallow: bool,
    force: bool,
    recursive: bool,
    operation: GitOperation,
}

/// Git operation to perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GitOperation {
    /// Clone a repository.
    #[default]
    Clone,
    /// Pull updates from remote.
    Pull,
    /// Fetch updates from remote without merging.
    Fetch,
    /// Checkout a branch, tag, or commit.
    Checkout,
    /// Update submodules.
    SubmoduleUpdate,
    /// Reset repository to a clean state.
    Reset,
}

impl GitTool {
    /// Creates a new `GitTool` with default settings.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            url: None,
            path: None,
            branch: None,
            remote: None,
            target: None,
            shallow: false,
            force: false,
            recursive: true,
            operation: GitOperation::Clone,
        }
    }

    #[must_use]
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    #[must_use]
    pub fn path(mut self, path: impl AsRef<Path>) -> Self {
        self.path = Some(path.as_ref().to_path_buf());
        self
    }

    #[must_use]
    pub fn branch(mut self, branch: impl Into<String>) -> Self {
        self.branch = Some(branch.into());
        self
    }

    #[must_use]
    pub fn remote(mut self, remote: impl Into<String>) -> Self {
        self.remote = Some(remote.into());
        self
    }

    #[must_use]
    pub fn target(mut self, target: impl Into<String>) -> Self {
        self.target = Some(target.into());
        self
    }

    #[must_use]
    pub const fn shallow(mut self, shallow: bool) -> Self {
        self.shallow = shallow;
        self
    }

    #[must_use]
    pub const fn force(mut self, force: bool) -> Self {
        self.force = force;
        self
    }

    #[must_use]
    pub const fn recursive(mut self, recursive: bool) -> Self {
        self.recursive = recursive;
        self
    }

    #[must_use]
    pub const fn clone_op(mut self) -> Self {
        self.operation = GitOperation::Clone;
        self
    }

    #[must_use]
    pub const fn pull_op(mut self) -> Self {
        self.operation = GitOperation::Pull;
        self
    }

    #[must_use]
    pub const fn fetch_op(mut self) -> Self {
        self.operation = GitOperation::Fetch;
        self
    }

    #[must_use]
    pub const fn checkout_op(mut self) -> Self {
        self.operation = GitOperation::Checkout;
        self
    }

    #[must_use]
    pub const fn submodule_update_op(mut self) -> Self {
        self.operation = GitOperation::SubmoduleUpdate;
        self
    }

    #[must_use]
    pub const fn reset_op(mut self) -> Self {
        self.operation = GitOperation::Reset;
        self
    }

    /// Gets the remote name, defaulting to "origin".
    fn get_remote(&self) -> &str {
        self.remote.as_deref().unwrap_or("origin")
    }

    /// Executes a git clone operation.
    async fn do_clone(&self, ctx: &ToolContext) -> Result<()> {
        let url = self
            .url
            .as_ref()
            .context("GitTool: url is required for clone")?;
        let path = self
            .path
            .as_ref()
            .context("GitTool: path is required for clone")?;

        if ctx.is_dry_run() {
            info!(
                url = %url,
                path = %path.display(),
                shallow = self.shallow,
                branch = ?self.branch,
                "[dry-run] Would clone repository"
            );
            return Ok(());
        }

        let mut builder = ProcessBuilder::which("git").context("git executable not found")?;

        builder = builder.arg("clone");

        if self.shallow {
            builder = builder.arg("--depth").arg("1");
        }

        if let Some(ref branch) = self.branch {
            builder = builder.arg("--branch").arg(branch);
        }

        builder = builder.arg(url).arg(path);

        debug!(
            url = %url,
            path = %path.display(),
            shallow = self.shallow,
            "Cloning repository"
        );

        let output = builder
            .run_with_cancellation(ctx.cancel_token().clone())
            .await
            .with_context(|| format!("Failed to clone {url}"))?;

        if output.is_interrupted() {
            anyhow::bail!("Git clone was interrupted");
        }

        info!(
            url = %url,
            path = %path.display(),
            "Repository cloned successfully"
        );

        Ok(())
    }

    /// Executes a git pull operation.
    async fn do_pull(&self, ctx: &ToolContext) -> Result<()> {
        let path = self
            .path
            .as_ref()
            .context("GitTool: path is required for pull")?;

        if ctx.is_dry_run() {
            info!(
                path = %path.display(),
                "[dry-run] Would pull repository"
            );
            return Ok(());
        }

        let mut builder = ProcessBuilder::which("git")
            .context("git executable not found")?
            .arg("pull")
            .arg("--recurse-submodules")
            .arg("--quiet");

        let remote = self.get_remote();
        builder = builder.arg(remote);
        if let Some(ref branch) = self.branch {
            builder = builder.arg(branch);
        }

        builder = builder.cwd(path);

        debug!(path = %path.display(), remote, "Pulling repository");

        let output = builder
            .run_with_cancellation(ctx.cancel_token().clone())
            .await
            .with_context(|| format!("Failed to pull in {}", path.display()))?;

        if output.is_interrupted() {
            anyhow::bail!("Git pull was interrupted");
        }

        info!(path = %path.display(), "Repository pulled successfully");

        Ok(())
    }

    /// Executes a git fetch operation.
    async fn do_fetch(&self, ctx: &ToolContext) -> Result<()> {
        let path = self
            .path
            .as_ref()
            .context("GitTool: path is required for fetch")?;

        let remote = self.get_remote();

        if ctx.is_dry_run() {
            info!(
                path = %path.display(),
                remote,
                "[dry-run] Would fetch from remote"
            );
            return Ok(());
        }

        let builder = ProcessBuilder::which("git")
            .context("git executable not found")?
            .arg("fetch")
            .arg("--quiet")
            .arg(remote)
            .cwd(path);

        debug!(path = %path.display(), remote, "Fetching from remote");

        let output = builder
            .run_with_cancellation(ctx.cancel_token().clone())
            .await
            .with_context(|| format!("Failed to fetch {} in {}", remote, path.display()))?;

        if output.is_interrupted() {
            anyhow::bail!("Git fetch was interrupted");
        }

        info!(path = %path.display(), remote, "Fetched successfully");

        Ok(())
    }

    /// Executes a git checkout operation.
    async fn do_checkout(&self, ctx: &ToolContext) -> Result<()> {
        let path = self
            .path
            .as_ref()
            .context("GitTool: path is required for checkout")?;

        let target = self
            .target
            .as_ref()
            .context("GitTool: target is required for checkout")?;

        // Check for uncommitted changes (safety check)
        if !ctx.is_dry_run() && is_git_repo(path) {
            match has_uncommitted_changes(path) {
                Ok(true) if !self.force => {
                    warn!(
                        path = %path.display(),
                        target,
                        "Repository has uncommitted changes, checkout may fail"
                    );
                }
                Err(e) => {
                    debug!(
                        path = %path.display(),
                        error = %e,
                        "Could not check for uncommitted changes"
                    );
                }
                Ok(false | true) => {}
            }
        }

        if ctx.is_dry_run() {
            info!(
                path = %path.display(),
                target,
                "[dry-run] Would checkout"
            );
            return Ok(());
        }

        let builder = ProcessBuilder::which("git")
            .context("git executable not found")?
            .arg("-c")
            .arg("advice.detachedHead=false")
            .arg("checkout")
            .arg("-q")
            .arg(target)
            .cwd(path);

        debug!(path = %path.display(), target, "Checking out");

        let output = builder
            .run_with_cancellation(ctx.cancel_token().clone())
            .await
            .with_context(|| format!("Failed to checkout {} in {}", target, path.display()))?;

        if output.is_interrupted() {
            anyhow::bail!("Git checkout was interrupted");
        }

        info!(path = %path.display(), target, "Checked out successfully");

        Ok(())
    }

    /// Executes a git submodule update operation.
    async fn do_submodule_update(&self, ctx: &ToolContext) -> Result<()> {
        let path = self
            .path
            .as_ref()
            .context("GitTool: path is required for submodule update")?;

        if ctx.is_dry_run() {
            info!(
                path = %path.display(),
                recursive = self.recursive,
                "[dry-run] Would update submodules"
            );
            return Ok(());
        }

        let mut builder = ProcessBuilder::which("git")
            .context("git executable not found")?
            .arg("submodule")
            .arg("update")
            .arg("--init");

        if self.recursive {
            builder = builder.arg("--recursive");
        }

        builder = builder.cwd(path);

        debug!(
            path = %path.display(),
            recursive = self.recursive,
            "Updating submodules"
        );

        let output = builder
            .run_with_cancellation(ctx.cancel_token().clone())
            .await
            .with_context(|| format!("Failed to update submodules in {}", path.display()))?;

        if output.is_interrupted() {
            anyhow::bail!("Git submodule update was interrupted");
        }

        info!(
            path = %path.display(),
            "Submodules updated successfully"
        );

        Ok(())
    }

    /// Executes a git reset operation.
    ///
    /// By default, performs a soft reset. Use `.force(true)` for hard reset.
    /// Hard reset will discard all uncommitted changes.
    async fn do_reset(&self, ctx: &ToolContext) -> Result<()> {
        let path = self
            .path
            .as_ref()
            .context("GitTool: path is required for reset")?;

        let mode = if self.force { "--hard" } else { "--soft" };

        // Safety warning for hard reset
        if self.force
            && !ctx.is_dry_run()
            && is_git_repo(path)
            && matches!(has_uncommitted_changes(path), Ok(true))
        {
            warn!(
                path = %path.display(),
                "Hard reset will discard uncommitted changes"
            );
        }

        if ctx.is_dry_run() {
            info!(
                path = %path.display(),
                mode,
                target = ?self.target,
                "[dry-run] Would reset repository"
            );
            return Ok(());
        }

        let mut builder = ProcessBuilder::which("git")
            .context("git executable not found")?
            .arg("reset")
            .arg(mode);

        if let Some(ref target) = self.target {
            builder = builder.arg(target);
        }

        builder = builder.cwd(path);

        debug!(path = %path.display(), mode, "Resetting repository");

        let output = builder
            .run_with_cancellation(ctx.cancel_token().clone())
            .await
            .with_context(|| format!("Failed to reset {}", path.display()))?;

        if output.is_interrupted() {
            anyhow::bail!("Git reset was interrupted");
        }

        info!(path = %path.display(), mode, "Repository reset successfully");

        Ok(())
    }
}

impl Default for GitTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for GitTool {
    fn name(&self) -> &'static str {
        "git"
    }

    fn run<'a>(&'a self, ctx: &'a ToolContext) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            match self.operation {
                GitOperation::Clone => self.do_clone(ctx).await,
                GitOperation::Pull => self.do_pull(ctx).await,
                GitOperation::Fetch => self.do_fetch(ctx).await,
                GitOperation::Checkout => self.do_checkout(ctx).await,
                GitOperation::SubmoduleUpdate => self.do_submodule_update(ctx).await,
                GitOperation::Reset => self.do_reset(ctx).await,
            }
        })
    }
}

/// Check if a remote branch exists (async version with cancellation support).
///
/// Uses `git ls-remote --heads <url> <branch>` to check if a branch exists on a remote
/// without cloning the repository. This is the async variant suitable for use within
/// async task contexts with cancellation token support.
///
/// # Arguments
///
/// * `ctx` - Tool context with cancellation token
/// * `url` - The remote repository URL
/// * `branch` - The branch name to check (without refs/heads/ prefix)
///
/// # Returns
///
/// - `Ok(true)` if the branch exists on the remote
/// - `Ok(false)` if the branch does not exist or the remote is inaccessible
/// - `Err` only for system-level errors (not network errors)
///
/// # Note
///
/// Network errors (timeouts, authentication failures, etc.) are treated as "branch doesn't exist"
/// (returns `Ok(false)`). Only system-level errors like "git not found" will return an error.
///
/// # Errors
///
/// Returns an error if the git executable cannot be found or if the operation is interrupted.
pub async fn remote_branch_exists_ctx(ctx: &ToolContext, url: &str, branch: &str) -> Result<bool> {
    use std::time::Duration;

    let mut builder = ProcessBuilder::which("git")
        .context("git executable not found")?
        .arg("ls-remote")
        .arg("--heads")
        .arg(url)
        .arg(format!("refs/heads/{branch}"));

    builder = builder.timeout(Duration::from_secs(10));

    debug!(
        url = %url,
        branch,
        "Checking if remote branch exists"
    );

    let output = builder
        .run_with_cancellation(ctx.cancel_token().clone())
        .await
        .with_context(|| format!("Failed to check remote branch {branch} at {url}"))?;

    if output.is_interrupted() {
        anyhow::bail!("Remote branch check was interrupted");
    }

    let branch_exists = !output.stdout().trim().is_empty();

    debug!(
        url = %url,
        branch,
        exists = branch_exists,
        "Remote branch check completed"
    );

    Ok(branch_exists)
}

#[cfg(test)]
mod tests;
