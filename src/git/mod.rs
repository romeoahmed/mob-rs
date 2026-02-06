// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Git operations module.
//!
//! ```text
//!        Public API
//!   query.rs  cmd.rs  ops.rs
//!        \      |      /
//!         v     v     v
//!      ,------------------,
//!      | backend (traits) |
//!      '--+----------+----'
//!         |          |
//!         v          v
//!    GitQuery    GitMutation
//!   (gix, read)  (CLI, write)
//!         |          |
//!         v          v
//!    GixBackend  ShellBackend
//!    .is_repo    .clone/pull
//!    .branch     .checkout
//!    .tracked    .add_remote
//!    .uncommit   .putty_keys
//!    .stashed
//! ```
//!
//! **`GixBackend`** — pure Rust, no subprocess, read-only.
//! **`ShellBackend`** — git CLI for SSH/PuTTY, submodules, writes.

pub mod backend;
pub mod cmd;
pub mod discovery;
pub mod ops;
pub mod query;

#[cfg(test)]
mod tests;
