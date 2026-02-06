// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::Arc;

use anyhow::Result;
use tokio_util::sync::CancellationToken;

use super::MsBuildTool;
use crate::config::Config;
use crate::config::types::BuildConfiguration;
use crate::core::env::types::Arch;
use crate::task::tools::test_utils::run_with_logs;
use crate::task::tools::{Tool, ToolContext};

#[test]
fn test_msbuild_builder_defaults() {
    let tool = MsBuildTool::new();

    insta::assert_debug_snapshot!("msbuild_builder_defaults", tool);
}

#[test]
fn test_msbuild_builder_chain() {
    let tool = MsBuildTool::new()
        .solution("solution.sln")
        .target("Build")
        .configuration(BuildConfiguration::Release)
        .platform("x64")
        .max_cpu_count(true);

    insta::assert_debug_snapshot!("msbuild_builder_chain", tool);
}

#[tokio::test(flavor = "current_thread")]
async fn test_msbuild_build_dry_run() -> Result<()> {
    let logs = run_with_logs(|| async {
        let config = Arc::new(Config::default());
        let ctx = ToolContext::new(config, CancellationToken::new(), true);

        let tool = MsBuildTool::new()
            .solution("test.sln")
            .configuration(BuildConfiguration::Release)
            .build_op();

        tool.run(&ctx).await
    })
    .await?;

    assert!(
        logs.contains("[dry-run] Would build with MSBuild"),
        "log output should include dry-run build message: {logs}"
    );
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn test_msbuild_clean_dry_run() -> Result<()> {
    let logs = run_with_logs(|| async {
        let config = Arc::new(Config::default());
        let ctx = ToolContext::new(config, CancellationToken::new(), true);

        let tool = MsBuildTool::new()
            .solution("test.sln")
            .configuration(BuildConfiguration::Debug)
            .clean_op();

        tool.run(&ctx).await
    })
    .await?;

    assert!(
        logs.contains("[dry-run] Would clean with MSBuild"),
        "log output should include dry-run clean message: {logs}"
    );
    Ok(())
}

#[test]
fn test_msbuild_toolset_conversion() {
    let conversions: Vec<(&str, String)> = vec![
        ("14.3", MsBuildTool::convert_toolset_version("14.3")),
        ("14.2", MsBuildTool::convert_toolset_version("14.2")),
        ("14.1", MsBuildTool::convert_toolset_version("14.1")),
        ("14.0", MsBuildTool::convert_toolset_version("14.0")),
        ("13.0", MsBuildTool::convert_toolset_version("13.0")),
    ];
    insta::assert_debug_snapshot!(conversions);
}

#[test]
fn test_msbuild_platform_determination() {
    let platforms: Vec<(&str, String)> = vec![
        (
            "explicit ARM64 overrides x64 arch",
            MsBuildTool::new()
                .platform("ARM64")
                .architecture(Arch::X64)
                .determine_platform(),
        ),
        (
            "x86 arch maps to Win32",
            MsBuildTool::new()
                .architecture(Arch::X86)
                .determine_platform(),
        ),
        (
            "x64 arch maps to x64",
            MsBuildTool::new()
                .architecture(Arch::X64)
                .determine_platform(),
        ),
        ("default is x64", MsBuildTool::new().determine_platform()),
    ];
    insta::assert_debug_snapshot!(platforms);
}

#[test]
fn test_msbuild_multiple_targets() {
    let tool = MsBuildTool::new().targets(vec!["Build", "Test"]);

    insta::assert_debug_snapshot!("msbuild_multiple_targets", tool);
}

#[test]
fn test_msbuild_properties_deterministic() {
    let tool = MsBuildTool::new()
        .property("PropA", "ValueA")
        .property("PropB", "ValueB")
        .property("PropC", "ValueC");

    insta::assert_debug_snapshot!("msbuild_properties_deterministic", tool);
}
