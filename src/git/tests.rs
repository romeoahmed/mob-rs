// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::config::Config;
use crate::config::paths::PathsConfig;
use crate::git::discovery::get_repos;
use crate::git::ops::{
    add_remote_to_repos, fetch_refspec, list_branches, remote_branch_exists, set_ignore_ts,
    set_remotes_for_all,
};
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn temp_dir() -> TempDir {
    tempfile::tempdir().expect("failed to create temp dir")
}

/// Initialize a git repository without commit (for tests that only need repo existence)
fn init_test_repo(path: &Path) -> Result<(), Box<gix::init::Error>> {
    gix::init(path).map_err(Box::new)?;
    Ok(())
}

/// Initialize a git repository with an initial commit (for tests needing branches)
/// Uses shell git for simplicity and to avoid coupling tests to gix internals.
/// Returns the name of the default branch (master or main depending on git config).
fn init_test_repo_with_commit(path: &Path) -> std::io::Result<String> {
    // git init
    let output = Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(path)
        .output()?;
    if !output.status.success() {
        return Err(std::io::Error::other(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    // git config (needed for commit)
    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(path)
        .output()?;
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(path)
        .output()?;

    // git commit --allow-empty (creates initial commit without files)
    let output = Command::new("git")
        .args(["commit", "--allow-empty", "-m", "Initial commit", "--quiet"])
        .current_dir(path)
        .output()?;
    if !output.status.success() {
        return Err(std::io::Error::other(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    // Get the current branch name (could be master or main)
    let output = Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(path)
        .output()?;
    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();

    Ok(branch)
}

#[test]
fn test_get_repos_finds_repositories() {
    // Create temporary directory structure
    let temp = temp_dir();
    let build = temp.path();

    // Create usvfs and initialize as a git repo
    let usvfs = build.join("usvfs");
    std::fs::create_dir_all(&usvfs).expect("failed to create usvfs");
    init_test_repo(&usvfs).expect("failed to init usvfs repo");

    // Create modorganizer_super with some repo directories
    let super_path = build.join("modorganizer_super");
    std::fs::create_dir_all(&super_path).expect("failed to create modorganizer_super");
    let modorganizer_path = super_path.join("modorganizer");
    std::fs::create_dir_all(&modorganizer_path).expect("failed to create modorganizer");
    init_test_repo(&modorganizer_path).expect("failed to init modorganizer repo");
    let uibase_path = super_path.join("uibase");
    std::fs::create_dir_all(&uibase_path).expect("failed to create uibase");
    init_test_repo(&uibase_path).expect("failed to init uibase repo");
    std::fs::create_dir_all(super_path.join(".hidden")).expect("failed to create .hidden");

    // Build minimal config
    let config = Config {
        paths: PathsConfig {
            build: Some(build.to_path_buf()),
            ..Default::default()
        },
        ..Default::default()
    };

    let repos = get_repos(&config).expect("get_repos should succeed");

    // Extract just the repo names for snapshot (strips temp path prefix)
    let mut repo_names: Vec<_> = repos
        .iter()
        .filter_map(|p| p.file_name())
        .filter_map(|n| n.to_str())
        .collect();
    repo_names.sort_unstable();
    insta::assert_debug_snapshot!(repo_names);
}

#[test]
fn test_get_repos_errors_without_build_path() {
    let config = Config::default();
    let result = get_repos(&config);
    assert!(
        result.is_err(),
        "should error when paths.build is not configured"
    );
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("paths.build"),
        "error message should mention paths.build, got: {err_msg}"
    );
}

#[test]
fn test_get_repos_handles_missing_directories() {
    // Create temp dir but no subdirectories
    let temp = temp_dir();
    let build = temp.path();

    let config = Config {
        paths: PathsConfig {
            build: Some(build.to_path_buf()),
            ..Default::default()
        },
        ..Default::default()
    };

    let repos = get_repos(&config).expect("get_repos should succeed with missing dirs");
    // Should return empty vector (no usvfs, no modorganizer_super)
    assert_eq!(
        repos.len(),
        0,
        "expected empty repos when directories don't exist"
    );
}

#[test]
fn test_get_repos_skips_non_directories() {
    let temp = temp_dir();
    let build = temp.path();

    // Create modorganizer_super with a file (not a directory)
    let super_path = build.join("modorganizer_super");
    std::fs::create_dir_all(&super_path).expect("failed to create modorganizer_super");
    std::fs::write(super_path.join("readme.txt"), "test").expect("failed to create file");

    let config = Config {
        paths: PathsConfig {
            build: Some(build.to_path_buf()),
            ..Default::default()
        },
        ..Default::default()
    };

    let repos = get_repos(&config).expect("get_repos should succeed");
    // Should return empty vector (file is not a directory)
    assert_eq!(repos.len(), 0, "expected empty repos when only files exist");
}

#[test]
fn test_list_branches_returns_all_repos() {
    let temp = temp_dir();
    let build = temp.path();

    // Create modorganizer_super with test repos
    let super_path = build.join("modorganizer_super");
    std::fs::create_dir_all(&super_path).expect("failed to create modorganizer_super");

    // Create two repos with initial commits so they have branches
    let repo1 = super_path.join("repo1");
    let repo2 = super_path.join("repo2");
    std::fs::create_dir_all(&repo1).expect("failed to create repo1");
    std::fs::create_dir_all(&repo2).expect("failed to create repo2");

    let _ = init_test_repo_with_commit(&repo1).expect("failed to init repo1 with commit");
    let _ = init_test_repo_with_commit(&repo2).expect("failed to init repo2 with commit");

    let config = Config {
        paths: PathsConfig {
            build: Some(build.to_path_buf()),
            ..Default::default()
        },
        ..Default::default()
    };

    let branches = list_branches(&config).expect("list_branches should succeed");

    // Extract repo names and normalize branch (master/main -> "default")
    let mut branch_info: Vec<_> = branches
        .iter()
        .filter_map(|(p, branch)| {
            let name = p.file_name()?.to_str()?;
            let normalized_branch = if branch == "master" || branch == "main" {
                "default"
            } else {
                branch.as_str()
            };
            Some((name, normalized_branch))
        })
        .collect();
    branch_info.sort_unstable();
    insta::assert_debug_snapshot!(branch_info);
}

#[test]
fn test_set_ignore_ts_counts_files_correctly() {
    let temp = temp_dir();
    let build = temp.path();

    // Create modorganizer_super with a repo
    let super_path = build.join("modorganizer_super");
    let repo_path = super_path.join("test-repo");
    std::fs::create_dir_all(&repo_path).expect("failed to create repo");
    init_test_repo(&repo_path).expect("failed to init repo");

    // Create src directory with .ts files
    let src_dir = repo_path.join("src");
    std::fs::create_dir_all(&src_dir).expect("failed to create src");
    std::fs::write(src_dir.join("test1.ts"), "// test").expect("failed to create test1.ts");
    std::fs::write(src_dir.join("test2.ts"), "// test").expect("failed to create test2.ts");

    // Create subdirectory with another .ts file
    let subdir = src_dir.join("subdir");
    std::fs::create_dir_all(&subdir).expect("failed to create subdir");
    std::fs::write(subdir.join("test3.ts"), "// test").expect("failed to create test3.ts");

    // Create non-.ts file (should be ignored)
    std::fs::write(src_dir.join("test.js"), "// test").expect("failed to create test.js");

    let config = Config {
        paths: PathsConfig {
            build: Some(build.to_path_buf()),
            ..Default::default()
        },
        ..Default::default()
    };

    // Test dry-run mode
    let count = set_ignore_ts(&config, true, true).expect("set_ignore_ts should succeed");
    assert_eq!(count, 3, "should find 3 .ts files");
}

#[test]
fn test_add_remote_to_repos_filters_correctly() {
    let temp = temp_dir();
    let build = temp.path();

    // Create modorganizer_super with repos
    let super_path = build.join("modorganizer_super");
    std::fs::create_dir_all(&super_path).expect("failed to create modorganizer_super");

    let repo1 = super_path.join("repo1");
    let repo2 = super_path.join("repo2");
    std::fs::create_dir_all(&repo1).expect("failed to create repo1");
    std::fs::create_dir_all(&repo2).expect("failed to create repo2");
    init_test_repo(&repo1).expect("failed to init repo1");
    init_test_repo(&repo2).expect("failed to init repo2");

    let config = Config {
        paths: PathsConfig {
            build: Some(build.to_path_buf()),
            ..Default::default()
        },
        ..Default::default()
    };

    // Test with specific repos filter - dry run to avoid remote errors
    let result = add_remote_to_repos(
        &config,
        "test-remote",
        "testuser",
        None,
        &["repo1".to_string()],
        true, // dry_run
    );
    assert!(result.is_ok(), "add_remote_to_repos should succeed");

    // Test with empty filter (all repos) - dry run
    let result = add_remote_to_repos(&config, "test-remote", "testuser", None, &[], true);
    assert!(result.is_ok(), "add_remote_to_repos should succeed");
}

#[test]
fn test_set_remotes_for_all_dry_run() {
    let temp = temp_dir();
    let build = temp.path();

    // Create modorganizer_super with a repo
    let super_path = build.join("modorganizer_super");
    let repo_path = super_path.join("test-repo");
    std::fs::create_dir_all(&repo_path).expect("failed to create repo");
    init_test_repo(&repo_path).expect("failed to init repo");

    let config = Config {
        paths: PathsConfig {
            build: Some(build.to_path_buf()),
            ..Default::default()
        },
        ..Default::default()
    };

    // Test dry-run mode (should not fail)
    let result = set_remotes_for_all(&config, "testuser", "test@example.com", None, true);
    assert!(result.is_ok(), "set_remotes_for_all dry-run should succeed");
}

#[test]
fn test_remote_branch_exists_with_invalid_url() {
    // Use a file:// URL pointing to a nonexistent path instead of an HTTPS URL.
    // This avoids real network access while still exercising the "inaccessible remote" path:
    // git ls-remote fails → function returns Ok(false).
    let nonexistent = temp_dir();
    let invalid_path = nonexistent.path().join("does_not_exist");
    let invalid_url = format!("file://{}", invalid_path.display());

    let result = remote_branch_exists(&invalid_url, "main");
    assert!(
        result.is_ok(),
        "remote_branch_exists should not error on unreachable remotes, got: {result:?}"
    );
    // Unreachable remote should result in Ok(false)
    assert!(
        !result.unwrap(),
        "should return false for inaccessible remote"
    );
}

#[test]
fn test_remote_branch_exists_with_local_repo() {
    // Create a local "remote" repository
    let remote_repo = temp_dir();
    let branch =
        init_test_repo_with_commit(remote_repo.path()).expect("failed to init remote repo");

    // Use file:// URL to reference local repo as remote
    let remote_url = format!("file://{}", remote_repo.path().display());

    // Test that the default branch exists
    let result = remote_branch_exists(&remote_url, &branch);
    assert!(
        result.is_ok(),
        "remote_branch_exists should succeed with local repo: {result:?}"
    );
    assert!(
        result.unwrap(),
        "{branch} branch should exist in local repo"
    );

    // Test that nonexistent branch returns false
    let result = remote_branch_exists(&remote_url, "nonexistent-branch");
    assert!(result.is_ok());
    assert!(!result.unwrap(), "nonexistent branch should return false");
}

#[test]
fn test_fetch_refspec_invalid_remote() {
    let temp = temp_dir();
    let _ = init_test_repo_with_commit(temp.path()).expect("failed to init repo");

    // Use a file:// URL pointing to a nonexistent path instead of an HTTPS URL.
    // This avoids real network access while still exercising the "fetch from
    // unreachable remote" error path.
    let nonexistent = temp_dir();
    let invalid_path = nonexistent.path().join("does_not_exist");
    let invalid_url = format!("file://{}", invalid_path.display());

    let result = fetch_refspec(
        temp.path(),
        &invalid_url,
        "refs/heads/main:refs/remotes/origin/main",
    );

    assert!(
        result.is_err(),
        "fetch_refspec should fail with unreachable remote"
    );
}

#[test]
fn test_fetch_refspec_with_local_repo() {
    // Create a local "remote" repository with a commit
    let remote_repo = temp_dir();
    let branch =
        init_test_repo_with_commit(remote_repo.path()).expect("failed to init remote repo");

    // Create local repository to fetch into
    let local_repo = temp_dir();
    let _ = init_test_repo_with_commit(local_repo.path()).expect("failed to init local repo");

    // Use file:// URL to reference local repo as remote
    let remote_url = format!("file://{}", remote_repo.path().display());

    // Fetch the default branch from "remote"
    let refspec = format!("refs/heads/{branch}:refs/remotes/origin/{branch}");
    let result = fetch_refspec(local_repo.path(), &remote_url, &refspec);

    assert!(
        result.is_ok(),
        "fetch_refspec should succeed with local repo: {result:?}"
    );

    // Verify the ref was fetched by checking it exists
    let verify_ref = format!("refs/remotes/origin/{branch}");
    let output = Command::new("git")
        .args(["rev-parse", "--verify", &verify_ref])
        .current_dir(local_repo.path())
        .output()
        .expect("failed to run git rev-parse");

    assert!(
        output.status.success(),
        "fetched ref should exist: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
