// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Integration tests for release command CLI parsing.
//!
//! Tests the release subcommand with various argument patterns.

use clap::Parser;
use mob_rs::cli::Cli;

// =============================================================================
// Release Devbuild Command
// =============================================================================

#[test]
fn release_devbuild_basic() {
    let cli = Cli::try_parse_from(["mob", "release", "devbuild"]).unwrap();
    insta::assert_debug_snapshot!(cli);
}

#[test]
fn release_devbuild_with_version() {
    let cli = Cli::try_parse_from(["mob", "release", "devbuild", "--version", "2.5.0"]).unwrap();
    insta::assert_debug_snapshot!(cli);
}

#[test]
fn release_devbuild_with_suffix() {
    let cli = Cli::try_parse_from([
        "mob",
        "release",
        "devbuild",
        "--version",
        "2.5.0",
        "--suffix",
        "rc1",
    ])
    .unwrap();
    insta::assert_debug_snapshot!(cli);
}

#[test]
fn release_devbuild_no_bin() {
    let cli = Cli::try_parse_from(["mob", "release", "devbuild", "--no-bin"]).unwrap();
    insta::assert_debug_snapshot!(cli);
}

#[test]
fn release_devbuild_no_pdbs() {
    let cli = Cli::try_parse_from(["mob", "release", "devbuild", "--no-pdbs"]).unwrap();
    insta::assert_debug_snapshot!(cli);
}

#[test]
fn release_devbuild_no_src() {
    let cli = Cli::try_parse_from(["mob", "release", "devbuild", "--no-src"]).unwrap();
    insta::assert_debug_snapshot!(cli);
}

#[test]
fn release_devbuild_all_no_flags() {
    let cli = Cli::try_parse_from([
        "mob",
        "release",
        "devbuild",
        "--no-bin",
        "--no-pdbs",
        "--no-src",
        "--no-inst",
    ])
    .unwrap();
    insta::assert_debug_snapshot!(cli);
}

#[test]
fn release_devbuild_explicit_includes() {
    let cli = Cli::try_parse_from([
        "mob", "release", "devbuild", "--bin", "--pdbs", "--src", "--inst",
    ])
    .unwrap();
    insta::assert_debug_snapshot!(cli);
}

#[test]
fn release_devbuild_with_output_dir() {
    let cli = Cli::try_parse_from([
        "mob",
        "release",
        "devbuild",
        "--output-dir",
        "/custom/output",
        "--version",
        "2.5.0",
    ])
    .unwrap();
    insta::assert_debug_snapshot!(cli);
}

#[test]
fn release_devbuild_with_force() {
    let cli = Cli::try_parse_from(["mob", "release", "devbuild", "--force"]).unwrap();
    insta::assert_debug_snapshot!(cli);
}

#[test]
fn release_devbuild_version_from_exe() {
    let cli = Cli::try_parse_from(["mob", "release", "devbuild", "--version-from-exe"]).unwrap();
    insta::assert_debug_snapshot!(cli);
}

#[test]
fn release_devbuild_version_from_rc() {
    let cli = Cli::try_parse_from(["mob", "release", "devbuild", "--version-from-rc"]).unwrap();
    insta::assert_debug_snapshot!(cli);
}

#[test]
fn release_devbuild_custom_rc_path() {
    let cli =
        Cli::try_parse_from(["mob", "release", "devbuild", "--rc", "/path/to/version.rc"]).unwrap();
    insta::assert_debug_snapshot!(cli);
}

// =============================================================================
// Release Official Command
// =============================================================================

#[test]
fn release_official_basic() {
    // branch is a positional argument
    let cli = Cli::try_parse_from(["mob", "release", "official", "v2.5.0"]).unwrap();
    insta::assert_debug_snapshot!(cli);
}

#[test]
fn release_official_with_output_dir() {
    let cli = Cli::try_parse_from([
        "mob",
        "release",
        "official",
        "v2.5.0",
        "--output-dir",
        "/release/output",
    ])
    .unwrap();
    insta::assert_debug_snapshot!(cli);
}

#[test]
fn release_official_no_bin() {
    let cli = Cli::try_parse_from(["mob", "release", "official", "v2.5.0", "--no-bin"]).unwrap();
    insta::assert_debug_snapshot!(cli);
}

#[test]
fn release_official_no_pdbs() {
    let cli = Cli::try_parse_from(["mob", "release", "official", "v2.5.0", "--no-pdbs"]).unwrap();
    insta::assert_debug_snapshot!(cli);
}

#[test]
fn release_official_no_installer() {
    let cli =
        Cli::try_parse_from(["mob", "release", "official", "v2.5.0", "--no-installer"]).unwrap();
    insta::assert_debug_snapshot!(cli);
}

#[test]
fn release_official_with_force() {
    let cli = Cli::try_parse_from(["mob", "release", "official", "v2.5.0", "--force"]).unwrap();
    insta::assert_debug_snapshot!(cli);
}

#[test]
fn release_official_full_options() {
    let cli = Cli::try_parse_from([
        "mob",
        "release",
        "official",
        "v2.5.0",
        "--output-dir",
        "/releases",
        "--bin",
        "--pdbs",
        "--force",
    ])
    .unwrap();
    insta::assert_debug_snapshot!(cli);
}

// =============================================================================
// Error Cases
// =============================================================================

#[test]
fn release_official_requires_branch() {
    let result = Cli::try_parse_from(["mob", "release", "official"]);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("BRANCH"),
        "expected error about branch, got: {err}"
    );
}

#[test]
fn release_invalid_subcommand() {
    let result = Cli::try_parse_from(["mob", "release", "invalid"]);
    assert!(result.is_err());
}
