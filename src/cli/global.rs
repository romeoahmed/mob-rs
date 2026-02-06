// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Global CLI options available for all commands.
//!
//! # Option Precedence
//!
//! ```text
//! --ini FILE        ← Additional config files (can repeat)
//! --dry             ← Simulate filesystem ops
//! --log-level N     ← Console verbosity (0-6)
//! --file-log-level  ← File verbosity (overrides --log-level)
//! --destination DIR ← paths.prefix override
//! --set KEY=VAL     ← Direct config override
//!
//! Precedence: CLI flags > --set > --ini > defaults
//! ```

use clap::Args;
use std::path::PathBuf;

/// Global options available for all commands.
#[derive(Debug, Clone, Default, Args)]
pub struct GlobalOptions {
    /// Path to additional INI/TOML configuration file(s).
    /// Can be specified multiple times.
    #[arg(short = 'i', long = "ini", value_name = "FILE", action = clap::ArgAction::Append)]
    pub inis: Vec<PathBuf>,

    /// Simulates filesystem operations.
    /// Note that many operations will fail and the build process will most
    /// probably not complete. This is mostly useful to get a dump of the options.
    #[arg(long)]
    pub dry: bool,

    /// Console log level (0=silent, 1=errors, 2=warnings, 3=info, 4=debug, 5=trace, 6=dump).
    #[arg(short = 'l', long = "log-level", value_name = "LEVEL", value_parser = clap::value_parser!(u8).range(0..=6)
    )]
    pub log_level: Option<u8>,

    /// File log level, overrides --log-level for the log file.
    #[arg(long = "file-log-level", value_name = "LEVEL", value_parser = clap::value_parser!(u8).range(0..=6)
    )]
    pub file_log_level: Option<u8>,

    /// Path to log file.
    #[arg(long = "log-file", value_name = "FILE")]
    pub log_file: Option<PathBuf>,

    /// Base output directory (will contain build/, install/, etc.).
    #[arg(short = 'd', long = "destination", value_name = "DIR")]
    pub prefix: Option<PathBuf>,

    /// Sets an option, such as 'versions/openssl=1.2' or 'task:section/key=value'.
    /// Can be specified multiple times.
    #[arg(short = 's', long = "set", value_name = "OPTION", action = clap::ArgAction::Append)]
    pub options: Vec<String>,

    /// Disables auto loading of INI files, only uses --ini.
    /// The first --ini must be the master INI file.
    #[arg(long = "no-default-inis")]
    pub no_default_inis: bool,
}

impl GlobalOptions {
    /// Converts command-line options to configuration overrides.
    ///
    /// This is equivalent to C++ mob's `convert_cl_to_conf()`.
    #[must_use]
    pub fn to_config_overrides(&self) -> Vec<String> {
        let mut overrides = self.options.clone();

        if let Some(level) = self.log_level {
            overrides.push(format!("global/output_log_level={level}"));
        }

        // file_log_level falls back to log_level if not specified
        if let Some(level) = self.file_log_level.or(self.log_level) {
            overrides.push(format!("global/file_log_level={level}"));
        }

        if let Some(ref path) = self.log_file {
            overrides.push(format!("global/log_file={}", path.display()));
        }

        if self.dry {
            overrides.push("global/dry=true".to_string());
        }

        if let Some(ref prefix) = self.prefix {
            overrides.push(format!("paths/prefix={}", prefix.display()));
        }

        overrides
    }
}
