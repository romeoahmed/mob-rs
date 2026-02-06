// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Environment variable management.
//!
//! # Architecture
//!
//! ```text
//! Env (copy-on-write BTreeMap<String, String>)
//! Sources: current_env(), Env::vs(Arch), Env::empty()
//! Ops: set/get/prepend_path/append_path
//! ```
//!
//! - **Case-insensitive on Windows**
//! - **Copy-on-write**: Clones share data until modified
//! - **UTF-8 internal**: Encoding at I/O boundaries only

pub mod container;
pub mod types;
#[cfg(windows)]
pub mod vcvars;

#[cfg(test)]
mod tests;

/// Captures the current process environment.
#[must_use]
pub fn current_env() -> container::Env {
    let vars = std::env::vars().collect();
    container::Env::from_map(vars)
}
