// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Git command arguments.
//!
//! # Subcommands
//!
//! ```text
//! git set-remotes -u USER -e EMAIL
//!   → origin→upstream, new origin→fork
//! git ignore-ts [on|off]
//!   → mark/unmark .ts assume-unchanged
//! git add-remote NAME URL
//!   → add remote to all repos
//! git branches
//!   → list repos not on master
//! ```

use clap::{Args, Subcommand, ValueEnum};
use std::path::PathBuf;

/// Arguments for the `git` command.
#[derive(Debug, Clone, Args)]
pub struct GitArgs {
    /// Git subcommand.
    #[command(subcommand)]
    pub subcommand: GitSubcommand,
}

/// Git subcommands.
#[derive(Debug, Clone, Subcommand)]
pub enum GitSubcommand {
    /// Renames 'origin' to 'upstream' and creates a new 'origin' with the given info.
    #[command(name = "set-remotes")]
    SetRemotes(SetRemotesArgs),

    /// Adds a new remote with the given information.
    #[command(name = "add-remote")]
    AddRemote(AddRemoteArgs),

    /// Toggles the --assume-changed status of all .ts files in all repos.
    #[command(name = "ignore-ts")]
    IgnoreTs(IgnoreTsArgs),

    /// Lists all git repos that are not on master.
    Branches(BranchesArgs),
}

/// Arguments for set-remotes subcommand.
#[derive(Debug, Clone, Args)]
pub struct SetRemotesArgs {
    /// Git username.
    #[arg(short = 'u', long, required = true)]
    pub username: String,

    /// Git email.
    #[arg(short = 'e', long, required = true)]
    pub email: String,

    /// Path to `PuTTY` key.
    #[arg(short = 'k', long = "key", value_name = "PATH")]
    pub key: Option<PathBuf>,

    /// Disables pushing to 'upstream' by changing the push url to 'nopushurl'.
    #[arg(short = 's', long = "no-push")]
    pub no_push: bool,

    /// Sets the new 'origin' remote as the default push target.
    #[arg(short = 'p', long = "push-origin")]
    pub push_default: bool,

    /// Only use this repo instead of going through all of them.
    #[arg(value_name = "PATH")]
    pub path: Option<PathBuf>,
}

/// Arguments for add-remote subcommand.
#[derive(Debug, Clone, Args)]
pub struct AddRemoteArgs {
    /// Name of new remote.
    #[arg(short = 'n', long, required = true)]
    pub name: String,

    /// Git username.
    #[arg(short = 'u', long, required = true)]
    pub username: String,

    /// Path to `PuTTY` key.
    #[arg(short = 'k', long = "key", value_name = "PATH")]
    pub key: Option<PathBuf>,

    /// Sets this new remote as the default push target.
    #[arg(short = 'p', long = "push-origin")]
    pub push_default: bool,

    /// Only use this repo instead of going through all of them.
    #[arg(value_name = "PATH")]
    pub path: Option<PathBuf>,
}

/// Arguments for ignore-ts subcommand.
#[derive(Debug, Clone, Args)]
pub struct IgnoreTsArgs {
    /// Whether to turn on or off ignore-ts.
    #[arg(value_enum)]
    pub state: IgnoreTsState,
}

/// Ignore-ts state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum IgnoreTsState {
    /// Enable ignoring .ts files.
    On,
    /// Disable ignoring .ts files.
    Off,
}

/// Arguments for branches subcommand.
#[derive(Debug, Clone, Default, Args)]
pub struct BranchesArgs {
    /// Shows all branches, including those on master.
    #[arg(short = 'a', long)]
    pub all: bool,
}
