// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

use super::LicensesTask;
use crate::config::Config;
use std::path::PathBuf;

#[test]
fn test_licenses_task_info() {
    let task = LicensesTask::new();
    insta::assert_yaml_snapshot!(
        "licenses_task_info",
        serde_json::json!({
            "name": task.name(),
        })
    );
}

#[test]
fn test_source_path() {
    let mut config = Config::default();
    config.paths.licenses = Some(PathBuf::from("/test/licenses"));

    let _task = LicensesTask::new();

    let path = LicensesTask::source_path(&config).unwrap();
    insta::assert_debug_snapshot!("licenses_source_path", path);
}

#[test]
fn test_install_path() {
    let mut config = Config::default();
    config.paths.install_licenses = Some(PathBuf::from("/test/install/bin/licenses"));

    let _task = LicensesTask::new();

    let path = LicensesTask::install_path(&config).unwrap();
    insta::assert_debug_snapshot!("licenses_install_path", path);
}

#[test]
fn test_source_path_not_configured() {
    let config = Config::default();
    let _task = LicensesTask::new();

    // Should return error when not configured
    assert!(LicensesTask::source_path(&config).is_err());
}
