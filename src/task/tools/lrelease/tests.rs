// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

use super::LreleaseTool;
use crate::task::tools::Tool;
use std::path::PathBuf;

#[test]
fn test_lrelease_tool_builder() {
    let tool = LreleaseTool::new()
        .project("modorganizer")
        .add_source("/path/to/fr.ts")
        .add_source("/path/to/gamebryo/fr.ts")
        .output_dir("/install/bin/translations");

    insta::assert_debug_snapshot!("lrelease_tool_builder", tool);
}

#[test]
fn test_lrelease_tool_qm_filename() {
    let tool = LreleaseTool::new()
        .project("modorganizer")
        .add_source("/path/to/fr.ts")
        .output_dir("/output");

    assert_eq!(tool.qm_filename().unwrap(), "modorganizer_fr.qm");
}

#[test]
fn test_lrelease_tool_qm_path() {
    let tool = LreleaseTool::new()
        .project("uibase")
        .add_source("/path/to/de.ts")
        .output_dir("/install/bin/translations");

    assert_eq!(
        tool.qm_path().unwrap(),
        PathBuf::from("/install/bin/translations/uibase_de.qm")
    );
}

#[test]
fn test_lrelease_tool_sources() {
    let sources = vec!["/a.ts", "/b.ts", "/c.ts"];
    let tool = LreleaseTool::new().sources(sources);

    insta::assert_debug_snapshot!("lrelease_tool_sources", tool);
}

#[test]
fn test_lrelease_tool_name() {
    let tool = LreleaseTool::new();
    assert_eq!(tool.name(), "lrelease");
}

#[test]
fn test_lrelease_tool_missing_project() {
    let tool = LreleaseTool::new()
        .add_source("/path/to/fr.ts")
        .output_dir("/output");

    assert!(tool.qm_filename().is_err());
}

#[test]
fn test_lrelease_tool_missing_sources() {
    let tool = LreleaseTool::new().project("test").output_dir("/output");

    assert!(tool.qm_filename().is_err());
}
