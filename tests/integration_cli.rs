// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Integration tests for CLI parsing.
//!
//! Tests the CLI module with realistic command-line argument patterns.

use clap::Parser;
use mob_rs::cli::Cli;
use mob_rs::cli::build::{BuildArgs, CleanFullArgs};
use mob_rs::cli::global::GlobalOptions;

// =============================================================================
// Version Command
// =============================================================================

#[test]
fn cli_version_command() {
    let cli = Cli::try_parse_from(["mob", "version"]).unwrap();
    insta::assert_debug_snapshot!(cli);
}

#[test]
fn cli_version_alias() {
    let cli = Cli::try_parse_from(["mob", "-v"]).unwrap();
    insta::assert_debug_snapshot!(cli);
}

// =============================================================================
// Build Command
// =============================================================================

#[test]
fn cli_build_no_args() {
    let cli = Cli::try_parse_from(["mob", "build"]).unwrap();
    insta::assert_debug_snapshot!(cli);
}

#[test]
fn cli_build_with_tasks() {
    let cli = Cli::try_parse_from(["mob", "build", "usvfs", "cmake_common", "qt"]).unwrap();
    insta::assert_debug_snapshot!(cli);
}

#[test]
fn cli_build_new_flag() {
    let cli = Cli::try_parse_from(["mob", "build", "-n"]).unwrap();
    insta::assert_debug_snapshot!(cli);
}

#[test]
fn cli_build_all_rebuild_flags() {
    let cli = Cli::try_parse_from([
        "mob",
        "build",
        "-g",
        "-e",
        "-c",
        "-b",
        "--ignore-uncommitted-changes",
    ])
    .unwrap();
    insta::assert_debug_snapshot!(cli);
}

#[test]
fn cli_build_task_phase_flags() {
    // Test --no-fetch-task --build-task
    let cli = Cli::try_parse_from(["mob", "build", "--no-fetch-task", "--build-task"]).unwrap();
    insta::assert_debug_snapshot!(cli);
}

#[test]
fn cli_build_conflicting_flags_rejected() {
    // --clean-task and --no-clean-task should conflict
    let result = Cli::try_parse_from(["mob", "build", "--clean-task", "--no-clean-task"]);
    assert!(result.is_err());
}

// =============================================================================
// Global Options
// =============================================================================

#[test]
fn cli_global_options_prefix() {
    let cli = Cli::try_parse_from(["mob", "-d", "/tmp/mo2/build", "build"]).unwrap();
    insta::assert_debug_snapshot!(cli);
}

#[test]
fn cli_global_options_log_levels() {
    let cli = Cli::try_parse_from(["mob", "-l", "5", "--file-log-level", "3", "build"]).unwrap();
    insta::assert_debug_snapshot!(cli);
}

#[test]
fn cli_global_options_dry_run() {
    let cli = Cli::try_parse_from(["mob", "--dry", "build"]).unwrap();
    insta::assert_debug_snapshot!(cli);
}

#[test]
fn cli_global_options_multiple_inis() {
    let cli =
        Cli::try_parse_from(["mob", "-i", "base.toml", "-i", "override.toml", "build"]).unwrap();
    insta::assert_debug_snapshot!(cli);
}

#[test]
fn cli_global_options_set_options() {
    let cli = Cli::try_parse_from([
        "mob",
        "-s",
        "versions/qt=6.7.0",
        "-s",
        "global/dry=true",
        "build",
    ])
    .unwrap();
    insta::assert_debug_snapshot!(cli);
}

#[test]
fn cli_global_options_to_config_overrides() {
    let opts = GlobalOptions {
        log_level: Some(4),
        dry: true,
        prefix: Some(std::path::PathBuf::from("/build")),
        options: vec!["custom/key=value".to_string()],
        ..Default::default()
    };
    let overrides = opts.to_config_overrides();
    insta::assert_debug_snapshot!(overrides);
}

// =============================================================================
// List Command
// =============================================================================

#[test]
fn cli_list_all_tasks() {
    let cli = Cli::try_parse_from(["mob", "list", "-a"]).unwrap();
    insta::assert_debug_snapshot!(cli);
}

#[test]
fn cli_list_aliases_only() {
    let cli = Cli::try_parse_from(["mob", "list", "-i"]).unwrap();
    insta::assert_debug_snapshot!(cli);
}

// =============================================================================
// Git Command
// =============================================================================

#[test]
fn cli_git_set_remotes() {
    let cli = Cli::try_parse_from([
        "mob",
        "git",
        "set-remotes",
        "-u",
        "myuser",
        "-e",
        "user@example.com",
        "-s",
        "-p",
    ])
    .unwrap();
    insta::assert_debug_snapshot!(cli);
}

#[test]
fn cli_git_ignore_ts_on() {
    let cli = Cli::try_parse_from(["mob", "git", "ignore-ts", "on"]).unwrap();
    insta::assert_debug_snapshot!(cli);
}

#[test]
fn cli_git_ignore_ts_off() {
    let cli = Cli::try_parse_from(["mob", "git", "ignore-ts", "off"]).unwrap();
    insta::assert_debug_snapshot!(cli);
}

#[test]
fn cli_git_branches_all() {
    let cli = Cli::try_parse_from(["mob", "git", "branches", "-a"]).unwrap();
    insta::assert_debug_snapshot!(cli);
}

// =============================================================================
// Release Command
// =============================================================================

#[test]
fn cli_release_devbuild() {
    let cli = Cli::try_parse_from(["mob", "release", "devbuild", "--no-src", "--force"]).unwrap();
    insta::assert_debug_snapshot!(cli);
}

#[test]
fn cli_release_official() {
    let cli = Cli::try_parse_from(["mob", "release", "official", "release-2.5"]).unwrap();
    insta::assert_debug_snapshot!(cli);
}

// =============================================================================
// PR Command
// =============================================================================

#[test]
fn cli_pr_find() {
    let cli = Cli::try_parse_from(["mob", "pr", "find", "modorganizer/456"]).unwrap();
    insta::assert_debug_snapshot!(cli);
}

#[test]
fn cli_pr_pull() {
    let cli = Cli::try_parse_from(["mob", "pr", "pull", "usvfs/123"]).unwrap();
    insta::assert_debug_snapshot!(cli);
}

// =============================================================================
// CMake Config Command
// =============================================================================

#[test]
fn cli_cmake_config_prefix_path() {
    let cli = Cli::try_parse_from(["mob", "cmake-config", "prefix-path"]).unwrap();
    insta::assert_debug_snapshot!(cli);
}

#[test]
fn cli_cmake_config_install_prefix() {
    let cli = Cli::try_parse_from(["mob", "cmake-config", "install-prefix"]).unwrap();
    insta::assert_debug_snapshot!(cli);
}

// =============================================================================
// BuildArgs Helper Methods
// =============================================================================

#[test]
fn build_args_to_config_overrides() {
    let args = BuildArgs {
        clean_full: CleanFullArgs { new_build: true },
        tasks: vec!["super".to_string()],
        ..Default::default()
    };
    let overrides = args.to_config_overrides();
    insta::assert_debug_snapshot!(overrides);
}

// =============================================================================
// Error Cases
// =============================================================================

#[test]
fn cli_invalid_log_level() {
    // Log level must be 0-6
    let result = Cli::try_parse_from(["mob", "-l", "10", "build"]);
    assert!(result.is_err());
}

#[test]
fn cli_missing_required_args() {
    // git set-remotes requires -u and -e
    let result = Cli::try_parse_from(["mob", "git", "set-remotes", "-u", "user"]);
    assert!(result.is_err());
}
