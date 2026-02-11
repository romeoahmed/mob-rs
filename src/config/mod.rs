// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Configuration management for mob-rs.
//!
//! # Configuration Hierarchy
//!
//! ```text
//! Priority (low → high)
//! 1. defaults
//! 2. master mob.toml (exe dir)
//! 3. MOBINI (semicolon-separated paths)
//! 4. local mob.toml (cwd)
//! 5. --config
//! 6. MOB_* env vars
//! 7. CLI overrides
//! ```
//!
//! # Environment Variable Mapping
//!
//! ```text
//! MOB_GLOBAL_DRY=true     → global.dry = true
//! MOB_PATHS_PREFIX=/path  → paths.prefix = "/path"
//! MOB_TASK_MO_ORG=MyOrg   → task.mo_org = "MyOrg"
//! ```
//!
//! # Task-Specific Overrides
//!
//! ```toml
//! [task]
//! git_shallow = true
//!
//! [tasks.usvfs]
//! git_shallow = false # override for usvfs only
//! ```

pub mod loader;
pub mod merge;
pub mod paths;
pub mod types;

#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use wax::Program as _;

use crate::error::Result;

use loader::ConfigLoader;
use merge::TaskConfigOverride;
use paths::PathsConfig;
use types::{
    Aliases, CmakeConfig, GlobalConfig, TaskConfig, ToolsConfig, TransifexConfig, VersionsConfig,
};

/// Complete application configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// Global options.
    pub global: GlobalConfig,
    /// `CMake` options.
    pub cmake: CmakeConfig,
    /// Task aliases.
    pub aliases: Aliases,
    /// Default task configuration.
    pub task: TaskConfig,
    /// Per-task configuration overrides (field-level merging).
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub tasks: BTreeMap<String, TaskConfigOverride>,
    /// Tool paths.
    pub tools: ToolsConfig,
    /// Transifex configuration.
    pub transifex: TransifexConfig,
    /// Version numbers.
    pub versions: VersionsConfig,
    /// Paths configuration.
    pub paths: PathsConfig,
}

impl Config {
    /// Create a new configuration builder.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use mob_rs::config::Config;
    ///
    /// let config = Config::builder()
    ///     .add_toml_file("config/default.toml")
    ///     .add_toml_file_optional("config/local.toml")
    ///     .with_env_prefix("MOB")
    ///     .build()?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    #[must_use]
    pub fn builder() -> ConfigLoader {
        ConfigLoader::new()
    }

    /// Load configuration from a single TOML file (simple API).
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read, contains invalid TOML, or
    /// does not match the `Config` structure.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self::builder().add_toml_file(path).build()
    }

    /// Load configuration from a TOML string.
    ///
    /// # Errors
    ///
    /// Returns an error if the content is not valid TOML or does not match the
    /// `Config` structure.
    pub fn parse(content: &str) -> Result<Self> {
        Self::builder().add_toml_str(content).build()
    }

    /// Get task configuration for a specific task.
    ///
    /// Resolution order:
    /// 1. Exact match on task name (e.g., `[tasks.usvfs]`)
    /// 2. Glob pattern match (e.g., `[tasks.installer_*]`)
    /// 3. Alias expansion — if the task name matches any pattern in an alias's
    ///    target list, and that alias has a `[tasks.<alias>]` override, it applies.
    ///    (e.g., `[tasks.super]` applies to all tasks in the `super` alias group)
    /// 4. Default `[task]` config
    #[must_use]
    pub fn task_config(&self, task_name: &str) -> TaskConfig {
        // Exact match
        if let Some(config) = self.tasks.get(task_name) {
            return merge::merge_task_config(&self.task, config);
        }

        // Glob pattern match (non-alias entries only)
        for (pattern, config) in &self.tasks {
            if self.aliases.contains_key(pattern) {
                continue;
            }
            if let Ok(glob) = wax::Glob::new(pattern)
                && glob.is_match(task_name)
            {
                return merge::merge_task_config(&self.task, config);
            }
        }

        // Alias expansion: check if task_name matches any pattern in an alias's
        // target list, and if that alias has a [tasks.<alias>] config override.
        for (alias_name, alias_targets) in &self.aliases {
            if let Some(config) = self.tasks.get(alias_name) {
                for target_pattern in alias_targets {
                    // Check exact match first
                    if target_pattern == task_name {
                        return merge::merge_task_config(&self.task, config);
                    }
                    // Then glob match
                    if let Ok(glob) = wax::Glob::new(target_pattern)
                        && glob.is_match(task_name)
                    {
                        return merge::merge_task_config(&self.task, config);
                    }
                }
            }
        }

        self.task.clone()
    }

    /// Resolve all paths and validate configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if path resolution fails (e.g., missing required `prefix`).
    pub fn resolve_and_validate(&mut self) -> Result<()> {
        if self.paths.prefix.is_some() {
            self.paths.resolve()?;
        }
        Ok(())
    }

    /// Format configuration options for display.
    ///
    /// Returns a vector of formatted strings representing all configuration options.
    /// Sensitive fields (like passwords and keys) are hidden with `[hidden]` marker.
    /// Output is deterministically ordered using `BTreeMap`.
    #[must_use]
    pub fn format_options(&self) -> Vec<String> {
        let mut options = BTreeMap::new();
        self.format_global_options(&mut options);
        self.format_cmake_options(&mut options);
        self.format_task_options(&mut options);
        self.format_tools_options(&mut options);
        self.format_transifex_options(&mut options);
        self.format_versions_options(&mut options);
        self.format_paths_options(&mut options);

        let max_key_len = options.keys().map(String::len).max().unwrap_or(0);

        options
            .into_iter()
            .map(|(key, value)| format!("{key:<max_key_len$} = {value}"))
            .collect()
    }

    fn format_global_options(&self, options: &mut BTreeMap<String, String>) {
        options.insert("global.dry".into(), self.global.dry.to_string());
        options.insert(
            "global.redownload".into(),
            self.global.clean_download_actions.redownload.to_string(),
        );
        options.insert(
            "global.reextract".into(),
            self.global.clean_download_actions.reextract.to_string(),
        );
        options.insert(
            "global.output_log_level".into(),
            self.global.output_log_level.as_u8().to_string(),
        );
        options.insert(
            "global.file_log_level".into(),
            self.global.file_log_level.as_u8().to_string(),
        );
        options.insert(
            "global.log_file".into(),
            self.global.log_file.display().to_string(),
        );
        options.insert(
            "global.ignore_uncommitted".into(),
            self.global.ignore_uncommitted.to_string(),
        );
    }

    fn format_cmake_options(&self, options: &mut BTreeMap<String, String>) {
        options.insert(
            "cmake.install_message".into(),
            self.cmake.install_message.to_string(),
        );
        if !self.cmake.host.is_empty() {
            options.insert("cmake.host".into(), self.cmake.host.clone());
        }
    }

    fn format_task_options(&self, options: &mut BTreeMap<String, String>) {
        options.insert("task.enabled".into(), self.task.enabled.to_string());
        options.insert("task.mo_org".into(), self.task.mo_org.clone());
        options.insert("task.mo_branch".into(), self.task.mo_branch.clone());
        if !self.task.mo_fallback.is_empty() {
            options.insert("task.mo_fallback".into(), self.task.mo_fallback.clone());
        }
        options.insert(
            "task.no_pull".into(),
            self.task.git_behavior.no_pull.to_string(),
        );
        options.insert(
            "task.configuration".into(),
            self.task.configuration.to_string(),
        );
        options.insert(
            "task.git_url_prefix".into(),
            self.task.git_url_prefix.clone(),
        );
        options.insert(
            "task.git_shallow".into(),
            self.task.git_clone.git_shallow.to_string(),
        );
        if !self.task.remote_setup.remote_org.is_empty() {
            options.insert(
                "task.remote_org".into(),
                self.task.remote_setup.remote_org.clone(),
            );
        }
        options.insert(
            "task.remote_no_push_upstream".into(),
            self.task.remote_setup.remote_no_push_upstream.to_string(),
        );
        options.insert(
            "task.remote_push_default_origin".into(),
            self.task
                .remote_setup
                .remote_push_default_origin
                .to_string(),
        );
    }

    fn format_tools_options(&self, options: &mut BTreeMap<String, String>) {
        options.insert("tools.7z".into(), self.tools.sevenz.display().to_string());
        options.insert("tools.cmake".into(), self.tools.cmake.display().to_string());
        options.insert(
            "tools.msbuild".into(),
            self.tools.msbuild.display().to_string(),
        );
        options.insert("tools.tx".into(), self.tools.tx.display().to_string());
        options.insert(
            "tools.lrelease".into(),
            self.tools.lrelease.display().to_string(),
        );
        options.insert("tools.iscc".into(), self.tools.iscc.display().to_string());
    }

    fn format_transifex_options(&self, options: &mut BTreeMap<String, String>) {
        options.insert(
            "transifex.enabled".into(),
            self.transifex.enabled.to_string(),
        );
        if !self.transifex.key.is_empty() {
            options.insert("transifex.key".into(), "[hidden]".into());
        }
        options.insert("transifex.team".into(), self.transifex.team.clone());
        options.insert("transifex.project".into(), self.transifex.project.clone());
        options.insert("transifex.url".into(), self.transifex.url.clone());
        options.insert(
            "transifex.minimum".into(),
            self.transifex.minimum.to_string(),
        );
        options.insert(
            "transifex.force".into(),
            self.transifex.actions.force.to_string(),
        );
        options.insert(
            "transifex.configure".into(),
            self.transifex.actions.configure.to_string(),
        );
        options.insert(
            "transifex.pull".into(),
            self.transifex.actions.pull.to_string(),
        );
    }

    fn format_versions_options(&self, options: &mut BTreeMap<String, String>) {
        options.insert(
            "versions.vs_toolset".into(),
            self.versions.vs_toolset.clone(),
        );
        options.insert("versions.sdk".into(), self.versions.sdk.clone());
        options.insert("versions.usvfs".into(), self.versions.usvfs.clone());
        options.insert(
            "versions.explorerpp".into(),
            self.versions.explorerpp.clone(),
        );
        for (name, version) in &self.versions.stylesheets {
            options.insert(format!("versions.{name}"), version.clone());
        }
    }

    fn format_paths_options(&self, options: &mut BTreeMap<String, String>) {
        let fmt = |p: &Option<PathBuf>| {
            p.as_ref()
                .map_or_else(String::new, |p| p.display().to_string())
        };

        options.insert("paths.prefix".into(), fmt(&self.paths.prefix));
        options.insert("paths.cache".into(), fmt(&self.paths.cache));
        options.insert("paths.build".into(), fmt(&self.paths.build));
        options.insert("paths.install".into(), fmt(&self.paths.install));
        options.insert("paths.install_bin".into(), fmt(&self.paths.install_bin));
        options.insert(
            "paths.install_installer".into(),
            fmt(&self.paths.install_installer),
        );
        options.insert("paths.install_libs".into(), fmt(&self.paths.install_libs));
        options.insert("paths.install_pdbs".into(), fmt(&self.paths.install_pdbs));
        options.insert(
            "paths.install_stylesheets".into(),
            fmt(&self.paths.install_stylesheets),
        );
        options.insert(
            "paths.install_licenses".into(),
            fmt(&self.paths.install_licenses),
        );
        options.insert(
            "paths.install_translations".into(),
            fmt(&self.paths.install_translations),
        );
        options.insert("paths.licenses".into(), fmt(&self.paths.licenses));
        options.insert("paths.vcpkg".into(), fmt(&self.paths.vcpkg));
        options.insert("paths.qt_install".into(), fmt(&self.paths.qt_install));
        options.insert("paths.qt_bin".into(), fmt(&self.paths.qt_bin));
        options.insert(
            "paths.qt_translations".into(),
            fmt(&self.paths.qt_translations),
        );
    }
}
