// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

use super::{GitMutation, GitQuery, GixBackend, ShellBackend};
use tempfile::TempDir;

fn temp_dir() -> TempDir {
    tempfile::tempdir().expect("failed to create temp dir")
}

#[test]
fn test_gix_backend_is_git_repo() {
    let temp = temp_dir();
    assert!(!GixBackend::is_git_repo(temp.path()));

    gix::init(temp.path()).expect("failed to init repo");
    assert!(GixBackend::is_git_repo(temp.path()));
}

#[test]
fn test_shell_backend_is_git_repo() {
    let temp = temp_dir();
    assert!(!ShellBackend::is_git_repo(temp.path()));

    ShellBackend::init_repo(temp.path()).expect("failed to init repo");
    assert!(ShellBackend::is_git_repo(temp.path()));
}

#[test]
fn test_gix_has_stashed_changes_no_stash() {
    let temp = temp_dir();
    gix::init(temp.path()).expect("failed to init repo");

    // New repo has no stashes
    let result = GixBackend::has_stashed_changes(temp.path());
    assert!(result.is_ok());
    assert!(!result.unwrap());
}

#[test]
fn test_backends_consistency() {
    // Both backends should agree on basic queries
    let temp = temp_dir();

    // Before init: both say not a repo
    assert!(!GixBackend::is_git_repo(temp.path()));
    assert!(!ShellBackend::is_git_repo(temp.path()));

    // After init: both say it's a repo
    gix::init(temp.path()).expect("failed to init repo");
    assert!(GixBackend::is_git_repo(temp.path()));
    assert!(ShellBackend::is_git_repo(temp.path()));
}
