// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Build command arguments.
//!
//! # Flag Effects
//!
//! ```text
//! --new (-n) implies: --redownload --reextract --reconfigure --rebuild
//!
//! Phase control: --clean-task/--no-clean-task, --fetch-task/--no-fetch-task,
//! --build-task/--no-build-task
//! Git options: --pull/--no-pull, --revert-ts/--no-revert-ts
//! ```

use clap::ArgAction;
use clap::Args;

/// Arguments for the `build` command.
#[derive(Debug, Clone, Default, Args)]
pub struct BuildArgs {
    /// Clean download actions.
    #[command(flatten)]
    pub clean_download: CleanDownloadArgs,

    /// Clean build actions.
    #[command(flatten)]
    pub clean_build: CleanBuildArgs,

    /// Full clean action.
    #[command(flatten)]
    pub clean_full: CleanFullArgs,

    /// Clean phase toggles.
    #[command(flatten)]
    pub clean_phase: CleanPhaseArgs,

    /// Fetch phase toggles.
    #[command(flatten)]
    pub fetch_phase: FetchPhaseArgs,

    /// Build phase toggles.
    #[command(flatten)]
    pub build_phase: BuildPhaseArgs,

    /// Pull behavior toggles.
    #[command(flatten)]
    pub pull_behavior: PullArgs,

    /// Revert .ts behavior toggles.
    #[command(flatten)]
    pub revert_ts_behavior: RevertTsArgs,

    /// When --reextract is given, directories controlled by git will be
    /// deleted even if they contain uncommitted changes.
    #[arg(long = "ignore-uncommitted-changes")]
    pub ignore_uncommitted: bool,

    /// Don't terminate msbuild.exe instances after building.
    #[arg(long = "keep-msbuild")]
    pub keep_msbuild: bool,

    /// Tasks to run. Specify 'super' to only build modorganizer projects.
    /// Globs like 'installer_*' are supported.
    #[arg(value_name = "TASK")]
    pub tasks: Vec<String>,
}

/// Clean download actions.
#[derive(Debug, Clone, Default, Args)]
pub struct CleanDownloadArgs {
    /// Re-downloads archives, see --reextract.
    #[arg(short = 'g', long, action = ArgAction::SetTrue)]
    pub redownload: bool,

    /// Deletes source directories and re-extracts archives.
    /// If the directory is controlled by git, deletes it and clones again.
    #[arg(short = 'e', long, action = ArgAction::SetTrue)]
    pub reextract: bool,
}

/// Clean build actions.
#[derive(Debug, Clone, Default, Args)]
pub struct CleanBuildArgs {
    /// Reconfigures the task by running cmake, configure scripts, etc.
    /// Some tasks might have to delete the whole source directory.
    #[arg(short = 'c', long, action = ArgAction::SetTrue)]
    pub reconfigure: bool,

    /// Cleans and rebuilds projects.
    /// Some tasks might have to delete the whole source directory.
    #[arg(short = 'b', long, action = ArgAction::SetTrue)]
    pub rebuild: bool,
}

/// Full clean action.
#[derive(Debug, Clone, Default, Args)]
pub struct CleanFullArgs {
    /// Deletes everything and starts from scratch.
    /// Implies --redownload, --reextract, --reconfigure, --rebuild.
    #[arg(short = 'n', long = "new", action = ArgAction::SetTrue)]
    pub new_build: bool,
}

/// Clean phase toggles.
#[derive(Debug, Clone, Default, Args)]
pub struct CleanPhaseArgs {
    /// Sets whether tasks are cleaned.
    #[arg(long = "clean-task", action = ArgAction::SetTrue, conflicts_with = "no_clean_task")]
    pub clean_task: bool,

    /// Sets whether tasks are NOT cleaned.
    #[arg(long = "no-clean-task", action = ArgAction::SetTrue, conflicts_with = "clean_task")]
    pub no_clean_task: bool,
}

/// Fetch phase toggles.
#[derive(Debug, Clone, Default, Args)]
pub struct FetchPhaseArgs {
    /// Sets whether tasks are fetched.
    #[arg(long = "fetch-task", action = ArgAction::SetTrue, conflicts_with = "no_fetch_task")]
    pub fetch_task: bool,

    /// Sets whether tasks are NOT fetched (download, git, etc.).
    #[arg(long = "no-fetch-task", action = ArgAction::SetTrue, conflicts_with = "fetch_task")]
    pub no_fetch_task: bool,
}

/// Build phase toggles.
#[derive(Debug, Clone, Default, Args)]
pub struct BuildPhaseArgs {
    /// Sets whether tasks are built.
    #[arg(long = "build-task", action = ArgAction::SetTrue, conflicts_with = "no_build_task")]
    pub build_task: bool,

    /// Sets whether tasks are NOT built.
    #[arg(long = "no-build-task", action = ArgAction::SetTrue, conflicts_with = "build_task")]
    pub no_build_task: bool,
}

/// Pull behavior toggles.
#[derive(Debug, Clone, Default, Args)]
pub struct PullArgs {
    /// Pull repos that are already cloned.
    #[arg(long = "pull", action = ArgAction::SetTrue, conflicts_with = "no_pull")]
    pub pull: bool,

    /// Don't pull repos that are already cloned.
    #[arg(long = "no-pull", action = ArgAction::SetTrue, conflicts_with = "pull")]
    pub no_pull: bool,
}

/// Revert .ts behavior toggles.
#[derive(Debug, Clone, Default, Args)]
pub struct RevertTsArgs {
    /// Revert all .ts files in a repo before pulling to avoid merge errors.
    #[arg(long = "revert-ts", action = ArgAction::SetTrue, conflicts_with = "no_revert_ts")]
    pub revert_ts: bool,

    /// Don't revert .ts files before pulling.
    #[arg(long = "no-revert-ts", action = ArgAction::SetTrue, conflicts_with = "revert_ts")]
    pub no_revert_ts: bool,
}

impl BuildArgs {
    /// Returns the effective `clean_task` setting.
    #[must_use]
    pub const fn clean_task_setting(&self) -> Option<bool> {
        if self.clean_phase.clean_task {
            Some(true)
        } else if self.clean_phase.no_clean_task {
            Some(false)
        } else {
            None
        }
    }

    /// Returns the effective `fetch_task` setting.
    #[must_use]
    pub const fn fetch_task_setting(&self) -> Option<bool> {
        if self.fetch_phase.fetch_task {
            Some(true)
        } else if self.fetch_phase.no_fetch_task {
            Some(false)
        } else {
            None
        }
    }

    /// Returns the effective `build_task` setting.
    #[must_use]
    pub const fn build_task_setting(&self) -> Option<bool> {
        if self.build_phase.build_task {
            Some(true)
        } else if self.build_phase.no_build_task {
            Some(false)
        } else {
            None
        }
    }

    /// Returns the effective pull setting.
    #[must_use]
    pub const fn pull_setting(&self) -> Option<bool> {
        if self.pull_behavior.pull {
            Some(true)
        } else if self.pull_behavior.no_pull {
            Some(false)
        } else {
            None
        }
    }

    /// Returns the effective `revert_ts` setting.
    #[must_use]
    pub const fn revert_ts_setting(&self) -> Option<bool> {
        if self.revert_ts_behavior.revert_ts {
            Some(true)
        } else if self.revert_ts_behavior.no_revert_ts {
            Some(false)
        } else {
            None
        }
    }

    /// Converts build arguments to configuration overrides.
    #[must_use]
    pub fn to_config_overrides(&self) -> Vec<String> {
        // Boolean flags that trigger when true (or when new_build is set)
        let bool_overrides = [
            (
                self.clean_download.redownload || self.clean_full.new_build,
                "global/redownload=true",
            ),
            (
                self.clean_download.reextract || self.clean_full.new_build,
                "global/reextract=true",
            ),
            (self.ignore_uncommitted, "global/ignore_uncommitted=true"),
        ]
        .into_iter()
        .filter(|(cond, _)| *cond)
        .map(|(_, key)| key.to_string());

        // Optional settings that format when Some
        let option_overrides = self
            .pull_setting()
            .map(|v| format!("_override:task/no_pull={}", !v))
            .into_iter();

        // Task filters: disable all, then enable specified ones
        let task_overrides = if self.tasks.is_empty() {
            Vec::new()
        } else {
            std::iter::once("task/enabled=false".to_string())
                .chain(self.tasks.iter().map(|t| format!("{t}:task/enabled=true")))
                .collect()
        };

        bool_overrides
            .chain(option_overrides)
            .chain(task_overrides)
            .collect()
    }
}

/// Arguments for the `list` command.
#[derive(Debug, Clone, Default, Args)]
pub struct ListArgs {
    /// Shows all tasks, including pseudo parallel tasks.
    #[arg(short = 'a', long)]
    pub all: bool,

    /// Shows only aliases.
    #[arg(short = 'i', long)]
    pub aliases: bool,

    /// With -a; when given, acts like the tasks given to `build` and
    /// shows only the tasks that would run.
    #[arg(value_name = "TASK")]
    pub tasks: Vec<String>,
}
