// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

use super::{CleanFlags, ParallelTasks, Phase, Task, TaskContext, Taskable};
use crate::config::Config;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

fn test_config() -> Arc<Config> {
    Arc::new(Config::default())
}

#[test]
fn test_phase_all() {
    let phases = Phase::all();
    insta::assert_debug_snapshot!("test_phase_all", phases);
}

#[test]
fn test_phase_name() {
    let names: Vec<_> = Phase::all().iter().map(super::Phase::name).collect();
    insta::assert_debug_snapshot!("test_phase_names", names);
}

#[test]
fn test_clean_flags() {
    let flags = CleanFlags::REDOWNLOAD | CleanFlags::REBUILD;
    assert!(flags.contains(CleanFlags::REDOWNLOAD));
    assert!(flags.contains(CleanFlags::REBUILD));
    assert!(!flags.contains(CleanFlags::REEXTRACT));
    assert!(!flags.contains(CleanFlags::RECONFIGURE));
}

#[test]
fn test_clean_flags_default() {
    let flags = CleanFlags::default();
    assert!(flags.is_empty());
}

#[test]
fn test_task_context_creation() {
    let config = test_config();
    let token = CancellationToken::new();
    let ctx = TaskContext::new(config, token);

    insta::assert_yaml_snapshot!(
        "task_context_creation",
        serde_json::json!({
            "dry_run": ctx.is_dry_run(),
            "do_clean": ctx.phases().do_clean(),
            "do_fetch": ctx.phases().do_fetch(),
            "do_build": ctx.phases().do_build(),
            "clean_flags_empty": ctx.clean_flags().is_empty(),
        })
    );
}

#[test]
fn test_task_context_builder() {
    let config = test_config();
    let token = CancellationToken::new();
    let ctx = TaskContext::new(config, token)
        .with_dry_run(true)
        .with_do_clean(true)
        .with_clean_flags(CleanFlags::REBUILD);

    insta::assert_debug_snapshot!(
        "test_task_context_builder",
        &(
            ctx.is_dry_run(),
            ctx.phases().do_clean(),
            ctx.phases().do_fetch(),
            ctx.phases().do_build(),
            ctx.clean_flags(),
        )
    );
}

#[test]
fn test_task_context_cancellation() {
    let config = test_config();
    let token = CancellationToken::new();
    let ctx = TaskContext::new(config, token.clone());

    assert!(!ctx.is_cancelled());
    token.cancel();
    assert!(ctx.is_cancelled());
}

#[test]
fn test_parallel_tasks() {
    let parallel = ParallelTasks::new(vec![]);
    let task = Task::Parallel(parallel.clone());
    insta::assert_yaml_snapshot!(
        "parallel_tasks",
        serde_json::json!({
            "children_empty": parallel.children().is_empty(),
            "task_name": task.name(),
        })
    );
}
