// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! CLI module for mob-rs using clap derive.
//!
//! # Command Structure
//!
//! ```text
//! mob [global options] <command>
//! build [tasks...]
//! list
//! release {devbuild|official}
//! git {set-remotes|ignore-ts|add-remote|branches}
//! pr
//! cmake-config
//! tx
//! ```

pub mod build;
pub mod cmake;
pub mod git;
pub mod global;
pub mod pr;
pub mod release;

#[cfg(test)]
mod tests;
pub mod tx;

use crate::cli::build::{BuildArgs, ListArgs};
use crate::cli::cmake::CmakeConfigArgs;
use crate::cli::git::GitArgs;
use crate::cli::global::GlobalOptions;
use crate::cli::pr::PrArgs;
use crate::cli::release::ReleaseArgs;
use crate::cli::tx::TxArgs;
use clap::{Parser, Subcommand};

/// `ModOrganizer` Build Tool - Rust Port
///
/// A build automation tool for the `ModOrganizer2` project.
#[derive(Debug, Parser)]
#[command(
    name = "mob",
    author,
    version,
    about = "ModOrganizer Build Tool",
    long_about = "mob-rs Copyright (C) 2026 Romeo Ahmed\n\
                  This program comes with ABSOLUTELY NO WARRANTY\n\
                  This is free software, and you are welcome to redistribute it\n\
                  under certain conditions; see LICENSE for details.\n\n\
                  A build automation tool for the ModOrganizer2 project.\n\n\
                  Invoking `mob -d some/prefix build` builds everything. Do\n\
                  `mob build <task name>...` to build specific tasks. See\n\
                  `mob <command> --help` for more information about a command.",
    after_help = "INI FILES:\n\n\
                  By default, mob will look for a master INI `mob.toml` in the\n\
                  root directory (typically where mob.exe resides). Once mob has\n\
                  found the master INI, it will look for the same filename in the\n\
                  current directory, if different from the root. If found, both will\n\
                  be loaded, but the one in the current directory will override the\n\
                  other. Additional INIs can be specified with --ini, those will\n\
                  be loaded after the two mentioned above. Use --no-default-inis to\n\
                  disable auto detection and only use --ini."
)]
pub struct Cli {
    /// Global options shared by all commands
    #[command(flatten)]
    pub global: GlobalOptions,

    /// Command to execute
    #[command(subcommand)]
    pub command: Option<Command>,
}

/// Available commands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Shows the version.
    #[command(visible_alias = "-v")]
    Version,

    /// Lists all options and their values from the INIs.
    Options,

    /// Lists the INIs used by mob.
    Inis,

    /// Builds tasks.
    Build(BuildArgs),

    /// Lists available tasks.
    List(ListArgs),

    /// Creates a release.
    Release(ReleaseArgs),

    /// Manages the git repos.
    Git(GitArgs),

    /// Applies changes from PRs.
    Pr(PrArgs),

    /// Manages transifex translations.
    Tx(TxArgs),

    /// Print `CMake` configuration variables.
    #[command(name = "cmake-config")]
    CmakeConfig(CmakeConfigArgs),
}

/// Parses command-line arguments.
#[must_use]
pub fn parse() -> Cli {
    Cli::parse()
}

/// Parses command-line arguments from an iterator.
pub fn parse_from<I, T>(iter: I) -> Cli
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    Cli::parse_from(iter)
}

/// Tries to parse command-line arguments, returning an error on failure.
///
/// # Errors
///
/// Returns a `clap::Error` if the arguments are invalid or if help/version information
/// was requested.
pub fn try_parse() -> Result<Cli, clap::Error> {
    Cli::try_parse()
}
