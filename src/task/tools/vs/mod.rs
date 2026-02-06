// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Visual Studio helper utilities.
//!
//! ```text
//! vs::find_latest() --> VsInstallation
//! find_msbuild() / find_devenv()
//! get_env(arch)  --> VS Dev Prompt env
//! ```
//!
//! This module provides high-level utilities for locating Visual Studio
//! installations and tools. It wraps `core::vs` for discovery and adds
//! tool-specific path resolution.
//!
//! # Windows-only
//!
//! This module is only available on Windows platforms (`cfg(windows)`).
//!
//! # Example
//!
//! ```ignore
//! use mob_rs::task::tools::vs::VsHelper;
//!
//! let msbuild = VsHelper::find_msbuild()?;
//! ```

use crate::error::Result;
use anyhow::Context;
use std::path::{Path, PathBuf};
use tracing::debug;

use crate::core::env::container::Env;
use crate::core::env::types::Arch;
use crate::core::vs::VsInstallation;

/// Visual Studio helper utilities.
pub struct VsHelper;

impl VsHelper {
    /// Find Visual Studio installations using vswhere.
    ///
    /// Returns all found VS installations sorted by version (newest first),
    /// filtered to only include complete, non-prerelease installations.
    ///
    /// # Arguments
    /// * `vswhere_override` - Optional config-provided path to vswhere.exe
    ///
    /// # Returns
    /// A vector of `VsInstallation` records, or an error if vswhere cannot be found.
    ///
    /// # Errors
    ///
    /// Returns an error if vswhere cannot be found or fails to execute.
    pub fn find_installations(vswhere_override: Option<&Path>) -> Result<Vec<VsInstallation>> {
        crate::core::vs::find_installations(vswhere_override)
    }

    /// Find the latest Visual Studio installation.
    ///
    /// Results are cached for subsequent calls (when no override is provided).
    ///
    /// # Arguments
    /// * `vswhere_override` - Optional config-provided path to vswhere.exe
    ///
    /// # Returns
    /// The latest `VsInstallation`, or an error if no installations are found.
    ///
    /// # Errors
    ///
    /// Returns an error if no Visual Studio installations are found or if discovery fails.
    pub fn find_latest(vswhere_override: Option<&Path>) -> Result<VsInstallation> {
        crate::core::vs::find_latest(vswhere_override)
    }

    /// Find the vswhere executable path.
    ///
    /// # Arguments
    /// * `override_path` - Optional config-provided path to vswhere.exe
    ///
    /// # Returns
    /// Path to vswhere.exe, or an error if not found.
    ///
    /// # Errors
    ///
    /// Returns an error if vswhere.exe cannot be located.
    pub fn find_vswhere(override_path: Option<&Path>) -> Result<PathBuf> {
        crate::core::vs::find_vswhere(override_path)
    }

    /// Find the `MSBuild` executable path.
    ///
    /// Locates MSBuild.exe in the latest VS installation.
    ///
    /// # Returns
    /// Path to MSBuild.exe, or an error if VS cannot be found.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No Visual Studio installation is found.
    /// - MSBuild.exe does not exist in the found installation.
    pub fn find_msbuild() -> Result<PathBuf> {
        debug!("Finding MSBuild executable");

        let vs = Self::find_latest(None)?;
        let msbuild = vs.msbuild_path();

        if !msbuild.exists() {
            return Err(anyhow::anyhow!(
                "MSBuild.exe not found at: {}",
                msbuild.display()
            ));
        }

        debug!(path = %msbuild.display(), "Found MSBuild");
        Ok(msbuild)
    }

    /// Find the devenv executable path (may not exist).
    ///
    /// Locates devenv.exe in the latest VS installation. Returns `Ok(None)` if
    /// VS is found but devenv is not installed (e.g., `BuildTools` SKU).
    ///
    /// # Returns
    /// `Ok(Some(path))` if devenv.exe is found, `Ok(None)` if not found but VS exists,
    /// or an error if VS cannot be found.
    ///
    /// # Errors
    ///
    /// Returns an error if no Visual Studio installation is found.
    pub fn find_devenv() -> Result<Option<PathBuf>> {
        debug!("Finding devenv executable");

        let vs = Self::find_latest(None)?;
        let devenv = vs.devenv_path();

        if devenv.exists() {
            debug!(path = %devenv.display(), "Found devenv");
            Ok(Some(devenv))
        } else {
            debug!(path = %devenv.display(), "devenv.exe not found (BuildTools SKU?)");
            Ok(None)
        }
    }

    /// Get the VS environment for a given architecture.
    ///
    /// This wraps the existing `Env::vs()` from `crate::core::env`.
    ///
    /// # Arguments
    /// * `arch` - Target architecture (x86 or x64)
    ///
    /// # Returns
    /// The VS environment variables for the given architecture.
    ///
    /// # Errors
    ///
    /// Returns an error if the Visual Studio environment cannot be captured.
    pub fn get_env(arch: Arch) -> Result<Env> {
        Env::vs(arch).context("Failed to capture VS environment")
    }
}

#[cfg(test)]
mod tests;
