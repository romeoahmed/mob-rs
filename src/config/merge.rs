// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Task configuration merging.
//!
//! ```text
//! TaskConfig + TaskConfigOverride --> field-by-field merge
//! ```
//!
//! Only explicitly set fields (`Some`) in override replace base values.

use serde::{Deserialize, Serialize};

use super::types::{BuildConfiguration, GitBehavior, GitCloneOptions, RemoteSetup, TaskConfig};

/// Task configuration with optional fields for field-level merging.
///
/// Used for task-specific overrides where only explicitly set fields
/// should override the base configuration. All fields are optional to
/// distinguish between "not set" (None) and "explicitly set to value".
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TaskConfigOverride {
    /// Whether this task is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// GitHub organization for `ModOrganizer` projects.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mo_org: Option<String>,
    /// Git branch to use for `ModOrganizer` projects.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mo_branch: Option<String>,
    /// Fallback branch if `mo_branch` doesn't exist.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mo_fallback: Option<String>,
    /// Git behavior: don't pull if repo is already cloned.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_pull: Option<bool>,
    /// Build configuration (Debug, Release, `RelWithDebInfo`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub configuration: Option<BuildConfiguration>,
    /// Git URL prefix for cloning.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_url_prefix: Option<String>,
    /// Use shallow clones (--depth 1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_shallow: Option<bool>,
    /// GitHub organization for the new origin remote.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_org: Option<String>,
    /// Disable pushing to upstream.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_no_push_upstream: Option<bool>,
    /// Set origin as default push remote.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_push_default_origin: Option<bool>,
}

/// Merge task-specific config over default config.
///
/// Only explicitly set fields (Some values) in the override take precedence.
/// None values are ignored, allowing the base configuration to be used.
pub(super) fn merge_task_config(
    base: &TaskConfig,
    override_config: &TaskConfigOverride,
) -> TaskConfig {
    TaskConfig {
        enabled: override_config.enabled.unwrap_or(base.enabled),
        mo_org: override_config
            .mo_org
            .clone()
            .unwrap_or_else(|| base.mo_org.clone()),
        mo_branch: override_config
            .mo_branch
            .clone()
            .unwrap_or_else(|| base.mo_branch.clone()),
        mo_fallback: override_config
            .mo_fallback
            .clone()
            .unwrap_or_else(|| base.mo_fallback.clone()),
        git_behavior: GitBehavior {
            no_pull: override_config.no_pull.unwrap_or(base.git_behavior.no_pull),
        },
        configuration: override_config.configuration.unwrap_or(base.configuration),
        git_url_prefix: override_config
            .git_url_prefix
            .clone()
            .unwrap_or_else(|| base.git_url_prefix.clone()),
        git_clone: GitCloneOptions {
            git_shallow: override_config
                .git_shallow
                .unwrap_or(base.git_clone.git_shallow),
        },
        remote_setup: RemoteSetup {
            remote_org: override_config
                .remote_org
                .clone()
                .unwrap_or_else(|| base.remote_setup.remote_org.clone()),
            remote_no_push_upstream: override_config
                .remote_no_push_upstream
                .unwrap_or(base.remote_setup.remote_no_push_upstream),
            remote_push_default_origin: override_config
                .remote_push_default_origin
                .unwrap_or(base.remote_setup.remote_push_default_origin),
        },
    }
}
