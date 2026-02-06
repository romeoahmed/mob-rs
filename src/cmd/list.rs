// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! List command implementation for mob-rs.

use crate::cli::build::ListArgs;
use crate::cmd::build::{BUILTIN_TASKS, register_config_tasks};
use crate::config::Config;
use crate::error::Result;
use crate::task::registry::TaskRegistry;

/// Main handler for list command.
///
/// # Errors
///
/// Returns an error if task resolution fails.
pub fn run_list_command(args: &ListArgs, config: &Config) -> Result<()> {
    if args.aliases {
        if config.aliases.is_empty() {
            println!("No aliases defined");
        } else {
            for (name, targets) in &config.aliases {
                println!("{} = {}", name, targets.join(", "));
            }
        }
        return Ok(());
    }

    let mut registry = TaskRegistry::new(config.aliases.clone());
    register_config_tasks(&mut registry, config);
    registry.register_all(BUILTIN_TASKS.iter().map(std::string::ToString::to_string));

    let tasks_to_list = if args.all && !args.tasks.is_empty() {
        match registry.resolve(&args.tasks) {
            Ok(names) => names,
            Err(e) => {
                eprintln!("Failed to resolve task patterns: {e}");
                return Err(e);
            }
        }
    } else {
        registry.all_tasks().iter().cloned().collect()
    };

    if tasks_to_list.is_empty() {
        println!("No tasks found");
    } else {
        for task in &tasks_to_list {
            println!("{task}");
        }
    }
    Ok(())
}
