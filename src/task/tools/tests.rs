// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

use super::ToolContext;
use crate::config::Config;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

#[test]
fn test_tool_context_creation() {
    let config = Arc::new(Config::default());
    let token = CancellationToken::new();
    let ctx = ToolContext::new(config, token, false);

    assert!(!ctx.is_cancelled());
    assert!(!ctx.is_dry_run());
}

#[test]
fn test_tool_context_cancellation() {
    let config = Arc::new(Config::default());
    let token = CancellationToken::new();
    let ctx = ToolContext::new(config, token.clone(), false);

    assert!(!ctx.is_cancelled());
    token.cancel();
    assert!(ctx.is_cancelled());
}

#[test]
fn test_tool_context_dry_run() {
    let config = Arc::new(Config::default());
    let token = CancellationToken::new();
    let ctx = ToolContext::new(config, token, true);

    assert!(ctx.is_dry_run());
}
