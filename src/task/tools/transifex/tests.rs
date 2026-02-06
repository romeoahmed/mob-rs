// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

use super::TransifexTool;
use crate::task::tools::Tool;

#[test]
fn test_transifex_tool_builder() {
    let tool = TransifexTool::new()
        .root("/path/to/tx")
        .api_key("secret")
        .url("https://example.com")
        .minimum(60)
        .force(true)
        .pull_op();

    insta::assert_debug_snapshot!("transifex_tool_builder", tool);
}

#[test]
fn test_transifex_tool_minimum_clamped() {
    let tool = TransifexTool::new().minimum(150);
    assert_eq!(tool.minimum, 100);
}

#[test]
fn test_transifex_tool_operations() {
    let operations = [
        ("init", TransifexTool::new().init_op().operation),
        ("config", TransifexTool::new().config_op().operation),
        ("pull", TransifexTool::new().pull_op().operation),
    ];
    insta::assert_debug_snapshot!("transifex_operations", operations);
}

#[test]
fn test_transifex_tool_name() {
    let tool = TransifexTool::new();
    assert_eq!(tool.name(), "transifex");
}
