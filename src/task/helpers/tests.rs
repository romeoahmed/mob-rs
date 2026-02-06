// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

use super::{TaskContext, check_source_safe_to_delete, copy_file_if_newer, ensure_dir};
use std::sync::Arc;
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

fn temp_dir() -> TempDir {
    tempfile::tempdir().expect("failed to create temp dir")
}

fn test_context() -> (TempDir, TaskContext) {
    let temp = temp_dir();
    let config = crate::config::Config::default();
    let ctx = TaskContext::new(Arc::new(config), CancellationToken::new());
    (temp, ctx)
}

#[test]
fn check_safe_to_delete_non_git_dir() {
    let temp = temp_dir();
    let result = check_source_safe_to_delete(temp.path(), false);
    assert!(result.is_ok());
}

#[test]
fn check_safe_to_delete_with_ignore_flag() {
    let temp = temp_dir();
    // Even if we can't determine state, ignore_uncommitted=true should pass
    let result = check_source_safe_to_delete(temp.path(), true);
    assert!(result.is_ok());
}

#[tokio::test]
async fn ensure_dir_creates_directory() {
    let (temp, ctx) = test_context();
    let new_dir = temp.path().join("subdir");

    ensure_dir(&ctx, &new_dir, "test directory")
        .await
        .expect("should create dir");

    assert!(new_dir.exists());
}

#[tokio::test]
async fn ensure_dir_dry_run_does_not_create() {
    let (temp, ctx) = test_context();
    let ctx = ctx.with_dry_run(true);
    let new_dir = temp.path().join("subdir");

    ensure_dir(&ctx, &new_dir, "test directory")
        .await
        .expect("should succeed");

    assert!(!new_dir.exists());
}

#[tokio::test]
async fn copy_file_if_newer_copies_when_missing() {
    let (temp, ctx) = test_context();
    let src = temp.path().join("src.txt");
    let dst = temp.path().join("dst.txt");

    tokio::fs::write(&src, "content").await.expect("write src");

    copy_file_if_newer(&ctx, &src, &dst, "test file")
        .await
        .expect("should copy");

    assert!(dst.exists());
    let content = tokio::fs::read_to_string(&dst).await.expect("read dst");
    assert_eq!(content, "content");
}

#[tokio::test]
async fn copy_file_if_newer_dry_run() {
    let (temp, ctx) = test_context();
    let ctx = ctx.with_dry_run(true);
    let src = temp.path().join("src.txt");
    let dst = temp.path().join("dst.txt");

    tokio::fs::write(&src, "content").await.expect("write src");

    copy_file_if_newer(&ctx, &src, &dst, "test file")
        .await
        .expect("should succeed");

    assert!(!dst.exists());
}
