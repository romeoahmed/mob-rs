// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::Arc;

use anyhow::Result;
use tokio_util::sync::CancellationToken;

use super::{ArchiveFormat, ExtractorTool};
use crate::config::Config;
use crate::task::tools::test_utils::run_with_logs;
use crate::task::tools::{Tool, ToolContext};
use std::path::Path;

#[test]
fn test_extractor_builder_defaults() {
    let tool = ExtractorTool::new();

    insta::assert_debug_snapshot!("extractor_builder_defaults", tool);
}

#[test]
fn test_extractor_format_detection() {
    // Format detection from archive extension (case-insensitive)
    let detections: Vec<(&str, Option<ArchiveFormat>)> = vec![
        (
            "archive.7z",
            ExtractorTool::new()
                .archive("archive.7z")
                .detect_format()
                .ok(),
        ),
        (
            "archive.zip",
            ExtractorTool::new()
                .archive("archive.zip")
                .detect_format()
                .ok(),
        ),
        (
            "archive.tar.gz",
            ExtractorTool::new()
                .archive("archive.tar.gz")
                .detect_format()
                .ok(),
        ),
        (
            "archive.tgz",
            ExtractorTool::new()
                .archive("archive.tgz")
                .detect_format()
                .ok(),
        ),
        (
            "archive.tar",
            ExtractorTool::new()
                .archive("archive.tar")
                .detect_format()
                .ok(),
        ),
        (
            "ARCHIVE.ZIP (case-insensitive)",
            ExtractorTool::new()
                .archive("ARCHIVE.ZIP")
                .detect_format()
                .ok(),
        ),
        (
            "archive.rar (unsupported)",
            ExtractorTool::new()
                .archive("archive.rar")
                .detect_format()
                .ok(),
        ),
    ];
    insta::assert_debug_snapshot!(detections);
}

#[tokio::test(flavor = "current_thread")]
async fn test_extractor_extract_dry_run() -> Result<()> {
    let logs = run_with_logs(|| async {
        let config = Arc::new(Config::default());
        let ctx = ToolContext::new(config, CancellationToken::new(), true);

        let tool = ExtractorTool::new()
            .archive("/tmp/archive.zip")
            .output("/tmp/extracted")
            .extract_op();

        tool.run(&ctx).await
    })
    .await?;

    assert!(
        logs.contains("[dry-run] Would extract archive"),
        "log output should include dry-run extract message: {logs}"
    );
    Ok(())
}

#[test]
fn test_archive_format_from_extension() {
    // Static method: ArchiveFormat::from_extension
    let extensions: Vec<(&str, Option<ArchiveFormat>)> = vec![
        (
            "test.7z",
            ArchiveFormat::from_extension(Path::new("test.7z")),
        ),
        (
            "test.zip",
            ArchiveFormat::from_extension(Path::new("test.zip")),
        ),
        (
            "test.tar.gz",
            ArchiveFormat::from_extension(Path::new("test.tar.gz")),
        ),
        (
            "test.tgz",
            ArchiveFormat::from_extension(Path::new("test.tgz")),
        ),
        (
            "test.tar",
            ArchiveFormat::from_extension(Path::new("test.tar")),
        ),
        (
            "test.rar (unsupported)",
            ArchiveFormat::from_extension(Path::new("test.rar")),
        ),
    ];
    insta::assert_debug_snapshot!(extensions);
}

#[test]
fn test_extractor_builder_chaining() {
    let tool = ExtractorTool::new()
        .archive("/tmp/archive.zip")
        .output("/tmp/extracted")
        .force(true)
        .extract_op();

    insta::assert_debug_snapshot!("extractor_builder_chaining", tool);
}

#[tokio::test(flavor = "current_thread")]
async fn test_extractor_clean_dry_run() -> Result<()> {
    let logs = run_with_logs(|| async {
        let config = Arc::new(Config::default());
        let ctx = ToolContext::new(config, CancellationToken::new(), true);

        let tool = ExtractorTool::new().output("/tmp/extracted").clean_op();

        tool.run(&ctx).await
    })
    .await?;

    assert!(
        logs.contains("[dry-run] Would clean output directory"),
        "log output should include dry-run clean message: {logs}"
    );
    Ok(())
}
