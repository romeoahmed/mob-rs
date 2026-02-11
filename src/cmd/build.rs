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
use crate::task::tasks::installer::InstallerTask;
use crate::task::tasks::licenses::LicensesTask;
use crate::task::tasks::modorganizer::ModOrganizerTask;
use crate::task::tasks::stylesheets::StylesheetsTask;
use crate::task::tasks::translations::TranslationsTask;
use crate::task::tasks::usvfs::UsvfsTask;
use crate::task::{CleanFlags, ParallelTasks, Task};

/// Built-in task names that have dedicated task types (not `ModOrganizerTask`).
pub(crate) const BUILTIN_TASKS: &[&str] = &[
    "usvfs",
    "modorganizer",
    "stylesheets",
    "explorerpp",
    "licenses",
    "translations",
    "installer",
];

/// Default `ModOrganizer` sub-projects, matching C++ mob's `add_tasks()` in `main.cpp`.
///
/// These are the `modorganizer-*` repos that C++ mob hardcodes. Without this list,
/// mob-rs only knows about `BUILTIN_TASKS` (7 items) and whatever `mob.toml` defines,
/// leaving ~25 sub-projects unregistered and therefore never cloned or built.
///
/// Each entry is `(canonical_name, &[alternate_names])`. Alternate names come from
/// transifex slugs which sometimes differ from project names.
const DEFAULT_MO_PROJECTS: &[(&str, &[&str])] = &[
    // Sequential group 1 (parallel with usvfs)
    ("cmake_common", &[]),
    // Sequential group 2 (single)
    ("modorganizer-uibase", &[]),
    // Sequential group 3 (parallel)
    ("modorganizer-archive", &[]),
    ("modorganizer-lootcli", &[]),
    ("modorganizer-esptk", &[]),
    ("modorganizer-bsatk", &[]),
    ("modorganizer-nxmhandler", &[]),
    ("modorganizer-helper", &[]),
    ("modorganizer-game_bethesda", &[]),
    // Sequential group 4 (parallel)
    ("modorganizer-bsapacker", &["bsa_packer"]),
    ("modorganizer-tool_inieditor", &["inieditor"]),
    ("modorganizer-tool_inibakery", &["inibakery"]),
    ("modorganizer-preview_bsa", &[]),
    ("modorganizer-preview_base", &[]),
    ("modorganizer-diagnose_basic", &[]),
    ("modorganizer-check_fnis", &[]),
    ("modorganizer-installer_bain", &[]),
    ("modorganizer-installer_manual", &[]),
    ("modorganizer-installer_bundle", &[]),
    ("modorganizer-installer_quick", &[]),
    ("modorganizer-installer_fomod", &[]),
    ("modorganizer-installer_fomod_csharp", &[]),
    ("modorganizer-installer_omod", &[]),
    ("modorganizer-installer_wizard", &[]),
    ("modorganizer-bsa_extractor", &[]),
    ("modorganizer-plugin_python", &[]),
    // Sequential group 5 (parallel, alongside stylesheets/licenses/explorerpp)
    ("modorganizer-tool_configurator", &["pycfg"]),
    ("modorganizer-fnistool", &[]),
    ("modorganizer-basic_games", &[]),
    (
        "modorganizer-script_extender_plugin_checker",
        &["scriptextenderpluginchecker"],
    ),
    ("modorganizer-form43_checker", &["form43checker"]),
    ("modorganizer-preview_dds", &["ddspreview"]),
    // "modorganizer" itself has alternate name "organizer" — handled in BUILTIN_TASKS
];

/// Main handler for build command.
///
/// When no specific tasks are given, builds all tasks in the order defined by
/// C++ mob's `add_tasks()` — sequential groups with parallel sub-tasks.
/// When specific tasks are given, resolves and runs them sequentially.
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
    register_default_projects(&mut registry);
    registry.register_all(BUILTIN_TASKS.iter().map(std::string::ToString::to_string));
    // Register alternate name for modorganizer
    registry.register("organizer".to_string());

    if args.tasks.is_empty() {
        // No specific tasks requested — use the full ordered task tree
        // matching C++ mob's add_tasks() sequential groups.
        add_default_task_tree(&mut manager);
    } else {
        // Specific tasks requested — resolve and run sequentially
        let resolved_names = resolve_task_names(&registry, args)?;
        for name in resolved_names {
            manager.add(task_from_name(name));
        }
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
        // Skip alias names (e.g., "super", "plugins") — they are config override
        // scopes, not actual buildable tasks. In C++ mob, `[super:task]` applies
        // overrides to all tasks in the `super` alias group.
        if config.aliases.contains_key(name) {
            continue;
        }

        // Skip built-in task names — they have their own task types and should
        // not be registered as modorganizer-* repos.
        if BUILTIN_TASKS.contains(&name.as_str()) {
            continue;
        }

        registry.register(name.clone());
        if let Some(short) = name.strip_prefix("modorganizer-") {
            registry.register(short.to_string());
        } else if name != "modorganizer" {
            registry.register(format!("modorganizer-{name}"));
        }
    }
}

/// Registers all default `ModOrganizer` sub-projects and their alternate names.
pub(crate) fn register_default_projects(registry: &mut TaskRegistry) {
    for &(canonical, alternates) in DEFAULT_MO_PROJECTS {
        registry.register(canonical.to_string());

        // Also register the short name (without `modorganizer-` prefix)
        if let Some(short) = canonical.strip_prefix("modorganizer-") {
            registry.register(short.to_string());
        }

        // Register alternate names (transifex slugs, etc.)
        for &alt in alternates {
            registry.register(alt.to_string());
        }
    }
}

/// Builds the default task tree matching C++ mob's `add_tasks()` ordering.
///
/// Tasks are organized into sequential groups. Within each group, tasks run
/// in parallel. Groups execute sequentially because later groups depend on
/// earlier ones (e.g., `uibase` must be built before `archive`).
///
/// ```text
/// Group 1 (parallel): usvfs, cmake_common
/// Group 2 (single):   modorganizer-uibase
/// Group 3 (parallel): archive, lootcli, esptk, bsatk, nxmhandler, helper, game_bethesda
/// Group 4 (parallel): bsapacker, tool_inieditor, ... plugin_python
/// Group 5 (parallel): stylesheets, licenses, explorerpp, pycfg, ... modorganizer
/// Group 6 (single):   translations
/// Group 7 (single):   installer
/// ```
fn add_default_task_tree(manager: &mut TaskManager) {
    // Group 1: usvfs + cmake_common (parallel)
    manager.add(Task::Parallel(
        ParallelTasks::new(vec![])
            .with_task(Task::Usvfs(UsvfsTask::new()))
            .with_task(mo("cmake_common")),
    ));

    // Group 2: modorganizer-uibase (must complete before group 3)
    manager.add(mo("modorganizer-uibase"));

    // Group 3: parallel batch
    manager.add(Task::Parallel(
        ParallelTasks::new(vec![])
            .with_task(mo("modorganizer-archive"))
            .with_task(mo("modorganizer-lootcli"))
            .with_task(mo("modorganizer-esptk"))
            .with_task(mo("modorganizer-bsatk"))
            .with_task(mo("modorganizer-nxmhandler"))
            .with_task(mo("modorganizer-helper"))
            .with_task(mo("modorganizer-game_bethesda")),
    ));

    // Group 4: parallel batch
    manager.add(Task::Parallel(
        ParallelTasks::new(vec![])
            .with_task(mo("modorganizer-bsapacker"))
            .with_task(mo("modorganizer-tool_inieditor"))
            .with_task(mo("modorganizer-tool_inibakery"))
            .with_task(mo("modorganizer-preview_bsa"))
            .with_task(mo("modorganizer-preview_base"))
            .with_task(mo("modorganizer-diagnose_basic"))
            .with_task(mo("modorganizer-check_fnis"))
            .with_task(mo("modorganizer-installer_bain"))
            .with_task(mo("modorganizer-installer_manual"))
            .with_task(mo("modorganizer-installer_bundle"))
            .with_task(mo("modorganizer-installer_quick"))
            .with_task(mo("modorganizer-installer_fomod"))
            .with_task(mo("modorganizer-installer_fomod_csharp"))
            .with_task(mo("modorganizer-installer_omod"))
            .with_task(mo("modorganizer-installer_wizard"))
            .with_task(mo("modorganizer-bsa_extractor"))
            .with_task(mo("modorganizer-plugin_python")),
    ));

    // Group 5: parallel batch (including stylesheets, licenses, explorerpp)
    manager.add(Task::Parallel(
        ParallelTasks::new(vec![])
            .with_task(Task::Stylesheets(StylesheetsTask::new()))
            .with_task(Task::Licenses(LicensesTask::new()))
            .with_task(Task::ExplorerPP(ExplorerPPTask::new()))
            .with_task(mo("modorganizer-tool_configurator"))
            .with_task(mo("modorganizer-fnistool"))
            .with_task(mo("modorganizer-basic_games"))
            .with_task(mo("modorganizer-script_extender_plugin_checker"))
            .with_task(mo("modorganizer-form43_checker"))
            .with_task(mo("modorganizer-preview_dds"))
            .with_task(Task::ModOrganizer(ModOrganizerTask::new(
                "modorganizer".to_string(),
            ))),
    ));

    // Group 6: translations (single)
    manager.add(Task::Translations(TranslationsTask::new()));

    // Group 7: installer (single)
    manager.add(Task::Installer(InstallerTask::new()));
}

/// Shorthand for creating a `ModOrganizerTask` wrapped in `Task`.
fn mo(name: &str) -> Task {
    Task::ModOrganizer(ModOrganizerTask::new(name.to_string()))
}

fn resolve_task_names(registry: &TaskRegistry, args: &BuildArgs) -> Result<Vec<String>> {
    let resolved_names: Vec<String> = match registry.resolve(&args.tasks) {
        Ok(names) => names,
        Err(e) => {
            eprintln!("Failed to resolve tasks: {e}");
            return Err(e);
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
        "translations" => Task::Translations(TranslationsTask::new()),
        "installer" => Task::Installer(InstallerTask::new()),
        // Resolve alternate names to their canonical modorganizer-* form
        "organizer" => mo("modorganizer"),
        "bsa_packer" => mo("modorganizer-bsapacker"),
        "inieditor" => mo("modorganizer-tool_inieditor"),
        "inibakery" => mo("modorganizer-tool_inibakery"),
        "pycfg" => mo("modorganizer-tool_configurator"),
        "scriptextenderpluginchecker" => mo("modorganizer-script_extender_plugin_checker"),
        "form43checker" => mo("modorganizer-form43_checker"),
        "ddspreview" => mo("modorganizer-preview_dds"),
        _ => Task::ModOrganizer(ModOrganizerTask::new(name)),
    }
}
