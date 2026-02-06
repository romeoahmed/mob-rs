// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::PathBuf;
use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use super::UsvfsTask;
use crate::config::Config;
use crate::core::env::types::Arch;
use crate::task::{TaskContext, Taskable};

fn test_config() -> Arc<Config> {
    let mut config = Config::default();
    config.paths.prefix = Some(PathBuf::from("/test/prefix"));
    config.paths.build = Some(PathBuf::from("/test/build"));
    config.paths.install = Some(PathBuf::from("/test/install"));
    Arc::new(config)
}

fn test_ctx(config: Arc<Config>) -> TaskContext {
    crate::task::TaskContext::new(config, CancellationToken::new()).with_dry_run(true)
}

#[test]
fn test_usvfs_task_info() {
    let task = UsvfsTask::new();
    let config = test_config();
    insta::assert_yaml_snapshot!(
        "usvfs_task_info",
        serde_json::json!({
            "name": task.name(),
            "version": UsvfsTask::version(&config),
        })
    );
}

#[test]
fn test_git_url() {
    let config = test_config();
    let _task = UsvfsTask::new();
    let url = UsvfsTask::git_url(&config);
    insta::assert_snapshot!("usvfs_git_url", url);
}

#[test]
fn test_source_path() {
    let config = test_config();
    let _task = UsvfsTask::new();
    let path = UsvfsTask::source_path(&config).unwrap();
    insta::assert_debug_snapshot!("usvfs_source_path", path);
}

#[test]
fn test_build_dirs() {
    let config = test_config();
    let _task = UsvfsTask::new();

    let x64_dir = UsvfsTask::build_dir(&config, Arch::X64).unwrap();
    insta::assert_debug_snapshot!("usvfs_build_dir_x64", x64_dir);

    let x86_dir = UsvfsTask::build_dir(&config, Arch::X86).unwrap();
    insta::assert_debug_snapshot!("usvfs_build_dir_x86", x86_dir);
}

#[test]
fn test_solution_paths() {
    let config = test_config();
    let _task = UsvfsTask::new();

    let x64_sln = UsvfsTask::solution_path(&config, Arch::X64).unwrap();
    insta::assert_debug_snapshot!("usvfs_solution_path_x64", x64_sln);

    let x86_sln = UsvfsTask::solution_path(&config, Arch::X86).unwrap();
    insta::assert_debug_snapshot!("usvfs_solution_path_x86", x86_sln);
}

#[test]
fn test_cmake_presets() {
    let _task = UsvfsTask::new();
    insta::assert_snapshot!("usvfs_cmake_preset_x64", UsvfsTask::cmake_preset(Arch::X64));
    insta::assert_snapshot!("usvfs_cmake_preset_x86", UsvfsTask::cmake_preset(Arch::X86));
}

#[test]
fn test_enabled() {
    let config = test_config();
    let ctx = test_ctx(config);
    let task = UsvfsTask::new();
    assert!(task.enabled(&ctx));
}

#[test]
fn test_version_configured() {
    let mut config = Config::default();
    config.paths.prefix = Some(PathBuf::from("/test"));
    config.paths.build = Some(PathBuf::from("/test/build"));
    config.versions.usvfs = "v0.5.0".to_string();

    let _task = UsvfsTask::new();
    let version = UsvfsTask::version(&config);
    insta::assert_snapshot!("usvfs_version_configured", version);
}
