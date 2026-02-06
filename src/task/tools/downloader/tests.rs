// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

use super::{DownloaderOperation, DownloaderTool};
use crate::task::tools::{Tool, ToolContext};
use std::path::PathBuf;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

fn create_test_ctx(dry_run: bool) -> ToolContext {
    ToolContext::new(
        Arc::new(crate::config::Config::default()),
        CancellationToken::new(),
        dry_run,
    )
}

#[test]
fn test_downloader_tool_new() {
    let tool = DownloaderTool::new();
    insta::assert_debug_snapshot!("downloader_tool_new", tool);
}

#[test]
fn test_downloader_tool_url() {
    let tool = DownloaderTool::new().url("https://example.com/file.zip");
    insta::assert_debug_snapshot!("downloader_tool_url", tool);
}

#[test]
fn test_downloader_tool_urls() {
    let urls = vec![
        "https://example.com/file.zip".to_string(),
        "https://backup.com/file.zip".to_string(),
    ];
    let tool = DownloaderTool::new().urls(urls);
    insta::assert_debug_snapshot!("downloader_tool_urls", tool);
}

#[test]
fn test_downloader_tool_file() {
    let path = PathBuf::from("/tmp/file.zip");
    let tool = DownloaderTool::new().file(path);
    insta::assert_debug_snapshot!("downloader_tool_file", tool);
}

#[test]
fn test_downloader_tool_force() {
    let tool = DownloaderTool::new().force(true);
    insta::assert_debug_snapshot!("downloader_tool_force_true", tool);

    let tool = DownloaderTool::new().force(false);
    insta::assert_debug_snapshot!("downloader_tool_force_false", tool);
}

#[test]
fn test_downloader_tool_operations() {
    let operations: Vec<_> = [
        ("download_op", DownloaderTool::new().download_op().operation),
        ("clean_op", DownloaderTool::new().clean_op().operation),
        ("default", DownloaderOperation::default()),
    ]
    .into_iter()
    .map(|(name, op)| (name, format!("{op:?}")))
    .collect();
    insta::assert_yaml_snapshot!("downloader_operations", operations);
}

#[test]
fn test_tool_name() {
    let tool = DownloaderTool::new();
    insta::assert_snapshot!("downloader_tool_name", tool.name());
}

#[tokio::test]
async fn test_clean_operation_dry_run() {
    let tool = DownloaderTool::new().file("/tmp/test.zip").clean_op();
    let ctx = create_test_ctx(true);

    let result = tool.run(&ctx).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_download_operation_no_urls() {
    let tool = DownloaderTool::new().file("/tmp/test.zip");
    let ctx = create_test_ctx(false);

    let result = tool.run(&ctx).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("no URLs provided"));
}

#[tokio::test]
async fn test_download_operation_no_output_file() {
    let tool = DownloaderTool::new().url("https://example.com/file.zip");
    let ctx = create_test_ctx(false);

    let result = tool.run(&ctx).await;
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("no output file specified")
    );
}

#[tokio::test]
async fn test_download_operation_dry_run() {
    let tool = DownloaderTool::new()
        .url("https://example.com/file.zip")
        .file("/tmp/test.zip");
    let ctx = create_test_ctx(true);

    let result = tool.run(&ctx).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_download_operation_cancelled() {
    let token = CancellationToken::new();
    token.cancel();
    let ctx = ToolContext::new(Arc::new(crate::config::Config::default()), token, false);

    let tool = DownloaderTool::new()
        .url("https://example.com/file.zip")
        .file("/tmp/test.zip");

    let result = tool.run(&ctx).await;
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("download cancelled")
    );
}

#[tokio::test]
async fn test_clean_operation_no_file() {
    let tool = DownloaderTool::new()
        .file("/tmp/nonexistent_file_12345.zip")
        .clean_op();
    let ctx = create_test_ctx(false);

    let result = tool.run(&ctx).await;
    assert!(result.is_ok());
}

#[test]
fn test_downloader_tool_builder_chain() {
    let tool = DownloaderTool::new()
        .url("https://example.com/file.zip")
        .file("/tmp/file.zip")
        .force(true)
        .download_op();

    insta::assert_debug_snapshot!("downloader_tool_builder_chain", tool);
}
