// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Integration tests for Git operations.
//!
//! Tests the git module with real temporary repositories.

use mob_rs::git::cmd::{
    add_remote, checkout, init_repo, rename_remote, set_config, set_remote_push_url,
};
use mob_rs::git::query::{
    current_branch, has_stashed_changes, has_uncommitted_changes, is_git_repo, is_tracked,
};
use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn temp_dir() -> TempDir {
    tempfile::tempdir().expect("failed to create temp dir")
}

/// Parse `git remote -v` output into a structured, deterministic representation.
/// Each line is parsed as (name, url, type) where type is "fetch" or "push".
/// This ensures snapshot stability across different environments.
fn parse_remotes(output: &str) -> Vec<(String, String, String)> {
    let mut remotes: Vec<(String, String, String)> = output
        .lines()
        .filter(|line| !line.is_empty())
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let name = parts[0].to_string();
                let url = parts[1].to_string();
                let remote_type = if line.contains("(fetch)") {
                    "fetch"
                } else if line.contains("(push)") {
                    "push"
                } else {
                    "unknown"
                }
                .to_string();
                Some((name, url, remote_type))
            } else {
                None
            }
        })
        .collect();
    // Sort for determinism
    remotes.sort();
    remotes
}

/// Helper to run git commands in a directory
fn run_git(args: &[&str], cwd: &std::path::Path) -> bool {
    Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@test.com")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@test.com")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Create an initialized git repo in the temp directory
fn init_test_repo(dir: &std::path::Path) {
    run_git(&["init", "-q"], dir);
    run_git(&["config", "user.email", "test@test.com"], dir);
    run_git(&["config", "user.name", "Test"], dir);
}

/// Create an initialized git repo with an initial commit (README.md)
fn init_test_repo_with_commit(dir: &std::path::Path) {
    init_test_repo(dir);
    let file = dir.join("README.md");
    fs::write(&file, "# Test").unwrap();
    run_git(&["add", "."], dir);
    run_git(&["commit", "-m", "Initial commit"], dir);
}

// =============================================================================
// is_git_repo
// =============================================================================

#[test]
fn git_is_git_repo_true() {
    let temp = temp_dir();
    init_test_repo(temp.path());
    assert!(is_git_repo(temp.path()));
}

#[test]
fn git_is_git_repo_false() {
    let temp = temp_dir();
    assert!(!is_git_repo(temp.path()));
}

#[test]
fn git_is_git_repo_subdirectory() {
    let temp = temp_dir();
    init_test_repo(temp.path());

    // Create a subdirectory
    let subdir = temp.path().join("subdir");
    fs::create_dir(&subdir).unwrap();

    // Subdirectory should still be recognized as inside a git repo
    assert!(is_git_repo(&subdir));
}

// =============================================================================
// current_branch
// =============================================================================

#[test]
fn git_current_branch_master() {
    let temp = temp_dir();
    init_test_repo_with_commit(temp.path());

    let branch = current_branch(temp.path()).unwrap();
    // Could be "master" or "main" depending on git config
    assert!(
        branch == Some("master".to_string()) || branch == Some("main".to_string()),
        "Expected master or main, got {branch:?}"
    );
}

#[test]
fn git_current_branch_custom() {
    let temp = temp_dir();
    init_test_repo_with_commit(temp.path());

    // Create and checkout new branch
    run_git(&["checkout", "-b", "feature-branch"], temp.path());

    let branch = current_branch(temp.path()).unwrap();
    insta::assert_yaml_snapshot!("git_current_branch_custom", branch);
}

// =============================================================================
// has_uncommitted_changes
// =============================================================================

#[test]
fn git_no_uncommitted_changes_clean() {
    let temp = temp_dir();
    init_test_repo_with_commit(temp.path());

    assert!(!has_uncommitted_changes(temp.path()).unwrap());
}

#[test]
fn git_has_uncommitted_changes_modified() {
    let temp = temp_dir();
    init_test_repo_with_commit(temp.path());

    // Modify the file
    let file = temp.path().join("README.md");
    fs::write(&file, "# Modified").unwrap();

    assert!(has_uncommitted_changes(temp.path()).unwrap());
}

#[test]
fn git_has_uncommitted_changes_staged() {
    let temp = temp_dir();
    init_test_repo_with_commit(temp.path());

    // Stage a new file
    let new_file = temp.path().join("new.txt");
    fs::write(&new_file, "new content").unwrap();
    run_git(&["add", "new.txt"], temp.path());

    assert!(has_uncommitted_changes(temp.path()).unwrap());
}

#[test]
fn git_has_uncommitted_changes_untracked() {
    let temp = temp_dir();
    init_test_repo_with_commit(temp.path());

    // Add untracked file
    let untracked = temp.path().join("untracked.txt");
    fs::write(&untracked, "untracked").unwrap();

    assert!(has_uncommitted_changes(temp.path()).unwrap());
}

// =============================================================================
// is_tracked
// =============================================================================

#[test]
fn git_is_tracked_true() {
    let temp = temp_dir();
    init_test_repo(temp.path());

    // Create and commit a file
    let file = temp.path().join("tracked.txt");
    fs::write(&file, "tracked content").unwrap();
    run_git(&["add", "tracked.txt"], temp.path());
    run_git(&["commit", "-m", "Add tracked file"], temp.path());

    assert!(is_tracked(temp.path(), &file).unwrap());
}

#[test]
fn git_is_tracked_false() {
    let temp = temp_dir();
    init_test_repo(temp.path());

    // Create initial commit with one file
    let file = temp.path().join("tracked.txt");
    fs::write(&file, "tracked content").unwrap();
    run_git(&["add", "."], temp.path());
    run_git(&["commit", "-m", "Initial commit"], temp.path());

    // Create untracked file
    let untracked = temp.path().join("untracked.txt");
    fs::write(&untracked, "untracked").unwrap();

    assert!(!is_tracked(temp.path(), &untracked).unwrap());
}

// =============================================================================
// has_stashed_changes
// =============================================================================

#[test]
fn git_no_stashed_changes() {
    let temp = temp_dir();
    init_test_repo_with_commit(temp.path());

    assert!(!has_stashed_changes(temp.path()).unwrap());
}

#[test]
fn git_has_stashed_changes() {
    let temp = temp_dir();
    init_test_repo_with_commit(temp.path());

    // Modify and stash
    let file = temp.path().join("README.md");
    fs::write(&file, "# Modified").unwrap();
    run_git(&["stash"], temp.path());

    assert!(has_stashed_changes(temp.path()).unwrap());
}

// =============================================================================
// init_repo
// =============================================================================

#[test]
fn git_init_repo() {
    let temp = temp_dir();

    // Directory should not be a git repo initially
    assert!(!is_git_repo(temp.path()));

    // Initialize via our function
    init_repo(temp.path()).unwrap();

    // Now it should be a git repo
    assert!(is_git_repo(temp.path()));
}

// =============================================================================
// checkout
// =============================================================================

#[test]
fn git_checkout_branch() {
    let temp = temp_dir();
    init_test_repo_with_commit(temp.path());

    // Create a new branch
    run_git(&["branch", "feature"], temp.path());

    // Checkout using our function
    checkout(temp.path(), "feature").unwrap();

    let branch = current_branch(temp.path()).unwrap();
    insta::assert_yaml_snapshot!("git_checkout_branch", branch);
}

// =============================================================================
// set_config
// =============================================================================

#[test]
fn git_set_config() {
    let temp = temp_dir();
    init_test_repo(temp.path());

    // Set a config value
    set_config(temp.path(), "user.name", "TestUser").unwrap();

    // Verify it was set
    let output = Command::new("git")
        .args(["config", "user.name"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
    insta::assert_yaml_snapshot!("git_set_config", name);
}

// =============================================================================
// add_remote / rename_remote
// =============================================================================

#[test]
fn git_add_and_rename_remote() {
    let temp = temp_dir();
    init_test_repo(temp.path());

    // Add a remote
    add_remote(
        temp.path(),
        "origin",
        "https://github.com/test/repo.git",
        None,
    )
    .unwrap();

    // Verify remote exists - snapshot after adding
    let output = Command::new("git")
        .args(["remote", "-v"])
        .current_dir(temp.path())
        .output()
        .unwrap();
    let remotes_after_add = parse_remotes(&String::from_utf8_lossy(&output.stdout));
    insta::assert_yaml_snapshot!("git_add_remote", remotes_after_add);

    // Rename remote
    rename_remote(temp.path(), "origin", "upstream").unwrap();

    // Verify rename - snapshot after renaming
    let output = Command::new("git")
        .args(["remote", "-v"])
        .current_dir(temp.path())
        .output()
        .unwrap();
    let remotes_after_rename = parse_remotes(&String::from_utf8_lossy(&output.stdout));
    insta::assert_yaml_snapshot!("git_rename_remote", remotes_after_rename);
}

// =============================================================================
// set_remote_push_url
// =============================================================================

#[test]
fn git_set_remote_push_url() {
    let temp = temp_dir();
    init_test_repo(temp.path());

    // Add a remote
    add_remote(
        temp.path(),
        "origin",
        "https://github.com/test/repo.git",
        None,
    )
    .unwrap();

    // Set push URL to disable pushing
    set_remote_push_url(temp.path(), "origin", "nopushurl").unwrap();

    // Verify push URL was changed - snapshot the result
    let output = Command::new("git")
        .args(["remote", "-v"])
        .current_dir(temp.path())
        .output()
        .unwrap();
    let remotes = parse_remotes(&String::from_utf8_lossy(&output.stdout));
    insta::assert_yaml_snapshot!("git_set_remote_push_url", remotes);
}

// =============================================================================
// Error Cases
// =============================================================================

#[test]
fn git_current_branch_not_a_repo() {
    let temp = temp_dir();
    // Not initialized as git repo
    let result = current_branch(temp.path());
    assert!(result.is_err());
}

#[test]
fn git_checkout_nonexistent_branch() {
    let temp = temp_dir();
    init_test_repo_with_commit(temp.path());

    // Try to checkout non-existent branch
    let result = checkout(temp.path(), "nonexistent-branch-xyz");
    assert!(result.is_err());
}
