// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! CLI arguments for the `cmake-config` command.
//!
//! # Architecture
//!
//! ```text
//! mob cmake-config <variable>
//! prefix-path    → print CMAKE_PREFIX_PATH
//! install-prefix → print CMAKE_INSTALL_PREFIX
//! ```

use clap::{Args, Subcommand};

/// Arguments for the `cmake-config` command.
#[derive(Debug, Clone, Args)]
pub struct CmakeConfigArgs {
    #[command(subcommand)]
    pub variable: CmakeVariable,
}

/// `CMake` variables that can be queried.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Subcommand)]
pub enum CmakeVariable {
    #[command(name = "prefix-path")]
    PrefixPath,

    #[command(name = "install-prefix")]
    InstallPrefix,
}
