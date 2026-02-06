// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! CLI arguments for the `pr` command.
//!
//! # Architecture
//!
//! ```text
//! mob pr <operation> <task/pr-number>
//! find   → list affected repos (dry-run preview)
//! pull   → fetch PR branch and checkout
//! revert → checkout master for affected repos
//!
//! USAGE:
//! $ mob pr find modorganizer/123
//! $ mob pr pull modorganizer/123 --github-token $TOKEN
//! $ mob pr revert modorganizer/123
//! ```

use clap::{Args, ValueEnum};

/// Arguments for the `pr` command.
#[derive(Debug, Clone, Args)]
pub struct PrArgs {
    /// GitHub API key.
    #[arg(long = "github-token", value_name = "TOKEN", env = "GITHUB_TOKEN")]
    pub github_token: Option<String>,

    #[arg(value_name = "OP")]
    pub operation: PrOperation,

    #[arg(value_name = "PR")]
    pub pr: String,
}

/// PR operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum PrOperation {
    /// List affected repos.
    Find,
    /// Fetch and checkout PR branch.
    Pull,
    /// Checkout master branch.
    Revert,
}
