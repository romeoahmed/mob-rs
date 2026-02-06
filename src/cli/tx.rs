// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Transifex command arguments.
//!
//! # Subcommands
//!
//! ```text
//! tx get -k API_KEY -t TEAM -p PROJECT
//!   → pull translations (min threshold, force)
//! tx build
//!   → compile .ts → .qm via lrelease
//! ```

use clap::{Args, Subcommand};
use std::path::PathBuf;

/// Arguments for the `tx` command.
#[derive(Debug, Clone, Args)]
pub struct TxArgs {
    /// Transifex subcommand.
    #[command(subcommand)]
    pub subcommand: TxSubcommand,
}

/// Transifex subcommands.
#[derive(Debug, Clone, Subcommand)]
pub enum TxSubcommand {
    /// Initializes a Transifex project and pulls all translation files.
    Get(TxGetArgs),

    /// Builds all .qm files from translation sources.
    Build(TxBuildArgs),
}

/// Arguments for tx get subcommand.
#[derive(Debug, Clone, Args)]
pub struct TxGetArgs {
    /// Transifex API key.
    #[arg(short = 'k', long = "key", value_name = "APIKEY", env = "TX_TOKEN")]
    pub key: Option<String>,

    /// Transifex team name.
    #[arg(short = 't', long = "team", value_name = "TEAM")]
    pub team: Option<String>,

    /// Transifex project name.
    #[arg(short = 'p', long = "project", value_name = "PROJECT")]
    pub project: Option<String>,

    /// Transifex project URL.
    #[arg(short = 'u', long = "url", value_name = "URL")]
    pub url: Option<String>,

    /// Minimum translation threshold to download (0-100).
    #[arg(short = 'm', long = "minimum", value_name = "PERCENT", value_parser = clap::value_parser!(u8).range(0..=100)
    )]
    pub minimum: Option<u8>,

    /// Don't check timestamps, re-download all translation files.
    #[arg(short = 'f', long)]
    pub force: bool,

    /// Path that will contain the .tx directory.
    #[arg(value_name = "PATH")]
    pub path: PathBuf,
}

/// Arguments for tx build subcommand.
#[derive(Debug, Clone, Args)]
pub struct TxBuildArgs {
    /// Path that contains the translation directories.
    #[arg(value_name = "SOURCE")]
    pub source: PathBuf,

    /// Path that will contain the .qm files.
    #[arg(value_name = "DESTINATION")]
    pub destination: PathBuf,
}
