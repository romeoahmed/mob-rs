// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::cli::Cli;
use clap::Parser;

#[test]
fn test_parse_version() {
    let cli = Cli::try_parse_from(["mob", "version"]).unwrap();
    insta::assert_debug_snapshot!("parse_version", cli);
}

#[test]
fn test_parse_global_options() {
    let cli = Cli::try_parse_from(["mob", "-l", "5", "-d", "/tmp/mo2", "--dry", "build"]).unwrap();
    insta::assert_debug_snapshot!("parse_global_options", cli);
}

#[test]
fn test_parse_git_set_remotes() {
    let cli = Cli::try_parse_from([
        "mob",
        "git",
        "set-remotes",
        "-u",
        "myuser",
        "-e",
        "myemail@example.com",
    ])
    .unwrap();
    insta::assert_debug_snapshot!("parse_git_set_remotes", cli);
}

#[test]
fn test_parse_pr() {
    let cli = Cli::try_parse_from(["mob", "pr", "find", "modorganizer/123"]).unwrap();
    insta::assert_debug_snapshot!("parse_pr", cli);
}

#[test]
fn test_parse_tx_get() {
    let cli = Cli::try_parse_from(["mob", "tx", "get", "-m", "80", "/path/to/tx"]).unwrap();
    insta::assert_debug_snapshot!("parse_tx_get", cli);
}
