// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

use super::IsccTool;
use crate::task::tools::Tool;

#[test]
fn test_iscc_tool_builder() {
    let tool = IsccTool::new()
        .iss("/path/to/script.iss")
        .output_dir("/output")
        .output_name("MyInstaller")
        .define("VERSION", "1.0.0")
        .define("ARCH", "x64");

    insta::assert_debug_snapshot!("iscc_tool_builder", tool);
}

#[test]
fn test_iscc_tool_name() {
    let tool = IsccTool::new();
    insta::assert_snapshot!("iscc_tool_name", tool.name());
}

#[test]
fn test_iscc_tool_default() {
    let tool = IsccTool::default();
    insta::assert_debug_snapshot!("iscc_tool_default", tool);
}
