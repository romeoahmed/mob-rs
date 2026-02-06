// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Library root.
//!
//! # Crate Architecture
//!
//! ```text
//!                        main.rs
//!                           |
//!                +----------+----------+
//!                v                     v
//!             cli (clap)          cmd (handlers)
//!                |           build / release / pr
//!                +----------+----------+
//!                           v
//!              ,---------------------------,
//!              |          config           |
//!              |   TOML, layered settings  |
//!              '--+-----------+--------+---'
//!                 |           |        |
//!                 v           v        v
//!              task         git       net
//!            manager     gix/CLI    HTTP/DL
//!               |
//!          +----+----+
//!          v         v
//!       tasks      tools
//!     (phases)   cmake/git/..
//!
//!   +-----------------------------------------+
//!   |  core   process, job, env, VS discovery |
//!   +-----------------------------------------+
//!   |  foundation   error, logging, utility   |
//!   +-----------------------------------------+
//! ```

pub mod cli;
pub mod cmd;
pub mod config;
pub mod core;
pub mod error;
pub mod git;
pub mod logging;
pub mod net;
pub mod task;
pub mod utility;
