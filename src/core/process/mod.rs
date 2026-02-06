// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Async process spawning and management.
//!
//! ```text
//! ProcessBuilder::new("cmake")
//!   .args() .cwd() .env() .capture_stdout()
//!   .run() / .run_with_cancellation()
//!       --> tokio::process::Command
//!           stream stdout/stderr
//!           Windows: CTRL_BREAK + Job Object
//!       --> ProcessOutput { exit_code, stdout, stderr }
//! ```

pub mod builder;
mod io;
mod runner;
#[cfg(test)]
mod tests;
#[cfg(windows)]
mod windows;
