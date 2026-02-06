// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Git command operations using shell backend.
//!
//! ```text
//! cmd.rs --> ShellBackend --> git.exe (SSH/PuTTY, submodules)
//! ```

use crate::error::MobResult;
use std::path::Path;

use super::backend::{GitMutation, ShellBackend};

/// Execute git command with standard environment variables.
/// ALWAYS sets `GCM_INTERACTIVE=never` and `GIT_TERMINAL_PROMPT=0`.
///
/// This is exposed for internal use by ops.rs which needs raw command execution.
pub(super) fn git_command(args: &[&str], cwd: &Path) -> MobResult<String> {
    ShellBackend::git_command(args, cwd)
}

/// Clone a repository.
///
/// # Errors
///
/// Returns a `GitError` if the clone operation fails or the destination path is invalid.
pub fn clone(url: &str, dest: &Path, branch: Option<&str>, shallow: bool) -> MobResult<()> {
    ShellBackend::clone(url, dest, branch, shallow)
}

/// Pull with recurse-submodules.
///
/// # Errors
///
/// Returns a `GitError` if the pull operation fails.
pub fn pull(repo_path: &Path, remote: &str, branch: &str) -> MobResult<()> {
    ShellBackend::pull(repo_path, remote, branch)
}

/// Fetch from remote.
///
/// # Errors
///
/// Returns a `GitError` if the fetch operation fails.
pub fn fetch(repo_path: &Path, remote: &str) -> MobResult<()> {
    ShellBackend::fetch(repo_path, remote)
}

/// Checkout a branch, tag, or commit.
///
/// # Errors
///
/// Returns a `GitError` if the checkout operation fails.
pub fn checkout(repo_path: &Path, what: &str) -> MobResult<()> {
    ShellBackend::checkout(repo_path, what)
}

/// Initialize a new repository.
///
/// # Errors
///
/// Returns a `GitError` if repository initialization fails.
pub fn init_repo(path: &Path) -> MobResult<()> {
    ShellBackend::init_repo(path)
}

/// Add a submodule.
///
/// # Errors
///
/// Returns a `GitError` if the submodule cannot be added.
pub fn add_submodule(repo_path: &Path, url: &str, submodule_path: &str) -> MobResult<()> {
    ShellBackend::add_submodule(repo_path, url, submodule_path)
}

/// Add a remote, optionally with `PuTTY` key.
///
/// # Errors
///
/// Returns a `GitError` if the remote cannot be added or the `PuTTY` key path is invalid.
pub fn add_remote(
    repo_path: &Path,
    name: &str,
    url: &str,
    putty_key: Option<&Path>,
) -> MobResult<()> {
    ShellBackend::add_remote(repo_path, name, url, putty_key)
}

/// Rename a remote.
///
/// # Errors
///
/// Returns a `GitError` if the remote cannot be renamed.
pub fn rename_remote(repo_path: &Path, old_name: &str, new_name: &str) -> MobResult<()> {
    ShellBackend::rename_remote(repo_path, old_name, new_name)
}

/// Set remote push URL (e.g., to "nopushurl" to disable pushing).
///
/// # Errors
///
/// Returns a `GitError` if the push URL cannot be set.
pub fn set_remote_push_url(repo_path: &Path, remote: &str, url: &str) -> MobResult<()> {
    ShellBackend::set_remote_push_url(repo_path, remote, url)
}

/// Set git config value.
///
/// # Errors
///
/// Returns a `GitError` if the config value cannot be set.
pub fn set_config(repo_path: &Path, key: &str, value: &str) -> MobResult<()> {
    ShellBackend::set_config(repo_path, key, value)
}

/// Mark file as assume-unchanged (for .ts files).
///
/// # Errors
///
/// Returns a `GitError` if the update-index operation fails or the file path is invalid.
pub fn set_assume_unchanged(repo_path: &Path, file: &Path) -> MobResult<()> {
    ShellBackend::set_assume_unchanged(repo_path, file)
}

/// Remove assume-unchanged flag from file.
///
/// # Errors
///
/// Returns a `GitError` if the update-index operation fails or the file path is invalid.
pub fn unset_assume_unchanged(repo_path: &Path, file: &Path) -> MobResult<()> {
    ShellBackend::unset_assume_unchanged(repo_path, file)
}
