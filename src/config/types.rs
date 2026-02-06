// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Configuration types for mob-rs.
//!
//! # Config Structure
//!
//! ```text
//! Config: GlobalConfig, TaskConfig, PathsConfig, ToolsConfig, VersionsConfig
//! Aliases: task name → [task list]
//! ```
//!
//! # Build Configuration
//!
//! ```text
//! BuildConfiguration: Debug | Release | RelWithDebInfo (default)
//! ```

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::error::ConfigError;
use crate::logging::LogLevel;

/// Build configuration type (Debug, Release, `RelWithDebInfo`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum BuildConfiguration {
    Debug,
    Release,
    #[default]
    RelWithDebInfo,
}

impl std::fmt::Display for BuildConfiguration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Debug => write!(f, "Debug"),
            Self::Release => write!(f, "Release"),
            Self::RelWithDebInfo => write!(f, "RelWithDebInfo"),
        }
    }
}

impl std::str::FromStr for BuildConfiguration {
    type Err = ConfigError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "debug" => Ok(Self::Debug),
            "release" => Ok(Self::Release),
            "relwithdebinfo" => Ok(Self::RelWithDebInfo),
            _ => Err(ConfigError::InvalidValue {
                section: "task".to_string(),
                key: "configuration".to_string(),
                message: format!("expected 'Debug', 'Release', or 'RelWithDebInfo', got '{s}'"),
            }),
        }
    }
}

/// `CMake` install message verbosity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CmakeInstallMessage {
    Always,
    Lazy,
    #[default]
    Never,
}

impl std::fmt::Display for CmakeInstallMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Always => write!(f, "ALWAYS"),
            Self::Lazy => write!(f, "LAZY"),
            Self::Never => write!(f, "NEVER"),
        }
    }
}

/// Global configuration options.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GlobalConfig {
    /// Simulate filesystem operations without making changes.
    pub dry: bool,
    /// Download clean actions.
    #[serde(flatten)]
    pub clean_download_actions: CleanDownloadActions,

    /// Log level for stdout output (0-6).
    pub output_log_level: LogLevel,
    /// Log level for file output (0-6).
    pub file_log_level: LogLevel,
    /// Path to log file.
    pub log_file: PathBuf,
    /// Allow deleting directories with uncommitted git changes.
    pub ignore_uncommitted: bool,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            dry: false,
            clean_download_actions: CleanDownloadActions::default(),
            output_log_level: LogLevel::INFO,
            file_log_level: LogLevel::TRACE,
            log_file: PathBuf::from("mob.log"),
            ignore_uncommitted: false,
        }
    }
}

/// Global clean actions for downloads.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CleanDownloadActions {
    /// Re-download archives even if they already exist.
    pub redownload: bool,
    /// Re-extract archives even if target directory exists.
    pub reextract: bool,
}

/// CMake-specific configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CmakeConfig {
    /// Value for `CMAKE_INSTALL_MESSAGE`.
    pub install_message: CmakeInstallMessage,
    /// Toolset host configuration (-T host=XXX).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub host: String,
}

/// Task aliases mapping alias names to task patterns.
pub type Aliases = BTreeMap<String, Vec<String>>;

/// Task-specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TaskConfig {
    /// Whether this task is enabled.
    pub enabled: bool,
    /// GitHub organization for `ModOrganizer` projects.
    pub mo_org: String,
    /// Git branch to use for `ModOrganizer` projects.
    pub mo_branch: String,
    /// Fallback branch if `mo_branch` doesn't exist.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub mo_fallback: String,
    /// Git behavior settings.
    #[serde(flatten)]
    pub git_behavior: GitBehavior,
    /// Build configuration (Debug, Release, `RelWithDebInfo`).
    pub configuration: BuildConfiguration,
    /// Git URL prefix for cloning.
    pub git_url_prefix: String,
    /// Git clone settings.
    #[serde(flatten)]
    pub git_clone: GitCloneOptions,
    /// Remote setup settings.
    #[serde(flatten)]
    pub remote_setup: RemoteSetup,
}

impl Default for TaskConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            mo_org: "ModOrganizer2".to_string(),
            mo_branch: "master".to_string(),
            mo_fallback: String::new(),
            git_behavior: GitBehavior::default(),
            configuration: BuildConfiguration::default(),
            git_url_prefix: "https://github.com/".to_string(),
            git_clone: GitCloneOptions::default(),
            remote_setup: RemoteSetup::default(),
        }
    }
}

/// Git behavior settings for tasks.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GitBehavior {
    /// Don't pull if repo is already cloned.
    pub no_pull: bool,
}

/// Git clone options.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GitCloneOptions {
    /// Use shallow clones (--depth 1).
    pub git_shallow: bool,
}

impl Default for GitCloneOptions {
    fn default() -> Self {
        Self { git_shallow: true }
    }
}

/// Remote setup settings for tasks.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RemoteSetup {
    /// GitHub organization for the new origin remote.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub remote_org: String,
    /// Disable pushing to upstream.
    pub remote_no_push_upstream: bool,
    /// Set origin as default push remote.
    pub remote_push_default_origin: bool,
}

/// Tool paths configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ToolsConfig {
    /// 7-Zip executable.
    #[serde(rename = "7z")]
    pub sevenz: PathBuf,
    /// `CMake` executable.
    pub cmake: PathBuf,
    /// `MSBuild` executable.
    pub msbuild: PathBuf,
    /// Transifex CLI.
    pub tx: PathBuf,
    /// Qt lrelease (translation compiler).
    pub lrelease: PathBuf,
    /// Inno Setup compiler.
    pub iscc: PathBuf,
}

impl Default for ToolsConfig {
    fn default() -> Self {
        Self {
            sevenz: PathBuf::from("7z.exe"),
            cmake: PathBuf::from("cmake.exe"),
            msbuild: PathBuf::from("msbuild.exe"),
            tx: PathBuf::from("tx.exe"),
            lrelease: PathBuf::from("lrelease.exe"),
            iscc: PathBuf::from("ISCC.exe"),
        }
    }
}

/// Transifex translation service configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TransifexConfig {
    /// Whether Transifex integration is enabled.
    pub enabled: bool,
    /// Transifex API key.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub key: String,
    /// Transifex team slug.
    pub team: String,
    /// Transifex project slug.
    pub project: String,
    /// Transifex API URL.
    pub url: String,
    /// Minimum translation completion percentage.
    pub minimum: u8,
    /// Action toggles for Transifex operations.
    #[serde(flatten)]
    pub actions: TransifexActions,
}

impl Default for TransifexConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            key: String::new(),
            team: "mod-organizer-2-team".to_string(),
            project: "mod-organizer-2".to_string(),
            url: "https://app.transifex.com".to_string(),
            minimum: 60,
            actions: TransifexActions::default(),
        }
    }
}

/// Transifex action toggles.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransifexActions {
    /// Force re-pulling translations.
    pub force: bool,
    /// Run tx configure.
    pub configure: bool,
    /// Pull translations from Transifex.
    pub pull: bool,
}

impl Default for TransifexActions {
    fn default() -> Self {
        Self {
            force: false,
            configure: true,
            pull: true,
        }
    }
}

/// Version numbers for various dependencies.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct VersionsConfig {
    /// Visual Studio toolset version (e.g., 14.3).
    pub vs_toolset: String,
    /// Windows SDK version.
    pub sdk: String,
    /// USVFS version/branch.
    pub usvfs: String,
    /// Explorer++ version.
    pub explorerpp: String,
    /// Stylesheet versions (key: stylesheet name, value: version).
    #[serde(flatten)]
    pub stylesheets: BTreeMap<String, String>,
}

impl Default for VersionsConfig {
    fn default() -> Self {
        let mut stylesheets = BTreeMap::new();
        stylesheets.insert("ss_paper_lad_6788".to_string(), "7.2".to_string());
        stylesheets.insert("ss_paper_automata_6788".to_string(), "3.2".to_string());
        stylesheets.insert("ss_paper_mono_6788".to_string(), "3.2".to_string());
        stylesheets.insert("ss_dark_mode_1809_6788".to_string(), "3.0".to_string());
        stylesheets.insert("ss_morrowind_trosski".to_string(), "1.1".to_string());
        stylesheets.insert("ss_skyrim_trosski".to_string(), "v1.1".to_string());
        stylesheets.insert("ss_starfield_trosski".to_string(), "V1.11".to_string());
        stylesheets.insert("ss_fallout3_trosski".to_string(), "v1.11".to_string());
        stylesheets.insert("ss_fallout4_trosski".to_string(), "v1.11".to_string());

        Self {
            vs_toolset: "14.3".to_string(),
            sdk: "10.0.26100.0".to_string(),
            usvfs: "master".to_string(),
            explorerpp: "1.4.0".to_string(),
            stylesheets,
        }
    }
}
