// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Git command implementation for mob-rs.

use crate::cli::git::{GitArgs, GitSubcommand, IgnoreTsState};
use crate::config::Config;
use crate::error::Result;
use crate::git::ops::{add_remote_to_repos, list_branches, set_ignore_ts, set_remotes_for_all};

/// Main handler for git command.
///
/// # Errors
///
/// Returns an error if any git operation fails.
pub fn run_git_command(args: &GitArgs, config: &Config, dry_run: bool) -> Result<()> {
    match &args.subcommand {
        GitSubcommand::SetRemotes(sr) => {
            let key_path = sr.key.as_deref();
            set_remotes_for_all(config, &sr.username, &sr.email, key_path, dry_run).map_err(|e| {
                eprintln!("Failed to set remotes: {e}");
                e
            })
        }
        GitSubcommand::AddRemote(ar) => {
            let key_path = ar.key.as_deref();
            let repos: Vec<String> = ar
                .path
                .as_ref()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .map(|s| vec![s.to_string()])
                .unwrap_or_default();
            add_remote_to_repos(config, &ar.name, &ar.username, key_path, &repos, dry_run).map_err(
                |e| {
                    eprintln!("Failed to add remote: {e}");
                    e
                },
            )
        }
        GitSubcommand::IgnoreTs(it) => {
            let enable = it.state == IgnoreTsState::On;
            match set_ignore_ts(config, enable, dry_run) {
                Ok(count) => {
                    if dry_run {
                        println!(
                            "Would {} assume-unchanged on {} .ts files",
                            if enable { "set" } else { "unset" },
                            count
                        );
                    } else {
                        println!(
                            "{} assume-unchanged on {} .ts files",
                            if enable { "Set" } else { "Unset" },
                            count
                        );
                    }
                    Ok(())
                }
                Err(e) => {
                    eprintln!("Failed to set ignore-ts: {e}");
                    Err(e)
                }
            }
        }
        GitSubcommand::Branches(br) => match list_branches(config) {
            Ok(branches) => {
                for (path, branch) in branches {
                    let repo_name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown");
                    if !br.all && (branch == "master" || branch == "main") {
                        continue;
                    }
                    println!("{repo_name:30} {branch}");
                }
                Ok(())
            }
            Err(e) => {
                eprintln!("Failed to list branches: {e}");
                Err(e)
            }
        },
    }
}
