// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Git backend abstraction layer.
//!
//! ```text
//! GitQuery (read)  --> GixBackend (pure Rust gix)
//! GitMutation (write) --> ShellBackend (git CLI)
//! ```

use crate::error::{GitError, GixError, MobResult};
use std::path::Path;

// --- Query Trait (Read-only operations) ---

/// Read-only git query operations.
///
/// Implementors provide methods to inspect repository state without modification.
pub trait GitQuery {
    /// Check if path is inside a git work tree.
    fn is_git_repo(path: &Path) -> bool;

    /// Get current branch name (None if HEAD is detached).
    ///
    /// # Errors
    ///
    /// Returns a `GitError` if repository discovery or head resolution fails.
    fn current_branch(path: &Path) -> MobResult<Option<String>>;

    /// Check if file is tracked by git.
    ///
    /// # Errors
    ///
    /// Returns a `GitError` if repository discovery or index access fails.
    fn is_tracked(repo_path: &Path, file: &Path) -> MobResult<bool>;

    /// Check for uncommitted changes (staged, unstaged, or untracked files).
    ///
    /// # Errors
    ///
    /// Returns a `GitError` if repository discovery or status check fails.
    fn has_uncommitted_changes(path: &Path) -> MobResult<bool>;

    /// Check for stashed changes.
    ///
    /// # Errors
    ///
    /// Returns a `GitError` if repository discovery or reference lookup fails.
    fn has_stashed_changes(path: &Path) -> MobResult<bool>;
}

// --- Mutation Trait (Write operations) ---

/// Git mutation operations that modify repository state.
///
/// These operations use shell git for:
/// - `PuTTY` SSH key support (Windows)
/// - Submodule recursive operations
/// - Full git CLI compatibility
pub trait GitMutation {
    /// Clone a repository.
    ///
    /// # Errors
    ///
    /// Returns a `GitError` if the clone operation fails or the destination path is invalid.
    fn clone(url: &str, dest: &Path, branch: Option<&str>, shallow: bool) -> MobResult<()>;

    /// Pull with recurse-submodules.
    ///
    /// # Errors
    ///
    /// Returns a `GitError` if the pull operation fails.
    fn pull(repo_path: &Path, remote: &str, branch: &str) -> MobResult<()>;

    /// Fetch from remote.
    ///
    /// # Errors
    ///
    /// Returns a `GitError` if the fetch operation fails.
    fn fetch(repo_path: &Path, remote: &str) -> MobResult<()>;

    /// Checkout a branch, tag, or commit.
    ///
    /// # Errors
    ///
    /// Returns a `GitError` if the checkout operation fails.
    fn checkout(repo_path: &Path, what: &str) -> MobResult<()>;

    /// Initialize a new repository.
    ///
    /// # Errors
    ///
    /// Returns a `GitError` if repository initialization fails.
    fn init_repo(path: &Path) -> MobResult<()>;

    /// Add a submodule.
    ///
    /// # Errors
    ///
    /// Returns a `GitError` if the submodule cannot be added.
    fn add_submodule(repo_path: &Path, url: &str, submodule_path: &str) -> MobResult<()>;

    /// Add a remote, optionally with `PuTTY` key.
    ///
    /// # Errors
    ///
    /// Returns a `GitError` if the remote cannot be added or the `PuTTY` key path is invalid.
    fn add_remote(
        repo_path: &Path,
        name: &str,
        url: &str,
        putty_key: Option<&Path>,
    ) -> MobResult<()>;

    /// Rename a remote.
    ///
    /// # Errors
    ///
    /// Returns a `GitError` if the remote cannot be renamed.
    fn rename_remote(repo_path: &Path, old_name: &str, new_name: &str) -> MobResult<()>;

    /// Set remote push URL.
    ///
    /// # Errors
    ///
    /// Returns a `GitError` if the push URL cannot be set.
    fn set_remote_push_url(repo_path: &Path, remote: &str, url: &str) -> MobResult<()>;

    /// Set git config value.
    ///
    /// # Errors
    ///
    /// Returns a `GitError` if the config value cannot be set.
    fn set_config(repo_path: &Path, key: &str, value: &str) -> MobResult<()>;

    /// Mark file as assume-unchanged.
    ///
    /// # Errors
    ///
    /// Returns a `GitError` if the update-index operation fails or the file path is invalid.
    fn set_assume_unchanged(repo_path: &Path, file: &Path) -> MobResult<()>;

    /// Remove assume-unchanged flag from file.
    ///
    /// # Errors
    ///
    /// Returns a `GitError` if the update-index operation fails or the file path is invalid.
    fn unset_assume_unchanged(repo_path: &Path, file: &Path) -> MobResult<()>;
}

// --- GixBackend Implementation (Pure Rust) ---

/// Pure Rust git backend using gix.
///
/// Provides efficient read-only operations without spawning subprocesses.
/// All methods are zero-cost for repository discovery and branch queries.
pub struct GixBackend;

impl GitQuery for GixBackend {
    fn is_git_repo(path: &Path) -> bool {
        gix::discover(path).is_ok()
    }

    fn current_branch(path: &Path) -> MobResult<Option<String>> {
        let repo =
            gix::discover(path).map_err(|e| GitError::Gix(GixError::Discover(Box::new(e))))?;
        let head = repo
            .head_name()
            .map_err(|e| GitError::Gix(GixError::Head(e)))?;
        Ok(head.map(|name| name.shorten().to_string()))
    }

    fn is_tracked(repo_path: &Path, file: &Path) -> MobResult<bool> {
        let repo =
            gix::discover(repo_path).map_err(|e| GitError::Gix(GixError::Discover(Box::new(e))))?;
        let workdir = repo
            .workdir()
            .ok_or(GitError::Gix(GixError::BareRepository))?;
        let relative = file.strip_prefix(workdir).unwrap_or(file);
        let index = repo
            .index()
            .map_err(|e| GitError::Gix(GixError::Index(e)))?;
        let relative_bstr = gix::path::into_bstr(relative);
        Ok(index.entry_by_path(&relative_bstr).is_some())
    }

    fn has_uncommitted_changes(path: &Path) -> MobResult<bool> {
        use gix::status::UntrackedFiles;

        let repo =
            gix::discover(path).map_err(|e| GitError::Gix(GixError::Discover(Box::new(e))))?;

        let has_changes = repo
            .status(gix::progress::Discard)
            .map_err(|_| GitError::CommandFailed {
                command: "status".to_string(),
                message: "failed to prepare status check".to_string(),
            })?
            .untracked_files(UntrackedFiles::Files)
            .into_iter(None)
            .map_err(|_| GitError::CommandFailed {
                command: "status".to_string(),
                message: "failed to check repository status".to_string(),
            })?
            .next()
            .is_some();

        Ok(has_changes)
    }

    fn has_stashed_changes(path: &Path) -> MobResult<bool> {
        let repo =
            gix::discover(path).map_err(|e| GitError::Gix(GixError::Discover(Box::new(e))))?;

        // refs/stash exists iff there are stashed changes
        match repo.find_reference("refs/stash") {
            Ok(_) => Ok(true),
            Err(gix::reference::find::existing::Error::NotFound { name: _ }) => Ok(false),
            Err(e) => Err(GitError::Gix(GixError::Head(e)).into()),
        }
    }
}

// --- ShellBackend Implementation (Git CLI) ---

/// Shell-based git backend using git CLI.
///
/// Required for:
/// - SSH with `PuTTY` keys (`GIT_SSH` integration)
/// - Submodule recursive operations
/// - Operations not yet supported by gix
pub struct ShellBackend;

impl ShellBackend {
    /// Execute a git command. Sets `GCM_INTERACTIVE=never` and `GIT_TERMINAL_PROMPT=0`.
    pub(crate) fn git_command(args: &[&str], cwd: &Path) -> MobResult<String> {
        use std::process::Command;

        let output = Command::new("git")
            .args(args)
            .current_dir(cwd)
            .env("GCM_INTERACTIVE", "never")
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .map_err(|e| std::io::Error::new(e.kind(), format!("failed to execute git: {e}")))?;

        if !output.status.success() {
            return Err(GitError::CommandFailed {
                command: format!("git {}", args.join(" ")),
                message: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            }
            .into());
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

impl GitMutation for ShellBackend {
    fn clone(url: &str, dest: &Path, branch: Option<&str>, shallow: bool) -> MobResult<()> {
        let mut args = vec!["clone", "--recurse-submodules", "--quiet"];
        args.extend(&["-c", "advice.detachedHead=false"]);
        if shallow {
            args.extend(&["--depth", "1"]);
        }
        if let Some(b) = branch {
            args.extend(&["--branch", b]);
        }
        args.push(url);
        let dest_str = dest.to_str().ok_or_else(|| GitError::CloneFailed {
            url: url.to_string(),
            message: "invalid destination path".to_string(),
        })?;
        args.push(dest_str);

        let parent = dest.parent().unwrap_or_else(|| Path::new("."));
        Self::git_command(&args, parent)?;
        Ok(())
    }

    fn pull(repo_path: &Path, remote: &str, branch: &str) -> MobResult<()> {
        Self::git_command(
            &["pull", "--recurse-submodules", "--quiet", remote, branch],
            repo_path,
        )?;
        Ok(())
    }

    fn fetch(repo_path: &Path, remote: &str) -> MobResult<()> {
        Self::git_command(&["fetch", "--quiet", remote], repo_path)?;
        Ok(())
    }

    fn checkout(repo_path: &Path, what: &str) -> MobResult<()> {
        Self::git_command(
            &["-c", "advice.detachedHead=false", "checkout", "-q", what],
            repo_path,
        )?;
        Ok(())
    }

    fn init_repo(path: &Path) -> MobResult<()> {
        Self::git_command(&["init", "--quiet"], path)?;
        Ok(())
    }

    fn add_submodule(repo_path: &Path, url: &str, submodule_path: &str) -> MobResult<()> {
        Self::git_command(
            &["submodule", "add", "--quiet", url, submodule_path],
            repo_path,
        )?;
        Ok(())
    }

    fn add_remote(
        repo_path: &Path,
        name: &str,
        url: &str,
        putty_key: Option<&Path>,
    ) -> MobResult<()> {
        Self::git_command(&["remote", "add", name, url], repo_path)?;
        if let Some(key) = putty_key {
            let config_key = format!("remote.{name}.puttykeyfile");
            let key_str = key.to_str().ok_or_else(|| GitError::CommandFailed {
                command: "git config".to_string(),
                message: "invalid key path".to_string(),
            })?;
            Self::git_command(&["config", &config_key, key_str], repo_path)?;
        }
        Ok(())
    }

    fn rename_remote(repo_path: &Path, old_name: &str, new_name: &str) -> MobResult<()> {
        Self::git_command(&["remote", "rename", old_name, new_name], repo_path)?;
        Ok(())
    }

    fn set_remote_push_url(repo_path: &Path, remote: &str, url: &str) -> MobResult<()> {
        Self::git_command(&["remote", "set-url", "--push", remote, url], repo_path)?;
        Ok(())
    }

    fn set_config(repo_path: &Path, key: &str, value: &str) -> MobResult<()> {
        Self::git_command(&["config", key, value], repo_path)?;
        Ok(())
    }

    fn set_assume_unchanged(repo_path: &Path, file: &Path) -> MobResult<()> {
        let file_str = file.to_str().ok_or_else(|| GitError::CommandFailed {
            command: "git update-index".to_string(),
            message: "invalid file path".to_string(),
        })?;
        Self::git_command(&["update-index", "--assume-unchanged", file_str], repo_path)?;
        Ok(())
    }

    fn unset_assume_unchanged(repo_path: &Path, file: &Path) -> MobResult<()> {
        let file_str = file.to_str().ok_or_else(|| GitError::CommandFailed {
            command: "git update-index".to_string(),
            message: "invalid file path".to_string(),
        })?;
        Self::git_command(
            &["update-index", "--no-assume-unchanged", file_str],
            repo_path,
        )?;
        Ok(())
    }
}

impl GitQuery for ShellBackend {
    fn is_git_repo(path: &Path) -> bool {
        Self::git_command(&["rev-parse", "--is-inside-work-tree"], path).is_ok()
    }

    fn current_branch(path: &Path) -> MobResult<Option<String>> {
        Self::git_command(&["symbolic-ref", "--short", "HEAD"], path)
            .map_or_else(|_| Ok(None), |branch| Ok(Some(branch)))
    }

    fn is_tracked(repo_path: &Path, file: &Path) -> MobResult<bool> {
        let file_str = file.to_str().ok_or_else(|| GitError::CommandFailed {
            command: "git ls-files".to_string(),
            message: "invalid file path".to_string(),
        })?;
        let output = Self::git_command(&["ls-files", "--error-unmatch", file_str], repo_path);
        Ok(output.is_ok())
    }

    fn has_uncommitted_changes(path: &Path) -> MobResult<bool> {
        let output = Self::git_command(&["status", "--porcelain"], path)?;
        Ok(!output.is_empty())
    }

    fn has_stashed_changes(path: &Path) -> MobResult<bool> {
        let output = Self::git_command(&["stash", "list"], path);
        output.map_or_else(|_| Ok(false), |list| Ok(!list.is_empty()))
    }
}

#[cfg(test)]
mod tests;
