// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Common task helper functions and macros.
//!
//! This module provides reusable utilities for task implementations to reduce
//! code duplication across tasks.
//!
//! # Functions
//!
//! | Function | Purpose |
//! |----------|---------|
//! | [`check_source_safe_to_delete`] | Verify git repo has no uncommitted/stashed changes |
//! | [`safe_remove_source`] | Remove directory with uncommitted changes check |
//! | [`ensure_dir`] | Create directory if it doesn't exist (dry-run aware) |
//! | [`copy_file_if_newer`] | Copy file only if source is newer than destination |
//!

use std::path::Path;

use anyhow::Context;
use tracing::info;

use crate::error::Result;
use crate::git::query::{has_stashed_changes, has_uncommitted_changes, is_git_repo};
use crate::task::TaskContext;

/// Check if a git source directory is safe to delete.
///
/// Returns `Ok(())` if safe to delete, or an error if:
/// - The directory has uncommitted changes (unless `ignore_uncommitted` is set)
/// - The directory has stashed changes (unless `ignore_uncommitted` is set)
///
/// # Arguments
///
/// * `path` - Path to the git repository
/// * `ignore_uncommitted` - If true, skip the uncommitted/stashed changes check
///
/// # Errors
///
/// Returns an error if the directory has uncommitted or stashed changes and `ignore_uncommitted` is false.
///
/// # Example
///
/// ```ignore
/// check_source_safe_to_delete(&source_path, config.global.ignore_uncommitted)?;
/// ```
pub fn check_source_safe_to_delete(path: &Path, ignore_uncommitted: bool) -> Result<()> {
    if ignore_uncommitted {
        return Ok(());
    }

    if !is_git_repo(path) {
        return Ok(());
    }

    if has_uncommitted_changes(path).unwrap_or(false) {
        anyhow::bail!(
            "Cannot delete {} - has uncommitted changes. \
             Use --ignore-uncommitted to force.",
            path.display()
        );
    }

    if has_stashed_changes(path).unwrap_or(false) {
        anyhow::bail!(
            "Cannot delete {} - has stashed changes. \
             Use --ignore-uncommitted to force.",
            path.display()
        );
    }

    Ok(())
}

/// Safely remove a source directory with uncommitted changes check.
///
/// This function:
/// 1. Checks if the path exists
/// 2. If it's a git repo, verifies no uncommitted/stashed changes (unless ignored)
/// 3. Deletes the directory (or logs dry-run message)
///
/// # Arguments
///
/// * `ctx` - Task context for dry-run and config access
/// * `path` - Path to delete
/// * `label` - Human-readable label for logging (e.g., "source directory")
///
/// # Errors
///
/// Returns an error if the directory has uncommitted changes or if the deletion fails.
///
/// # Example
///
/// ```ignore
/// safe_remove_source(ctx, &source_path, "source directory").await?;
/// ```
pub async fn safe_remove_source(ctx: &TaskContext, path: &Path, label: &str) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    check_source_safe_to_delete(path, ctx.config().global.ignore_uncommitted)?;

    if ctx.is_dry_run() {
        info!(
            path = %path.display(),
            "[DRY-RUN] would delete {}", label
        );
    } else {
        info!(path = %path.display(), "Deleting {}", label);
        tokio::fs::remove_dir_all(path)
            .await
            .with_context(|| format!("failed to delete {}", path.display()))?;
    }

    Ok(())
}

/// Ensure a directory exists, creating it if necessary.
///
/// This is dry-run aware - in dry-run mode, logs what would be created.
///
/// # Arguments
///
/// * `ctx` - Task context for dry-run check
/// * `path` - Directory path to ensure exists
/// * `label` - Human-readable label for logging
///
/// # Errors
///
/// Returns an error if the directory creation fails.
///
/// # Example
///
/// ```ignore
/// ensure_dir(ctx, &install_path, "translations directory").await?;
/// ```
pub async fn ensure_dir(ctx: &TaskContext, path: &Path, label: &str) -> Result<()> {
    if path.exists() {
        return Ok(());
    }

    if ctx.is_dry_run() {
        info!(
            path = %path.display(),
            "[DRY-RUN] would create {}", label
        );
    } else {
        tokio::fs::create_dir_all(path)
            .await
            .with_context(|| format!("failed to create {}", path.display()))?;
    }

    Ok(())
}
/// Copy a file only if source is newer than destination (or destination doesn't exist).
///
/// This is dry-run aware.
///
/// # Arguments
///
/// * `ctx` - Task context for dry-run check
/// * `src` - Source file path
/// * `dst` - Destination file path
/// * `label` - Human-readable label for logging (e.g., "Qt translation")
///
/// # Errors
///
/// Returns an error if metadata access or the copy operation fails.
///
/// # Example
///
/// ```ignore
/// copy_file_if_newer(ctx, &src_path, &dst_path, "Qt translation").await?;
/// ```
pub async fn copy_file_if_newer(
    ctx: &TaskContext,
    src: &Path,
    dst: &Path,
    label: &str,
) -> Result<()> {
    if ctx.is_dry_run() {
        info!(
            src = %src.display(),
            dst = %dst.display(),
            "[DRY-RUN] would copy {}", label
        );
        return Ok(());
    }

    // Copy if destination doesn't exist or is older
    let should_copy = if dst.exists() {
        let src_meta = tokio::fs::metadata(src)
            .await
            .with_context(|| format!("failed to get metadata for {}", src.display()))?;
        let dst_meta = tokio::fs::metadata(dst)
            .await
            .with_context(|| format!("failed to get metadata for {}", dst.display()))?;

        let src_modified = src_meta.modified().ok();
        let dst_modified = dst_meta.modified().ok();

        match (src_modified, dst_modified) {
            (Some(s), Some(d)) => s > d,
            _ => true,
        }
    } else {
        true
    };

    if should_copy {
        tracing::debug!(
            src = %src.display(),
            dst = %dst.display(),
            "Copying {}", label
        );
        tokio::fs::copy(src, dst)
            .await
            .with_context(|| format!("failed to copy {} to {}", src.display(), dst.display()))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests;
