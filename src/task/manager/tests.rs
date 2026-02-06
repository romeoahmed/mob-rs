// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::Arc;

use super::TaskManager;
use crate::config::Config;
use crate::task::{CleanFlags, ParallelTasks, Task};

fn test_config() -> Arc<Config> {
    Arc::new(Config::default())
}

#[test]
fn test_task_manager_new() {
    let config = test_config();
    let manager = TaskManager::new(config);

    insta::assert_yaml_snapshot!(
        "task_manager_new",
        serde_json::json!({
            "task_count": manager.task_count(),
            "is_cancelled": manager.is_cancelled(),
            "dry_run": manager.is_dry_run(),
            "clean_flags_empty": manager.clean_flags().is_empty(),
        })
    );
}

#[test]
fn test_task_manager_add() {
    let config = test_config();
    let mut manager = TaskManager::new(config);

    let counts: Vec<_> = [0, 1, 2]
        .into_iter()
        .map(|_| {
            let count = manager.task_count();
            manager.add(Task::Parallel(ParallelTasks::new(vec![])));
            count
        })
        .collect();
    insta::assert_yaml_snapshot!("task_manager_add_counts", counts);
}

#[test]
fn test_task_manager_interrupt() {
    let config = test_config();
    let manager = TaskManager::new(config);

    assert!(!manager.is_cancelled());
    manager.interrupt_all();
    assert!(manager.is_cancelled());
}

#[test]
fn test_task_manager_builder() {
    let config = test_config();
    let manager = TaskManager::new(config)
        .with_dry_run(true)
        .with_do_clean(true)
        .with_clean_flags(CleanFlags::REBUILD);

    insta::assert_debug_snapshot!(
        "test_task_manager_builder",
        &(
            manager.is_dry_run(),
            manager.phases().do_clean(),
            manager.clean_flags(),
            manager.phases().do_fetch(),
            manager.phases().do_build(),
        )
    );
}

#[test]
fn test_task_manager_with_concurrency() {
    let config = test_config();
    let manager = TaskManager::with_concurrency(config, 4);

    insta::assert_yaml_snapshot!(
        "task_manager_concurrency",
        serde_json::json!({
            "available_permits": manager.concurrency_semaphore().available_permits(),
        })
    );
}

#[tokio::test]
async fn test_task_manager_run_cancelled() {
    let config = test_config();
    let mut manager = TaskManager::new(config);

    // Add a task
    manager.add(Task::Parallel(ParallelTasks::new(vec![Task::Parallel(
        ParallelTasks::new(vec![]),
    )])));

    // Cancel before running
    manager.interrupt_all();

    // Should fail due to cancellation
    let result = manager.run_all().await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("interrupted"));
}
