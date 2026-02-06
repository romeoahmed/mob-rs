// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

use super::{GitOperation, GitTool};
use crate::task::tools::Tool;

#[test]
fn test_git_tool_builder() {
    let tool = GitTool::new()
        .url("https://github.com/example/repo.git")
        .path("/tmp/repo")
        .branch("main")
        .shallow(true);

    insta::assert_debug_snapshot!("git_tool_builder", tool);
}

#[test]
fn test_git_tool_all_operations() {
    // All GitOperation variants with their builder methods
    let operations: Vec<(&str, GitOperation)> = vec![
        ("clone_op", GitTool::new().clone_op().operation),
        ("pull_op", GitTool::new().pull_op().operation),
        ("fetch_op", GitTool::new().fetch_op().operation),
        ("checkout_op", GitTool::new().checkout_op().operation),
        (
            "submodule_update_op",
            GitTool::new().submodule_update_op().operation,
        ),
        ("reset_op", GitTool::new().reset_op().operation),
    ];
    insta::assert_debug_snapshot!(operations);
}

#[test]
fn test_git_tool_new_fields() {
    let tool = GitTool::new()
        .path("/tmp/repo")
        .remote("upstream")
        .target("v1.0.0")
        .force(true)
        .recursive(false);

    insta::assert_debug_snapshot!("git_tool_new_fields", tool);
}

#[test]
fn test_git_tool_get_remote() {
    // Remote resolution: explicit > default "origin"
    let default = GitTool::new();
    let custom = GitTool::new().remote("upstream");
    let remotes: Vec<(&str, &str)> = vec![
        ("default", default.get_remote()),
        ("custom upstream", custom.get_remote()),
    ];
    insta::assert_debug_snapshot!(remotes);
}

#[test]
fn test_git_tool_name() {
    let tool = GitTool::new();
    insta::assert_snapshot!("git_tool_name", tool.name());
}

#[test]
fn test_git_tool_default() {
    let tool = GitTool::default();
    insta::assert_debug_snapshot!("git_tool_default", tool);
}

#[test]
fn test_git_tool_checkout_builder() {
    let tool = GitTool::new()
        .path("/tmp/repo")
        .target("feature-branch")
        .checkout_op();

    insta::assert_debug_snapshot!("git_tool_checkout_builder", tool);
}

#[test]
fn test_git_tool_fetch_builder() {
    let tool = GitTool::new()
        .path("/tmp/repo")
        .remote("upstream")
        .fetch_op();

    insta::assert_debug_snapshot!("git_tool_fetch_builder", tool);
}

#[test]
fn test_git_tool_reset_builder() {
    let tool = GitTool::new()
        .path("/tmp/repo")
        .target("HEAD~1")
        .force(true)
        .reset_op();

    insta::assert_debug_snapshot!("git_tool_reset_builder", tool);
}

#[test]
fn test_git_tool_submodule_update_builder() {
    let tool = GitTool::new()
        .path("/tmp/repo")
        .recursive(true)
        .submodule_update_op();

    insta::assert_debug_snapshot!("git_tool_submodule_update_builder", tool);
}
