// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Core modules for process and environment management.
//!
//! ```text
//!              core
//!               |
//!     +---------+---------+
//!     |    |         |    |
//!     v    v         v    v
//!    env  vs     process  job
//!     |    |         |     |
//!   Env  vswhere  Builder JobObject
//!   Arch Install  Output  KILL_ON_CLOSE
//!   vcvars      (Windows only)
//! ```

pub mod env;
pub mod process;

#[cfg(windows)]
pub mod vs;

#[cfg(windows)]
pub mod job;
