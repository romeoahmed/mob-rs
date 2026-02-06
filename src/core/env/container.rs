// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Environment variable struct and copy-on-write implementation.
//!
//! # Architecture
//!
//! ```text
//! Env (copy-on-write)
//! data: Option<Arc<EnvData>> + owned flag
//! clone shares Arc until copy_for_write()
//!
//! Cached VS envs (Windows)
//! Env::vs_x86 / Env::vs_x64 via OnceLock + capture_vcvars()
//! ```

use super::types::{EnvData, EnvFlags, EnvKey};
use crate::error::Result;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

/// A set of environment variables with copy-on-write semantics.
///
/// This struct provides efficient cloning by sharing data between copies
/// until a modification is made.
///
/// # Thread Safety
/// `Env` is `Send` and `Sync` due to its use of `Arc`.
#[derive(Debug, Clone)]
pub struct Env {
    data: Option<Arc<EnvData>>,
    /// Whether we own the data exclusively (for copy-on-write)
    owned: bool,
}

impl Default for Env {
    fn default() -> Self {
        Self::new()
    }
}

impl Env {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            data: None,
            owned: false,
        }
    }

    /// Creates an environment from a map of variables.
    #[must_use]
    pub fn from_map(vars: BTreeMap<String, String>) -> Self {
        let data = EnvData::from_vars(vars.into_iter().map(|(k, v)| (EnvKey::new(k), v)).collect());
        Self {
            data: Some(Arc::new(data)),
            owned: true,
        }
    }

    /// Returns the Visual Studio x86 environment.
    ///
    /// The result is cached after the first call.
    ///
    /// # Errors
    ///
    /// Returns an error if the Visual Studio environment variables cannot be captured,
    /// typically because Visual Studio is not installed or `vcvarsall.bat` cannot be found.
    #[cfg(windows)]
    pub fn vs_x86() -> Result<Self> {
        use std::sync::OnceLock;

        static VS_X86: OnceLock<std::result::Result<Env, String>> = OnceLock::new();
        VS_X86
            .get_or_init(|| {
                super::vcvars::capture_vcvars(super::types::Arch::X86).map_err(|e| e.to_string())
            })
            .clone()
            .map_err(|e| anyhow::anyhow!(e))
    }

    /// Returns the Visual Studio x64 environment.
    ///
    /// The result is cached after the first call.
    ///
    /// # Errors
    ///
    /// Returns an error if the Visual Studio environment variables cannot be captured,
    /// typically because Visual Studio is not installed or `vcvarsall.bat` cannot be found.
    #[cfg(windows)]
    pub fn vs_x64() -> Result<Self> {
        use std::sync::OnceLock;

        static VS_X64: OnceLock<std::result::Result<Env, String>> = OnceLock::new();
        VS_X64
            .get_or_init(|| {
                super::vcvars::capture_vcvars(super::types::Arch::X64).map_err(|e| e.to_string())
            })
            .clone()
            .map_err(|e| anyhow::anyhow!(e))
    }

    /// Returns the Visual Studio environment for the given architecture.
    ///
    /// # Errors
    ///
    /// Returns an error if the Visual Studio environment variables cannot be captured.
    #[cfg(windows)]
    pub fn vs(arch: super::types::Arch) -> Result<Self> {
        match arch {
            super::types::Arch::X86 => Self::vs_x86(),
            super::types::Arch::X64 => Self::vs_x64(),
        }
    }

    /// Sets an environment variable.
    ///
    /// # Arguments
    /// * `key` - The variable name
    /// * `value` - The variable value
    /// * `flags` - How to combine with existing value (default: Replace)
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.set_with_flags(key, value, EnvFlags::Replace)
    }

    /// Sets an environment variable with specific flags.
    pub fn set_with_flags(
        &mut self,
        key: impl Into<String>,
        value: impl Into<String>,
        flags: EnvFlags,
    ) -> &mut Self {
        self.copy_for_write();
        let key = EnvKey::new(key.into());
        let value = value.into();

        if let Some(ref mut data) = self.data {
            // SAFETY: We know we own the data exclusively after copy_for_write
            let data = Arc::make_mut(data);

            match flags {
                EnvFlags::Replace => {
                    data.vars_mut().insert(key, value);
                }
                EnvFlags::Append => {
                    if let Some(existing) = data.vars_mut().get_mut(&key) {
                        existing.push_str(&value);
                    } else {
                        data.vars_mut().insert(key, value);
                    }
                }
                EnvFlags::Prepend => {
                    if let Some(existing) = data.vars_mut().get_mut(&key) {
                        let mut new_value = value;
                        new_value.push_str(existing);
                        *existing = new_value;
                    } else {
                        data.vars_mut().insert(key, value);
                    }
                }
            }
        }

        self
    }

    /// Gets an environment variable value.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.data.as_ref().and_then(|d| {
            d.vars()
                .get(&EnvKey::new(key))
                .map(std::string::String::as_str)
        })
    }

    /// Removes an environment variable.
    pub fn remove(&mut self, key: &str) -> &mut Self {
        self.copy_for_write();
        if let Some(ref mut data) = self.data {
            let data = Arc::make_mut(data);
            data.vars_mut().remove(&EnvKey::new(key));
        }
        self
    }

    /// Prepends a path to the PATH environment variable.
    pub fn prepend_path(&mut self, path: impl AsRef<Path>) -> &mut Self {
        self.modify_path(path, EnvFlags::Prepend)
    }

    /// Appends a path to the PATH environment variable.
    pub fn append_path(&mut self, path: impl AsRef<Path>) -> &mut Self {
        self.modify_path(path, EnvFlags::Append)
    }

    /// Modifies the PATH environment variable.
    fn modify_path(&mut self, path: impl AsRef<Path>, flags: EnvFlags) -> &mut Self {
        let path_str = path.as_ref().to_string_lossy();
        let separator = if cfg!(windows) { ";" } else { ":" };

        match flags {
            EnvFlags::Prepend => {
                if let Some(current) = self.get("PATH") {
                    let new_path = format!("{path_str}{separator}{current}");
                    self.set("PATH", new_path);
                } else {
                    self.set("PATH", path_str.into_owned());
                }
            }
            EnvFlags::Append => {
                if let Some(current) = self.get("PATH") {
                    let new_path = format!("{current}{separator}{path_str}");
                    self.set("PATH", new_path);
                } else {
                    self.set("PATH", path_str.into_owned());
                }
            }
            EnvFlags::Replace => {
                self.set("PATH", path_str.into_owned());
            }
        }

        self
    }

    /// Returns all environment variables as a map.
    #[must_use]
    pub fn to_map(&self) -> BTreeMap<String, String> {
        self.data
            .as_ref()
            .map(|d| {
                d.vars()
                    .iter()
                    .map(|(k, v)| (k.as_str().to_owned(), v.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Returns an iterator over environment variables.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.data
            .iter()
            .flat_map(|d| d.vars().iter().map(|(k, v)| (k.as_str(), v.as_str())))
    }

    /// Returns true if no variables are set.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.as_ref().is_none_or(|d| d.vars().is_empty())
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.data.as_ref().map_or(0, |d| d.vars().len())
    }

    /// Ensures we have exclusive ownership of the data for modification.
    pub fn copy_for_write(&mut self) {
        if self.owned {
            return;
        }

        match &self.data {
            Some(data) => {
                self.data = Some(Arc::new((**data).clone()));
            }
            None => {
                self.data = Some(Arc::new(EnvData::new()));
            }
        }

        self.owned = true;
    }

    /// Creates the environment block for `CreateProcess` (Windows).
    ///
    /// Returns a vector of UTF-16 code units representing the environment block,
    /// where each variable is `KEY=VALUE\0` and the block ends with an extra `\0`.
    #[cfg(windows)]
    #[cfg(test)]
    pub(crate) fn to_windows_env_block(&self) -> Vec<u16> {
        let mut block = Vec::new();

        if let Some(data) = &self.data {
            for (key, value) in data.vars() {
                for c in key.as_str().encode_utf16() {
                    block.push(c);
                }
                block.push('=' as u16);
                for c in value.encode_utf16() {
                    block.push(c);
                }
                block.push(0);
            }
        }

        block.push(0);
        block
    }
}
