// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

use super::ExplorerPPTask;
use crate::config::Config;
use std::path::PathBuf;

#[test]
fn test_explorerpp_task_info() {
    let task = ExplorerPPTask::new();
    let config = Config::default();
    insta::assert_yaml_snapshot!(
        "explorerpp_task_info",
        serde_json::json!({
            "name": task.name(),
            "version": ExplorerPPTask::version(&config),
        })
    );
}

#[test]
fn test_download_url_format() {
    let config = Config::default();
    let _task = ExplorerPPTask::new();

    let url = ExplorerPPTask::download_url(&config);
    assert!(url.contains("explorerplusplus.com"));
    assert!(url.contains(&config.versions.explorerpp));
    assert!(url.to_ascii_lowercase().ends_with(".zip"));
}

#[test]
fn test_cache_file_path() {
    let mut config = Config::default();
    config.paths.cache = Some(PathBuf::from("/test/cache"));

    let _task = ExplorerPPTask::new();

    let path = ExplorerPPTask::cache_file(&config).unwrap();
    insta::assert_debug_snapshot!("explorerpp_cache_file_path", path);
}

#[test]
fn test_source_path() {
    let mut config = Config::default();
    config.paths.build = Some(PathBuf::from("/test/build"));

    let _task = ExplorerPPTask::new();

    let path = ExplorerPPTask::source_path(&config).unwrap();
    insta::assert_debug_snapshot!("explorerpp_source_path", path);
}

#[test]
fn test_install_path() {
    let mut config = Config::default();
    config.paths.install_bin = Some(PathBuf::from("/test/install/bin"));

    let _task = ExplorerPPTask::new();

    let path = ExplorerPPTask::install_path(&config).unwrap();
    insta::assert_debug_snapshot!("explorerpp_install_path", path);
}
