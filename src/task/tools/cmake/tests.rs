// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::Arc;

use anyhow::Result;
use tokio_util::sync::CancellationToken;

use super::{CmakeArchitecture, CmakeGenerator, CmakeTool};
use crate::config::Config;
use crate::config::types::BuildConfiguration;
use crate::task::tools::test_utils::run_with_logs;
use crate::task::tools::{Tool, ToolContext};

/// Strip timestamp prefix from log lines, keeping only the [dry-run] message
fn normalize_dry_run_logs(logs: &str) -> String {
    logs.lines()
        .map(|line| line.find(" [dry-run]").map_or(line, |idx| &line[idx..]))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn test_cmake_builder_defaults() {
    let tool = CmakeTool::new();
    insta::assert_debug_snapshot!(tool);
}

#[tokio::test(flavor = "current_thread")]
async fn test_cmake_configure_dry_run() -> Result<()> {
    let logs = run_with_logs(|| async {
        let config = Arc::new(Config::default());
        let ctx = ToolContext::new(config, CancellationToken::new(), true);

        let tool = CmakeTool::new()
            .source_dir("/tmp/source")
            .build_dir("/tmp/build")
            .generator(CmakeGenerator::Ninja)
            .architecture(CmakeArchitecture::X64)
            .configure_op();

        tool.run(&ctx).await
    })
    .await?;

    insta::assert_snapshot!(normalize_dry_run_logs(&logs));
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn test_cmake_build_dry_run() -> Result<()> {
    let logs = run_with_logs(|| async {
        let config = Arc::new(Config::default());
        let ctx = ToolContext::new(config, CancellationToken::new(), true);

        let tool = CmakeTool::new()
            .build_dir("/tmp/build")
            .configuration(BuildConfiguration::Release)
            .target("all")
            .build_op();

        tool.run(&ctx).await
    })
    .await?;

    insta::assert_snapshot!(normalize_dry_run_logs(&logs));
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn test_cmake_install_dry_run() -> Result<()> {
    let logs = run_with_logs(|| async {
        let config = Arc::new(Config::default());
        let ctx = ToolContext::new(config, CancellationToken::new(), true);

        let tool = CmakeTool::new()
            .build_dir("/tmp/build")
            .install_prefix("/tmp/install")
            .configuration(BuildConfiguration::RelWithDebInfo)
            .install_op();

        tool.run(&ctx).await
    })
    .await?;

    insta::assert_snapshot!(normalize_dry_run_logs(&logs));
    Ok(())
}
