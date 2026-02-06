// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

use super::{LogContext, LogLevel};

#[test]
fn test_log_context_clear_tool() {
    let mut ctx = LogContext::with_task("cmake");
    ctx.set_tool("process");

    let results: Vec<_> = [("with_tool", ctx.prefix()), {
        ctx.clear_tool();
        ("after_clear", ctx.prefix())
    }]
    .into_iter()
    .collect();

    insta::assert_yaml_snapshot!("log_context_clear_tool", results);
    assert!(ctx.tool().is_none(), "tool should be None after clear_tool");
}

#[test]
fn test_log_level_conversion() {
    let conversions = vec![
        ("from_int(0)", LogLevel::from_int(0)),
        ("from_int(3)", LogLevel::from_int(3)),
        ("from_int(5)", LogLevel::from_int(5)),
        ("from_int(100)", LogLevel::from_int(100)),
    ];
    insta::assert_debug_snapshot!(conversions);
}

#[test]
fn test_log_context_prefix() {
    let ctx_task_only = LogContext::with_task("cmake");
    let mut ctx_with_tool = LogContext::with_task("msbuild");
    ctx_with_tool.set_tool("process");
    let ctx_empty = LogContext::default();

    insta::assert_yaml_snapshot!(
        "prefixes",
        vec![
            ("task_only", ctx_task_only.prefix()),
            ("with_tool", ctx_with_tool.prefix()),
            ("empty", ctx_empty.prefix()),
        ]
    );
}
