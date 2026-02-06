// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

use super::{RELEASES, StylesheetsTask};
use crate::config::Config;
use std::path::PathBuf;

#[test]
fn test_stylesheets_task_info() {
    let task = StylesheetsTask::new();
    insta::assert_yaml_snapshot!(
        "stylesheets_task_info",
        serde_json::json!({
            "name": task.name(),
            "releases_count": RELEASES.len(),
        })
    );
}

#[test]
fn test_download_url_format() {
    let config = Config::default();
    let _task = StylesheetsTask::new();
    let release = &RELEASES[0]; // paper-light-and-dark

    let url = StylesheetsTask::download_url(&config, release);
    assert!(url.contains("github.com"));
    assert!(url.contains("6788-00"));
    assert!(url.contains("paper-light-and-dark"));
    assert!(url.to_ascii_lowercase().ends_with(".7z"));
}

#[test]
fn test_cache_file_path() {
    let mut config = Config::default();
    config.paths.cache = Some(PathBuf::from("/test/cache"));

    let _task = StylesheetsTask::new();
    let release = &RELEASES[0];

    let path = StylesheetsTask::cache_file(&config, release).unwrap();
    insta::assert_debug_snapshot!("stylesheets_cache_file_path", path);
}

#[test]
fn test_build_path() {
    let mut config = Config::default();
    config.paths.build = Some(PathBuf::from("/test/build"));

    let _task = StylesheetsTask::new();
    let release = &RELEASES[0];

    let path = StylesheetsTask::build_path(&config, release).unwrap();
    insta::assert_debug_snapshot!("stylesheets_build_path", path);
}

#[test]
fn test_all_releases_have_valid_config() {
    let config = Config::default();
    let _task = StylesheetsTask::new();

    for release in RELEASES {
        let version = StylesheetsTask::get_version(&config, release);
        // Should get version from config, not "latest" fallback
        assert_ne!(
            version, "latest",
            "Missing default version for {}",
            release.version_key
        );
    }
}
