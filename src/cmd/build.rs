// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Build command implementation for mob-rs.

use std::sync::Arc;

use crate::cli::build::BuildArgs;
use crate::config::Config;
use crate::error::Result;
use crate::task::manager::TaskManager;
use crate::task::registry::TaskRegistry;
use crate::task::tasks::explorerpp::ExplorerPPTask;
use crate::task::tasks::licenses::LicensesTask;
use crate::task::tasks::modorganizer::ModOrganizerTask;
use crate::task::tasks::stylesheets::StylesheetsTask;
use crate::task::tasks::usvfs::UsvfsTask;
use crate::task::{CleanFlags, Task};

/// Built-in task names that are always available.
pub(crate) const BUILTIN_TASKS: &[&str] = &[
    "usvfs",
    "modorganizer",
    "stylesheets",
    "explorerpp",
    "licenses",
    "translations",
    "installer",
];

/// Main handler for build command.
///
/// # Errors
///
/// Returns an error if configuration fails, task resolution fails, or the task
/// runner reports a build failure.
pub async fn run_build_command(args: &BuildArgs, config: &Config, dry_run: bool) -> Result<()> {
    let config = Arc::new(config.clone());

    let clean_flags = compute_clean_flags(args);
    let do_clean = args.clean_phase.clean_task || !clean_flags.is_empty();
    let do_fetch = !args.fetch_phase.no_fetch_task;
    let do_build = !args.build_phase.no_build_task;

    let mut manager = TaskManager::new(Arc::clone(&config))
        .with_dry_run(dry_run)
        .with_clean_flags(clean_flags)
        .with_do_clean(do_clean)
        .with_do_fetch(do_fetch)
        .with_do_build(do_build);

    let cancel_token = manager.cancel_token();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            tracing::warn!("Received Ctrl+C, interrupting tasks...");
            cancel_token.cancel();
        }
    });

    let mut registry = TaskRegistry::new(config.aliases.clone());
    register_config_tasks(&mut registry, &config);
    registry.register_all(BUILTIN_TASKS.iter().map(std::string::ToString::to_string));

    let resolved_names = resolve_task_names(&registry, args)?;
    for name in resolved_names {
        manager.add(task_from_name(name));
    }

    match manager.run_all().await {
        Ok(()) => {
            tracing::info!("Build completed successfully");
            Ok(())
        }
        Err(e) => {
            eprintln!("Build failed: {e}");
            Err(e)
        }
    }
}

fn compute_clean_flags(args: &BuildArgs) -> CleanFlags {
    let mut clean_flags = CleanFlags::empty();
    if args.clean_download.redownload || args.clean_full.new_build {
        clean_flags |= CleanFlags::REDOWNLOAD;
    }
    if args.clean_download.reextract || args.clean_full.new_build {
        clean_flags |= CleanFlags::REEXTRACT;
    }
    if args.clean_build.reconfigure || args.clean_full.new_build {
        clean_flags |= CleanFlags::RECONFIGURE;
    }
    if args.clean_build.rebuild || args.clean_full.new_build {
        clean_flags |= CleanFlags::REBUILD;
    }
    clean_flags
}

pub(crate) fn register_config_tasks(registry: &mut TaskRegistry, config: &Config) {
    for name in config.tasks.keys() {
        registry.register(name.clone());
        if let Some(short) = name.strip_prefix("modorganizer-") {
            registry.register(short.to_string());
        } else if name != "modorganizer" {
            registry.register(format!("modorganizer-{name}"));
        }
    }
}

fn resolve_task_names(registry: &TaskRegistry, args: &BuildArgs) -> Result<Vec<String>> {
    let resolved_names: Vec<String> = if args.tasks.is_empty() {
        registry.all_tasks().iter().cloned().collect()
    } else {
        match registry.resolve(&args.tasks) {
            Ok(names) => names,
            Err(e) => {
                eprintln!("Failed to resolve tasks: {e}");
                return Err(e);
            }
        }
    };

    if resolved_names.is_empty() {
        tracing::warn!(patterns = ?args.tasks, "No tasks resolved from patterns");
    } else {
        tracing::info!(tasks = ?resolved_names, "Resolved tasks to run");
    }

    Ok(resolved_names)
}

fn task_from_name(name: String) -> Task {
    match name.as_str() {
        "usvfs" => Task::Usvfs(UsvfsTask::new()),
        "stylesheets" | "ss" => Task::Stylesheets(StylesheetsTask::new()),
        "explorerpp" | "explorer++" => Task::ExplorerPP(ExplorerPPTask::new()),
        "licenses" => Task::Licenses(LicensesTask::new()),
        _ => Task::ModOrganizer(ModOrganizerTask::new(name)),
    }
}
