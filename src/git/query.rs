// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Git query operations using gix backend.
//!
//! ```text
//! query.rs --> GixBackend --> .git/ (no subprocess)
//! ```
//!
//! Uses gix for read-only operations (faster, no subprocess overhead).

use crate::error::MobResult;
use std::path::Path;

use super::backend::{GitQuery, GixBackend};

#[must_use]
pub fn is_git_repo(path: &Path) -> bool {
    GixBackend::is_git_repo(path)
}

/// Get current branch name (None if HEAD is detached).
///
/// # Errors
///
/// Returns a `GitError` if repository discovery or head resolution fails.
pub fn current_branch(path: &Path) -> MobResult<Option<String>> {
    GixBackend::current_branch(path)
}

/// Check if file is tracked by git.
///
/// # Errors
///
/// Returns a `GitError` if repository discovery or index access fails.
pub fn is_tracked(repo_path: &Path, file: &Path) -> MobResult<bool> {
    GixBackend::is_tracked(repo_path, file)
}

/// Check for uncommitted changes (staged, unstaged, or untracked files).
///
/// # Errors
///
/// Returns a `GitError` if repository discovery or status check fails.
pub fn has_uncommitted_changes(path: &Path) -> MobResult<bool> {
    GixBackend::has_uncommitted_changes(path)
}

/// Check for stashed changes.
///
/// # Errors
///
/// Returns a `GitError` if repository discovery or reference lookup fails.
pub fn has_stashed_changes(path: &Path) -> MobResult<bool> {
    GixBackend::has_stashed_changes(path)
}
