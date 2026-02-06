// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Git repository discovery.
//!
//! ```text
//! paths.build/
//!   usvfs/              (included if git repo)
//!   modorganizer_super/
//!     modorganizer/     (included)
//!     uibase/           (included)
//!     .git/             (skipped, hidden)
//!     ...
//! ```
//!
//! Returns sorted list of repo paths for deterministic ordering.

use crate::config::Config;
use crate::error::Result;
use crate::utility::fs::walk::find_files;
use anyhow::Context;
use std::path::{Path, PathBuf};

use super::query::is_git_repo;

/// Discover all git repositories in the build directory.
///
/// Returns paths to:
/// - usvfs source directory (if it exists and is a git repo)
/// - All subdirectories of `modorganizer_super` (excluding hidden dirs starting with '.')
///
/// # Errors
///
/// Returns an error if `paths.build` is not configured in the config.
pub fn get_repos(config: &Config) -> Result<Vec<PathBuf>> {
    let build_path = config
        .paths
        .build
        .as_ref()
        .context("paths.build not configured")?;

    let mut repos = Vec::new();

    // Add usvfs if it exists and is a git repo
    let usvfs_path = build_path.join("usvfs");
    if usvfs_path.exists() && is_git_repo(&usvfs_path) {
        repos.push(usvfs_path);
    }

    // Add modorganizer repos from modorganizer_super
    let super_path = build_path.join("modorganizer_super");
    if super_path.exists() {
        for entry in std::fs::read_dir(&super_path)
            .with_context(|| format!("failed to read {}", super_path.display()))?
        {
            let entry = entry
                .with_context(|| format!("failed to read entry in {}", super_path.display()))?;
            let path = entry.path();

            // Skip non-directories
            if !path.is_dir() {
                continue;
            }

            // Skip hidden directories (starting with '.')
            if let Some(name) = path.file_name().and_then(|n| n.to_str())
                && name.starts_with('.')
            {
                continue;
            }

            repos.push(path);
        }
    }

    // Sort for determinism (BTreeMap ordering)
    repos.sort();
    Ok(repos)
}

/// Recursively find all .ts files in a directory.
///
/// Uses parallel directory traversal via `ignore::WalkParallel`.
///
/// # Errors
///
/// Returns an error if the directory traversal or glob matching fails.
pub fn find_ts_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut ts_files = find_files(dir, "**/*.ts")
        .with_context(|| format!("failed to find .ts files in {}", dir.display()))?;

    // Sort for determinism
    ts_files.sort();
    Ok(ts_files)
}
