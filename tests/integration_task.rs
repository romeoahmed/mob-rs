// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Integration tests for the task system.
//!
//! Tests `TaskManager` orchestration, cancellation, and concurrency behavior.
//! Unit tests for `Phase`, `CleanFlags`, etc. are in `src/task/tests.rs`.

use std::sync::Arc;

use mob_rs::config::Config;
use mob_rs::task::manager::TaskManager;
use mob_rs::task::{CleanFlags, ParallelTasks, Phase, Task, TaskContext, Taskable};
use tokio_util::sync::CancellationToken;

/// Creates a test config for task tests.
fn test_config() -> Arc<Config> {
    Arc::new(Config::default())
}

#[tokio::test]
async fn test_task_manager_empty_run() {
    let config = test_config();
    let manager = TaskManager::new(config);

    // Running empty task list should succeed
    let result = manager.run_all().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_task_manager_with_parallel_tasks() {
    let config = test_config();
    let mut manager = TaskManager::new(config);

    // Add empty parallel tasks
    let parallel = ParallelTasks::new(vec![
        Task::Parallel(ParallelTasks::new(vec![])),
        Task::Parallel(ParallelTasks::new(vec![])),
    ]);
    manager.add(Task::Parallel(parallel));

    let result = manager.run_all().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_task_manager_cancellation_before_start() {
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

    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("interrupted"),
        "Expected 'interrupted' in error message, got: {err_msg}"
    );
}

#[tokio::test]
async fn test_task_context_creation() {
    let config = test_config();
    let token = CancellationToken::new();
    let ctx = TaskContext::new(config, token.clone())
        .with_dry_run(true)
        .with_do_clean(true)
        .with_clean_flags(CleanFlags::REBUILD);

    assert!(ctx.is_dry_run());
    assert!(ctx.phases().do_clean());
    assert!(ctx.phases().do_fetch());
    assert!(ctx.phases().do_build());
    assert!(ctx.clean_flags().contains(CleanFlags::REBUILD));
    assert!(!ctx.is_cancelled());

    token.cancel();
    assert!(ctx.is_cancelled());
}

#[tokio::test]
async fn test_task_context_tool_context_conversion() {
    let config = test_config();
    let token = CancellationToken::new();
    let ctx = TaskContext::new(config, token).with_dry_run(true);

    let tool_ctx = ctx.tool_context();
    assert!(tool_ctx.is_dry_run());
}

#[tokio::test]
async fn test_task_manager_builder_pattern() {
    let config = test_config();
    let manager = TaskManager::new(config)
        .with_dry_run(true)
        .with_do_clean(true)
        .with_do_fetch(false)
        .with_do_build(true)
        .with_clean_flags(CleanFlags::REDOWNLOAD | CleanFlags::REBUILD);

    assert_eq!(manager.task_count(), 0);
    assert!(!manager.is_cancelled());
}

#[tokio::test]
async fn test_task_manager_concurrency_semaphore() {
    let config = test_config();
    let manager = TaskManager::with_concurrency(config, 8);

    let semaphore = manager.concurrency_semaphore();
    assert_eq!(semaphore.available_permits(), 8);
}

#[tokio::test]
async fn test_parallel_tasks_builder() {
    let parallel = ParallelTasks::new(vec![])
        .with_task(Task::Parallel(ParallelTasks::new(vec![])))
        .with_task(Task::Parallel(ParallelTasks::new(vec![])));

    assert_eq!(parallel.children().len(), 2);
}

#[tokio::test]
async fn test_task_enabled_check() {
    let config = test_config();
    let token = CancellationToken::new();
    let ctx = TaskContext::new(config, token);

    // Empty parallel task is disabled
    let empty = Task::Parallel(ParallelTasks::new(vec![]));
    assert!(!empty.enabled(&ctx));

    // Non-empty parallel task is enabled
    let non_empty = Task::Parallel(ParallelTasks::new(vec![Task::Parallel(
        ParallelTasks::new(vec![]),
    )]));
    assert!(non_empty.enabled(&ctx));
}

#[tokio::test]
async fn test_task_name() {
    let task = Task::Parallel(ParallelTasks::new(vec![]));
    assert_eq!(task.name(), "parallel");
}

#[tokio::test]
async fn test_nested_parallel_tasks() {
    let config = test_config();
    let mut manager = TaskManager::new(config);

    // Create deeply nested parallel structure
    let inner_most = ParallelTasks::new(vec![Task::Parallel(ParallelTasks::new(vec![]))]);
    let middle = ParallelTasks::new(vec![
        Task::Parallel(inner_most),
        Task::Parallel(ParallelTasks::new(vec![])),
    ]);
    let outer = ParallelTasks::new(vec![Task::Parallel(middle)]);

    manager.add(Task::Parallel(outer));

    let result = manager.run_all().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_multiple_sequential_tasks() {
    let config = test_config();
    let mut manager = TaskManager::new(config);

    // Add multiple tasks that run sequentially
    manager.add(Task::Parallel(ParallelTasks::new(vec![])));
    manager.add(Task::Parallel(ParallelTasks::new(vec![])));
    manager.add(Task::Parallel(ParallelTasks::new(vec![])));

    assert_eq!(manager.task_count(), 3);

    let result = manager.run_all().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_phase_names() {
    let phase_names = [
        Phase::Clean.name(),
        Phase::Fetch.name(),
        Phase::BuildAndInstall.name(),
    ];
    insta::assert_debug_snapshot!(phase_names);
}

#[tokio::test]
async fn test_task_manager_cancel_token_sharing() {
    let config = test_config();
    let manager = TaskManager::new(config);

    let token = manager.cancel_token();
    assert!(!token.is_cancelled());

    manager.interrupt_all();

    // The cloned token should also see the cancellation
    assert!(token.is_cancelled());
}
