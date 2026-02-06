// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

use super::walk::{WalkOptions, find_files, parallel_walk, parallel_walk_with_callback};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tempfile::TempDir;

fn temp_dir() -> TempDir {
    tempfile::tempdir().expect("failed to create temp dir")
}

#[test]
fn test_parallel_walk() {
    let temp = temp_dir();

    // Create some files and directories
    std::fs::create_dir(temp.path().join("subdir")).unwrap();
    std::fs::write(temp.path().join("file1.txt"), "").unwrap();
    std::fs::write(temp.path().join("subdir/file2.txt"), "").unwrap();

    let result = parallel_walk(temp.path(), &WalkOptions::default()).unwrap();

    insta::assert_yaml_snapshot!(
        "parallel_walk",
        serde_json::json!({
            "files_count": result.files().len(),
            "directories_count": result.directories().len(),
            "error_count": result.error_count(),
        })
    );
}

#[test]
fn test_parallel_walk_skip_dirs() {
    let temp = temp_dir();

    // Create node_modules directory
    std::fs::create_dir(temp.path().join("node_modules")).unwrap();
    std::fs::write(temp.path().join("node_modules/package.json"), "").unwrap();
    std::fs::write(temp.path().join("index.js"), "").unwrap();

    let result = parallel_walk(temp.path(), &WalkOptions::for_build_tool()).unwrap();

    // Only index.js should be found, not package.json in node_modules
    let file_names: Vec<String> = result
        .files()
        .iter()
        .filter_map(|p| p.file_name().and_then(|n| n.to_str()).map(String::from))
        .collect();

    insta::assert_yaml_snapshot!(
        "parallel_walk_skip_dirs",
        serde_json::json!({
            "files_count": result.files().len(),
            "found_files": file_names,
        })
    );
}

#[test]
fn test_find_files() {
    let temp = temp_dir();

    // Create test files
    std::fs::write(temp.path().join("file1.rs"), "").unwrap();
    std::fs::write(temp.path().join("file2.txt"), "").unwrap();
    std::fs::create_dir(temp.path().join("subdir")).unwrap();
    std::fs::write(temp.path().join("subdir/file3.rs"), "").unwrap();

    let rust_files = find_files(temp.path(), "**/*.rs").unwrap();

    insta::assert_yaml_snapshot!(
        "find_files",
        serde_json::json!({
            "count": rust_files.len(),
            "all_rs_extension": rust_files.iter().all(|p| p.extension().unwrap() == "rs"),
        })
    );
}

#[test]
fn test_parallel_walk_with_callback() {
    let temp = temp_dir();

    std::fs::write(temp.path().join("file1.txt"), "hello").unwrap();
    std::fs::write(temp.path().join("file2.txt"), "world").unwrap();

    let count = Arc::new(AtomicUsize::new(0));
    let count_clone = Arc::clone(&count);

    let processed = parallel_walk_with_callback(temp.path(), &WalkOptions::default(), move |_| {
        count_clone.fetch_add(1, Ordering::Relaxed);
    })
    .unwrap();

    insta::assert_yaml_snapshot!(
        "parallel_walk_with_callback",
        serde_json::json!({
            "processed": processed,
            "callback_count": count.load(Ordering::Relaxed),
        })
    );
}
