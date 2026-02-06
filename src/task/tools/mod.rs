// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tool abstractions for task execution.
//!
//! ```text
//! Task --> ToolContext --> ProcessBuilder --> Tools
//!   Git, CMake, MSBuild, ...
//! ToolContext: cancel token --> run_with_cancellation
//! ```
//!
//! All tools support graceful cancellation via `CancellationToken`.

use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::config::Config;
use crate::error::Result;

pub mod cmake;
pub mod downloader;
pub mod extractor;
pub mod git;
#[cfg(windows)]
pub mod iscc;
pub mod lrelease;
#[cfg(windows)]
pub mod msbuild;
pub mod packer;
pub mod transifex;
#[cfg(windows)]
pub mod vs;

use futures_util::future::BoxFuture;

/// Context provided to tools during execution.
///
/// Contains references to configuration, cancellation tokens, and execution flags.
#[derive(Clone)]
pub struct ToolContext {
    /// Cancellation token for cooperative cancellation.
    /// Tools should check this token periodically and abort if cancelled.
    cancel_token: CancellationToken,

    /// Whether this is a dry-run execution.
    /// When true, tools should log what they would do without making changes.
    dry_run: bool,

    /// Reference to the configuration.
    config: Arc<Config>,
}

impl ToolContext {
    /// Creates a new `ToolContext`.
    #[must_use]
    pub const fn new(config: Arc<Config>, cancel_token: CancellationToken, dry_run: bool) -> Self {
        Self {
            cancel_token,
            dry_run,
            config,
        }
    }

    /// Returns a reference to the configuration.
    #[must_use]
    pub const fn config(&self) -> &Arc<Config> {
        &self.config
    }

    /// Returns a reference to the cancellation token.
    #[must_use]
    pub const fn cancel_token(&self) -> &CancellationToken {
        &self.cancel_token
    }

    /// Returns whether this is a dry-run execution.
    #[must_use]
    pub const fn is_dry_run(&self) -> bool {
        self.dry_run
    }

    /// Checks if cancellation has been requested.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancel_token.is_cancelled()
    }
}

/// Trait for tools that execute external processes.
///
/// Tools are the building blocks of tasks. Each tool encapsulates a specific
/// external operation (git clone, cmake configure, msbuild compile, etc.).
///
/// # Implementation Notes
///
/// - Tools should use `ProcessBuilder::run_with_cancellation()` for process execution
/// - The `interrupt()` method is called when cancellation is requested
/// - Tools should respect `ctx.dry_run` and only log actions without executing
pub trait Tool: Send + Sync {
    /// Returns the name of this tool (e.g., "git", "cmake", "msbuild").
    fn name(&self) -> &str;

    /// Executes the tool's operation.
    ///
    /// # Arguments
    /// * `ctx` - The tool context with cancellation token and configuration
    ///
    /// # Returns
    /// * `Ok(())` if the operation completed successfully
    /// * `Err(...)` if the operation failed or was cancelled
    fn run<'a>(&'a self, ctx: &'a ToolContext) -> BoxFuture<'a, Result<()>>;

    /// Interrupts the tool's operation.
    ///
    /// Called when cancellation is requested. The default implementation
    /// does nothing, relying on the cancellation token being checked.
    /// Tools that spawn long-running processes may override this to
    /// send interrupt signals.
    fn interrupt(&self) {
        // Default: no-op, rely on cancellation token
    }
}

#[cfg(test)]
mod test_utils;
#[cfg(test)]
mod tests;
