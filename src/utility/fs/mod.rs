// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Filesystem utilities with parallel traversal and async copy.
//!
//! ```text
//! walk:  parallel_walk()  ignore::WalkParallel (multi-core)
//!        find_files()     glob pattern matching
//!        WalkOptions      max_depth, hidden, gitignore
//! copy:  copy_files_async()        tokio::fs parallel copy
//!        copy_dir_contents_async() recursive directory copy
//! ```

pub mod copy;
pub mod walk;

#[cfg(test)]
mod tests;
