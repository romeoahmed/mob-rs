// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

use super::InstallerTask;
use crate::config::Config;

#[test]
fn test_installer_task_naming() {
    let cases: Vec<_> = [InstallerTask::new(), InstallerTask::default()]
        .into_iter()
        .map(|task| task.name().to_string())
        .collect();
    insta::assert_yaml_snapshot!("installer_task_naming", cases);
}

#[test]
fn test_git_url() {
    let _task = InstallerTask::new();
    let config = Config::default();
    let url = InstallerTask::git_url(&config);
    insta::assert_snapshot!("installer_git_url", url);
}
