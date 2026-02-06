// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::PathBuf;
use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use super::ModOrganizerTask;
use crate::config::Config;
use crate::task::{TaskContext, Taskable};

fn test_config() -> Arc<Config> {
    let mut config = Config::default();
    config.paths.prefix = Some(PathBuf::from("/test/prefix"));
    config.paths.build = Some(PathBuf::from("/test/build"));
    config.paths.install = Some(PathBuf::from("/test/install"));
    config.paths.qt_install = Some(PathBuf::from("/test/qt"));
    Arc::new(config)
}

fn test_ctx(config: Arc<Config>) -> TaskContext {
    TaskContext::new(config, CancellationToken::new()).with_dry_run(true)
}

#[test]
fn test_modorganizer_task_naming() {
    let cases: Vec<_> = ["archive", "modorganizer-uibase"]
        .into_iter()
        .map(|input| {
            let task = ModOrganizerTask::new(input);
            (input, task.name().to_string(), task.repo_name().to_string())
        })
        .collect();

    insta::assert_yaml_snapshot!("modorganizer_task_naming", cases);
}

#[test]
fn test_git_url() {
    let config = test_config();
    let task = ModOrganizerTask::new("archive");
    let url = task.git_url(&config);
    insta::assert_snapshot!("modorganizer_git_url", url);
}

#[test]
fn test_source_path() {
    let config = test_config();
    let task = ModOrganizerTask::new("archive");
    let path = task.source_path(&config).unwrap();
    insta::assert_debug_snapshot!("modorganizer_source_path", path);
}

#[test]
fn test_cmake_prefix_path() {
    let config = test_config();
    let _task = ModOrganizerTask::new("archive");
    let prefix_path = ModOrganizerTask::cmake_prefix_path(&config).unwrap();

    // Should contain Qt install and lib/cmake paths
    assert!(prefix_path.contains("/test/qt"));
    assert!(prefix_path.contains("lib"));
    assert!(prefix_path.contains("cmake"));
}

#[test]
fn test_enabled() {
    let config = test_config();
    let ctx = test_ctx(config);
    let task = ModOrganizerTask::new("archive");

    assert!(task.enabled(&ctx));
}

#[test]
fn test_enabled_disabled_task() {
    let mut config = Config::default();
    config.paths.prefix = Some(PathBuf::from("/test"));
    config.paths.build = Some(PathBuf::from("/test/build"));
    config.paths.install = Some(PathBuf::from("/test/install"));

    // Disable specific task
    let task_override = crate::config::merge::TaskConfigOverride {
        enabled: Some(false),
        ..Default::default()
    };
    config.tasks.insert("archive".to_string(), task_override);

    let ctx = test_ctx(Arc::new(config));
    let task = ModOrganizerTask::new("archive");

    assert!(!task.enabled(&ctx));
}
