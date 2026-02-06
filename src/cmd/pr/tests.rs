// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

use super::{find_local_repo, parse_pr_arg};
use tempfile::TempDir;

fn temp_dir() -> TempDir {
    tempfile::tempdir().expect("failed to create temp dir")
}

#[test]
fn test_parse_pr_arg_cases() {
    let results: Vec<_> = ["modorganizer/123", "456"]
        .into_iter()
        .map(|input| {
            let result = parse_pr_arg(input);
            (input, format!("{result:?}"))
        })
        .collect();

    insta::assert_yaml_snapshot!("parse_pr_arg_cases", results);
}

#[test]
fn test_parse_pr_arg_invalid() {
    // These should all fail - just verify they're errors
    assert!(parse_pr_arg("invalid").is_err());
    assert!(parse_pr_arg("repo/invalid").is_err());
    assert!(parse_pr_arg("repo/123/extra").is_err());
}

#[test]
fn test_find_local_repo_usvfs() {
    use crate::config::Config;
    use crate::config::paths::PathsConfig;
    use std::process::Command;

    let temp = temp_dir();
    let build = temp.path();

    // Create usvfs repo
    let usvfs_path = build.join("usvfs");
    std::fs::create_dir_all(&usvfs_path).expect("failed to create usvfs");
    // Initialize git repo using shell command
    Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(&usvfs_path)
        .output()
        .expect("failed to init usvfs repo");

    let config = Config {
        paths: PathsConfig {
            build: Some(build.to_path_buf()),
            ..Default::default()
        },
        ..Default::default()
    };

    let found = find_local_repo(&config, "usvfs");
    assert!(found.is_some());
    assert!(found.unwrap().ends_with("usvfs"));
}

#[test]
fn test_find_local_repo_modorganizer() {
    use crate::config::Config;
    use crate::config::paths::PathsConfig;
    use std::process::Command;

    let temp = temp_dir();
    let build = temp.path();

    // Create modorganizer repo
    let super_path = build.join("modorganizer_super");
    let repo_path = super_path.join("modorganizer");
    std::fs::create_dir_all(&repo_path).expect("failed to create repo");
    // Initialize git repo using shell command (avoid git2 dependency)
    Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(&repo_path)
        .output()
        .expect("failed to init repo");

    let config = Config {
        paths: PathsConfig {
            build: Some(build.to_path_buf()),
            ..Default::default()
        },
        ..Default::default()
    };

    let found = find_local_repo(&config, "modorganizer");
    assert!(found.is_some());
    assert!(found.unwrap().ends_with("modorganizer"));
}

#[test]
fn test_find_local_repo_not_found() {
    use crate::config::Config;
    use crate::config::paths::PathsConfig;

    let temp = temp_dir();
    let build = temp.path();

    let config = Config {
        paths: PathsConfig {
            build: Some(build.to_path_buf()),
            ..Default::default()
        },
        ..Default::default()
    };

    let found = find_local_repo(&config, "nonexistent");
    assert!(found.is_none());
}
