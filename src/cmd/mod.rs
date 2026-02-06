// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Command implementations.
//!
//! ```text
//! CLI args --> cmd::run_* handlers
//!   build, config, git, list, pr, release, tx
//! ```

pub mod build;
pub mod config;
pub mod git;
pub mod list;
pub mod pr;
pub mod release;
pub mod tx;
