// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Entry point.
//!
//! ```text
//! cli::parse() --> Logging --> Command Dispatch
//!   Build | Release | Git | Pr | Tx | Config | List
//! ```

use std::process::ExitCode;

use mob_rs::cli::global::GlobalOptions;
use mob_rs::cli::{self, Command};
use mob_rs::cmd::build::run_build_command;
use mob_rs::cmd::config::{run_cmake_config_command, run_inis_command, run_options_command};
use mob_rs::cmd::git::run_git_command;
use mob_rs::cmd::list::run_list_command;
use mob_rs::cmd::pr::run_pr_command;
use mob_rs::cmd::release::run_release_command;
use mob_rs::cmd::tx::run_tx_command;
use mob_rs::config::Config;
use mob_rs::config::loader::ConfigLoader;
use mob_rs::logging::init_logging;
use mob_rs::logging::{LogConfig, LogLevel};

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[tokio::main]
async fn main() -> ExitCode {
    let cli = cli::parse();

    let log_config = build_log_config(&cli.global);
    let _log_guard = match init_logging(&log_config) {
        Ok(guard) => guard,
        Err(e) => {
            eprintln!("Failed to initialize logging: {e}");
            return ExitCode::FAILURE;
        }
    };

    dispatch_command(&cli).await
}

fn build_log_config(global: &GlobalOptions) -> LogConfig {
    let console_level = global
        .log_level
        .and_then(LogLevel::from_u8)
        .unwrap_or(LogLevel::INFO);

    let file_level = global
        .file_log_level
        .and_then(LogLevel::from_u8)
        .unwrap_or(console_level);

    LogConfig::builder()
        .with_console_level(console_level)
        .with_file_level(file_level)
        .maybe_with_log_file(global.log_file.as_ref().map(|p| p.display().to_string()))
        .build()
}

async fn dispatch_command(cli: &cli::Cli) -> ExitCode {
    let result = match &cli.command {
        Some(Command::Version) => {
            handle_version_command();
            Ok(())
        }
        Some(Command::Options) => {
            load_config(&cli.global).map(|config| run_options_command(&config))
        }
        Some(Command::Inis) => {
            let loader = build_config_loader(&cli.global);
            run_inis_command(&loader.format_loaded_files());
            Ok(())
        }
        Some(Command::Build(args)) => match load_config(&cli.global) {
            Ok(config) => run_build_command(args, &config, cli.global.dry).await,
            Err(e) => Err(e),
        },
        Some(Command::List(args)) => {
            load_config(&cli.global).and_then(|config| run_list_command(args, &config))
        }
        Some(Command::Release(args)) => match load_config(&cli.global) {
            Ok(config) => run_release_command(args, &config, cli.global.dry).await,
            Err(e) => Err(e),
        },
        Some(Command::Git(args)) => load_config(&cli.global)
            .and_then(|config| run_git_command(args, &config, cli.global.dry)),
        Some(Command::Pr(args)) => match load_config(&cli.global) {
            Ok(config) => run_pr_command(args, &config).await,
            Err(e) => Err(e),
        },
        Some(Command::Tx(args)) => match load_config(&cli.global) {
            Ok(config) => run_tx_command(args, &config, cli.global.dry).await,
            Err(e) => Err(e),
        },
        Some(Command::CmakeConfig(args)) => {
            load_config(&cli.global).and_then(|config| run_cmake_config_command(args, &config))
        }
        None => {
            eprintln!("No command specified. Use --help for usage information.");
            Err(anyhow::anyhow!("No command specified"))
        }
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn handle_version_command() {
    println!("{}", env!("CARGO_PKG_VERSION"));
}

fn build_config_loader(global: &GlobalOptions) -> ConfigLoader {
    let mut loader = ConfigLoader::new();
    for ini_path in &global.inis {
        loader = loader.add_toml_file(ini_path);
    }
    loader.add_toml_file_optional("mob.toml")
}

fn load_config(global: &GlobalOptions) -> mob_rs::error::Result<Config> {
    let loader = build_config_loader(global);
    loader.build().map_err(|e| {
        eprintln!("Failed to load config: {e}");
        e
    })
}
