// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Types for environment variable management.
//!
//! # Architecture
//!
//! ```text
//! Arch: X86 → "x86" / X64 → "amd64" (vcvars_arg) + "x86"/"x64" (Display)
//! EnvFlags: Replace | Append | Prepend
//! EnvKey: case-insensitive on Windows (PATH == Path == path)
//! EnvData: BTreeMap<EnvKey, String> for deterministic order
//! ```

use std::collections::BTreeMap;

/// Target architecture for builds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Arch {
    /// 32-bit x86
    X86,
    /// 64-bit x86-64
    X64,
}

impl Arch {
    /// Returns the vcvars architecture string.
    #[must_use]
    pub const fn vcvars_arg(&self) -> &'static str {
        match self {
            Self::X86 => "x86",
            Self::X64 => "amd64",
        }
    }
}

impl std::fmt::Display for Arch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::X86 => write!(f, "x86"),
            Self::X64 => write!(f, "x64"),
        }
    }
}

/// Flags for environment variable operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EnvFlags {
    /// Replace the existing value (default)
    #[default]
    Replace,
    /// Append to the existing value
    Append,
    /// Prepend to the existing value
    Prepend,
}

/// A case-insensitive environment variable key (Windows-compatible).
#[derive(Debug, Clone, Eq)]
pub(super) struct EnvKey(String);

impl EnvKey {
    pub(super) fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub(super) fn as_str(&self) -> &str {
        &self.0
    }
}

impl PartialEq for EnvKey {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq_ignore_ascii_case(&other.0)
    }
}

impl std::hash::Hash for EnvKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for c in self.0.chars() {
            c.to_ascii_lowercase().hash(state);
        }
    }
}

impl PartialOrd for EnvKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for EnvKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0
            .to_ascii_lowercase()
            .cmp(&other.0.to_ascii_lowercase())
    }
}

/// Shared environment data for copy-on-write semantics.
#[derive(Debug, Clone)]
pub(super) struct EnvData {
    vars: BTreeMap<EnvKey, String>,
}

impl EnvData {
    pub(super) const fn new() -> Self {
        Self {
            vars: BTreeMap::new(),
        }
    }

    pub(super) const fn from_vars(vars: BTreeMap<EnvKey, String>) -> Self {
        Self { vars }
    }

    pub(super) const fn vars(&self) -> &BTreeMap<EnvKey, String> {
        &self.vars
    }

    pub(super) const fn vars_mut(&mut self) -> &mut BTreeMap<EnvKey, String> {
        &mut self.vars
    }
}
