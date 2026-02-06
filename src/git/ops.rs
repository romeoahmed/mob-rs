// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Git operations for repository management.
//!
//! ```text
//! set_remotes_for_all  configure user/remotes per repo
//! set_ignore_ts        mark .ts files assume-unchanged
//! list_branches        report current branch per repo
//! fetch_refspec        fetch specific refspec
//! remote_branch_exists check remote branch
//! ```
//!
//! All operations iterate over repos discovered in `paths.build`.

use crate::config::Config;
use crate::error::Result;
use anyhow::Context;
use std::path::PathBuf;
use tracing::{debug, info, trace};

use super::cmd::{
    add_remote, rename_remote, set_assume_unchanged, set_config, set_remote_push_url,
    unset_assume_unchanged,
};
use super::discovery::get_repos;
use super::query::current_branch;
use super::{cmd::git_command, discovery::find_ts_files};

/// Set git remotes for all repositories.
///
/// Configures user.name, user.email, renames origin to upstream,
/// and creates new origin pointing to user's fork.
///
/// # Arguments
///
/// * `config` - Configuration containing paths.build and remote settings
/// * `username` - Git username for user.name config and fork URL
/// * `email` - Git email for user.email config
/// * `key_file` - Optional `PuTTY` key file for SSH authentication
/// * `dry_run` - If true, only log what would be done without executing
///
/// # Errors
///
/// Returns an error if:
/// - paths.build is not configured
/// - Repository discovery fails
/// - Any git operation fails
pub fn set_remotes_for_all(
    config: &Config,
    username: &str,
    email: &str,
    key_file: Option<&std::path::Path>,
    dry_run: bool,
) -> Result<()> {
    let repos = get_repos(config)?;
    let git_url_prefix = &config.task.git_url_prefix;
    let remote_org = if config.task.remote_setup.remote_org.is_empty() {
        username
    } else {
        &config.task.remote_setup.remote_org
    };

    for repo in &repos {
        let repo_name = repo
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        info!(repo = %repo_name, "setting remotes");

        if dry_run {
            debug!(repo = %repo_name, username, email, "would set user config");
            debug!(repo = %repo_name, "would rename origin to upstream");
            debug!(repo = %repo_name, remote_org, "would add new origin");
            if config.task.remote_setup.remote_no_push_upstream {
                debug!(repo = %repo_name, "would disable upstream push");
            }
            if config.task.remote_setup.remote_push_default_origin {
                debug!(repo = %repo_name, "would set default push remote to origin");
            }
            continue;
        }

        // Set user config
        set_config(repo, "user.name", username)
            .with_context(|| format!("failed to set user.name in {repo_name}"))?;
        set_config(repo, "user.email", email)
            .with_context(|| format!("failed to set user.email in {repo_name}"))?;

        // Rename origin to upstream (ignore error if upstream exists or origin doesn't exist)
        let _ = rename_remote(repo, "origin", "upstream");

        // Disable pushing to upstream
        if config.task.remote_setup.remote_no_push_upstream {
            let _ = set_remote_push_url(repo, "upstream", "nopushurl");
        }

        // Add new origin
        let origin_url = format!(
            "{}:{}/{}.git",
            git_url_prefix.trim_end_matches('/'),
            remote_org,
            repo_name
        );
        add_remote(repo, "origin", &origin_url, key_file)
            .with_context(|| format!("failed to add origin remote for {repo_name}"))?;

        // Set default push remote
        if config.task.remote_setup.remote_push_default_origin {
            set_config(repo, "remote.pushDefault", "origin")
                .with_context(|| format!("failed to set pushDefault in {repo_name}"))?;
        }
    }

    Ok(())
}

/// Add a remote to specified repositories (or all if repos is empty).
///
/// # Arguments
///
/// * `config` - Configuration containing paths.build and `git_url_prefix`
/// * `name` - Name for the new remote (e.g., "upstream", "fork")
/// * `username` - Username/organization for the remote URL
/// * `key_file` - Optional `PuTTY` key file for SSH authentication
/// * `repos` - List of repository names to operate on (empty = all repositories)
/// * `dry_run` - If true, only log what would be done without executing
///
/// # Errors
///
/// Returns an error if:
/// - paths.build is not configured
/// - Repository discovery fails
/// - Any git add remote operation fails
pub fn add_remote_to_repos(
    config: &Config,
    name: &str,
    username: &str,
    key_file: Option<&std::path::Path>,
    repos: &[String],
    dry_run: bool,
) -> Result<()> {
    let all_repos = get_repos(config)?;
    let git_url_prefix = &config.task.git_url_prefix;

    for repo in &all_repos {
        let repo_name = repo
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        // Filter if specific repos requested
        if !repos.is_empty() && !repos.iter().any(|r| r == repo_name) {
            continue;
        }

        let url = format!(
            "{}:{}/{}.git",
            git_url_prefix.trim_end_matches('/'),
            username,
            repo_name
        );

        info!(repo = %repo_name, remote = name, "adding remote");

        if dry_run {
            debug!(repo = %repo_name, remote = name, url, "would add remote");
            continue;
        }

        add_remote(repo, name, &url, key_file)
            .with_context(|| format!("failed to add remote {name} for {repo_name}"))?;
    }

    Ok(())
}

/// Set or unset assume-unchanged flag on .ts files in all repositories.
///
/// Finds all .ts files in src/ directories and marks them as assume-unchanged
/// (or removes the flag if disabling). This is useful for generated TypeScript
/// files that should be ignored by git status.
///
/// # Arguments
///
/// * `config` - Configuration containing paths.build
/// * `enable` - If true, set assume-unchanged; if false, unset it
/// * `dry_run` - If true, only log what would be done without executing
///
/// # Returns
///
/// The total number of .ts files processed across all repositories.
///
/// # Errors
///
/// Returns an error if:
/// - paths.build is not configured
/// - Repository discovery fails
/// - Any git update-index operation fails
pub fn set_ignore_ts(config: &Config, enable: bool, dry_run: bool) -> Result<usize> {
    let repos = get_repos(config)?;
    let mut count = 0;

    for repo in &repos {
        let src_dir = repo.join("src");
        if !src_dir.exists() {
            continue;
        }

        let repo_name = repo
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        // Find .ts files recursively
        let ts_files = find_ts_files(&src_dir)
            .with_context(|| format!("failed to find .ts files in {repo_name}"))?;

        for ts_file in ts_files {
            trace!(
                repo = %repo_name,
                file = %ts_file.display(),
                enable,
                "setting assume-unchanged"
            );

            if !dry_run {
                if enable {
                    set_assume_unchanged(repo, &ts_file).with_context(|| {
                        format!(
                            "failed to set assume-unchanged for {} in {}",
                            ts_file.display(),
                            repo_name
                        )
                    })?;
                } else {
                    unset_assume_unchanged(repo, &ts_file).with_context(|| {
                        format!(
                            "failed to unset assume-unchanged for {} in {}",
                            ts_file.display(),
                            repo_name
                        )
                    })?;
                }
            }
            count += 1;
        }

        if count > 0 {
            let action = if enable { "ignored" } else { "unignored" };
            info!(
                repo = %repo_name,
                count,
                "{} .ts files",
                action
            );
        }
    }

    Ok(count)
}

/// List current branch for each repository.
///
/// Returns a vector of (`repository_path`, `branch_name`) tuples, sorted by path.
/// If a repository has a detached HEAD, the branch name will be "(detached)".
///
/// # Arguments
///
/// * `config` - Configuration containing paths.build
///
/// # Returns
///
/// A sorted vector of (`PathBuf`, String) tuples representing repo paths and their branches.
///
/// # Errors
///
/// Returns an error if:
/// - paths.build is not configured
/// - Repository discovery fails
/// - Querying the current branch fails for any repository
pub fn list_branches(config: &Config) -> Result<Vec<(PathBuf, String)>> {
    let repos = get_repos(config)?;
    let mut branches = Vec::new();

    for repo in repos {
        let branch = current_branch(&repo)
            .with_context(|| {
                format!(
                    "failed to get current branch for {}",
                    repo.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                )
            })?
            .unwrap_or_else(|| "(detached)".to_string());
        branches.push((repo, branch));
    }

    Ok(branches)
}

/// Fetch a specific refspec from a remote URL.
///
/// # Errors
///
/// Returns an error if the fetch operation fails.
pub fn fetch_refspec(repo_path: &std::path::Path, remote_url: &str, refspec: &str) -> Result<()> {
    git_command(&["fetch", "--quiet", remote_url, refspec], repo_path)?;
    Ok(())
}

/// Check if a remote branch exists.
///
/// Uses `git ls-remote --heads <url> <branch>` to check if a branch exists on a remote
/// without cloning the repository. This is useful for determining fallback branches
/// before attempting to clone.
///
/// # Arguments
///
/// * `url` - The remote repository URL
/// * `branch` - The branch name to check (without refs/heads/ prefix)
///
/// # Returns
///
/// - `Ok(true)` if the branch exists on the remote
/// - `Ok(false)` if the branch does not exist or the remote is inaccessible
/// - `Err` only for system-level errors (not network errors)
///
/// # Errors
///
/// Returns an error only for system-level errors like "git not found". Network errors
/// and "not found" results are returned as `Ok(false)`.
///
/// # Note
///
/// Network errors (timeouts, authentication failures, etc.) are treated as "branch doesn't exist"
/// (returns `Ok(false)`). Only system-level errors like "git not found" will return an error.
pub fn remote_branch_exists(url: &str, branch: &str) -> Result<bool> {
    let output = git_command(
        &["ls-remote", "--heads", url, &format!("refs/heads/{branch}")],
        std::path::Path::new("."),
    );

    output.map_or_else(|_| Ok(false), |stdout| Ok(!stdout.is_empty()))
}
