// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

use super::{PackOperation, PackerTool};
use crate::task::tools::{Tool, ToolContext};
use std::path::PathBuf;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

#[test]
fn test_packer_tool_creation() {
    let tool = PackerTool::new();
    assert_eq!(tool.name(), "packer");
    assert_eq!(tool.operation, PackOperation::PackDir);
}

#[test]
fn test_packer_tool_builder_archive() {
    let tool = PackerTool::new().archive("output.7z");
    assert_eq!(tool.archive_required().unwrap().to_str(), Some("output.7z"));
}

#[test]
fn test_packer_tool_builder_base_dir() {
    let tool = PackerTool::new().base_dir("source");
    assert_eq!(tool.base_dir_required().unwrap().to_str(), Some("source"));
}

#[test]
fn test_packer_tool_builder_exclude_patterns() {
    let patterns = vec!["*.tmp", "*.log"];
    let tool = PackerTool::new().exclude_patterns(patterns);
    insta::assert_debug_snapshot!("packer_exclude_patterns", tool.exclude_patterns);
}

#[test]
fn test_packer_tool_builder_files() {
    let files = vec![PathBuf::from("file1.txt"), PathBuf::from("file2.txt")];
    let tool = PackerTool::new().files(files);
    assert_eq!(tool.files.len(), 2);
}

#[test]
fn test_packer_tool_operation_pack_dir() {
    let tool = PackerTool::new().pack_dir_op();
    assert_eq!(tool.operation, PackOperation::PackDir);
}

#[test]
fn test_packer_tool_operation_pack_files() {
    let tool = PackerTool::new().pack_files_op();
    assert_eq!(tool.operation, PackOperation::PackFiles);
}

#[test]
fn test_packer_tool_missing_archive() {
    let tool = PackerTool::new().base_dir("source");
    assert!(tool.archive_required().is_err());
}

#[test]
fn test_packer_tool_missing_base_dir() {
    let tool = PackerTool::new().archive("output.7z");
    assert!(tool.base_dir_required().is_err());
}

#[test]
fn test_packer_tool_default() {
    let tool = PackerTool::default();
    assert_eq!(tool.operation, PackOperation::PackDir);
    assert!(tool.archive.is_none());
    assert!(tool.base_dir.is_none());
}

#[test]
fn test_pack_operation_default() {
    let op = PackOperation::default();
    assert_eq!(op, PackOperation::PackDir);
}

#[tokio::test]
async fn test_packer_tool_dry_run_pack_dir() {
    let config = Arc::new(crate::config::Config::default());
    let token = CancellationToken::new();
    let ctx = ToolContext::new(config, token, true);

    let tool = PackerTool::new()
        .archive("output.7z")
        .base_dir("source")
        .pack_dir_op();

    // Should succeed in dry-run mode without actually creating archive
    assert!(tool.run(&ctx).await.is_ok());
}

#[tokio::test]
async fn test_packer_tool_dry_run_pack_files() {
    let config = Arc::new(crate::config::Config::default());
    let token = CancellationToken::new();
    let ctx = ToolContext::new(config, token, true);

    let tool = PackerTool::new()
        .archive("output.7z")
        .base_dir("source")
        .files(vec![PathBuf::from("file1.txt")])
        .pack_files_op();

    // Should succeed in dry-run mode without actually creating archive
    assert!(tool.run(&ctx).await.is_ok());
}
