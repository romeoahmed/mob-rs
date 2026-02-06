// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Config-related commands for mob-rs.

use crate::cli::cmake::{CmakeConfigArgs, CmakeVariable};
use crate::config::Config;
use crate::error::Result;
use anyhow::anyhow;

/// Display current configuration options.
pub fn run_options_command(config: &Config) {
    for line in config.format_options() {
        println!("{line}");
    }
}

/// Display loaded configuration files.
pub fn run_inis_command(config_files: &[String]) {
    if config_files.is_empty() {
        println!("No configuration files loaded");
    } else {
        for line in config_files {
            println!("{line}");
        }
    }
}

/// Run the cmake-config command.
///
/// # Errors
///
/// Returns an error if `paths.install` is not configured when querying `InstallPrefix`.
pub fn run_cmake_config_command(args: &CmakeConfigArgs, config: &Config) -> Result<()> {
    match args.variable {
        CmakeVariable::PrefixPath => {
            println!("{}", config.paths.cmake_prefix_path());
            Ok(())
        }
        CmakeVariable::InstallPrefix => config.paths.cmake_install_prefix().map_or_else(
            || Err(anyhow!("paths.install not configured")),
            |path| {
                println!("{path}");
                Ok(())
            },
        ),
    }
}
