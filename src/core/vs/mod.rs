// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Visual Studio installation discovery via vswhere.
//!
//! ```text
//! vswhere.exe --> find_installations() --> VsInstallation
//!   instance_id, path, version, display_name, flags
//!   derived: devshell_dll(), msbuild_path(), devenv_path()
//! ```

use crate::error::Result;
use anyhow::Context;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use tracing::{debug, trace};

/// Standard vswhere.exe installation paths.
const VSWHERE_PATHS: &[&str] = &[
    r"C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe",
    r"C:\Program Files\Microsoft Visual Studio\Installer\vswhere.exe",
];

/// Global cache for vswhere path discovery.
static VSWHERE_PATH: OnceLock<std::result::Result<PathBuf, String>> = OnceLock::new();

/// Global cache for latest VS installation.
static LATEST_INSTALLATION: OnceLock<std::result::Result<VsInstallation, String>> = OnceLock::new();

/// Visual Studio installation information from vswhere JSON output.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VsInstallation {
    /// Unique instance identifier (used by Enter-VsDevShell).
    pub instance_id: String,

    /// Installation root path.
    pub installation_path: PathBuf,

    /// Full version string (e.g., "17.14.36915.13").
    pub installation_version: String,

    /// Human-readable display name.
    pub display_name: String,

    /// Whether the installation is complete (no errors or reboot required).
    #[serde(default = "default_true")]
    pub is_complete: bool,

    /// Whether this is a prerelease version.
    #[serde(default)]
    pub is_prerelease: bool,
}

const fn default_true() -> bool {
    true
}

impl VsInstallation {
    /// Path to Microsoft.VisualStudio.DevShell.dll (for Enter-VsDevShell cmdlet).
    #[must_use]
    pub fn devshell_dll(&self) -> PathBuf {
        self.installation_path
            .join("Common7")
            .join("Tools")
            .join("Microsoft.VisualStudio.DevShell.dll")
    }

    /// Path to MSBuild.exe.
    #[must_use]
    pub fn msbuild_path(&self) -> PathBuf {
        self.installation_path
            .join("MSBuild")
            .join("Current")
            .join("Bin")
            .join("MSBuild.exe")
    }

    /// Path to devenv.exe (Visual Studio IDE).
    #[must_use]
    pub fn devenv_path(&self) -> PathBuf {
        self.installation_path
            .join("Common7")
            .join("IDE")
            .join("devenv.exe")
    }

    /// Parse version into numeric components (major, minor, patch, build).
    ///
    /// Numeric comparison correctly handles "17.14" > "17.9", unlike string comparison.
    fn version_tuple(&self) -> (u32, u32, u32, u32) {
        let parts: Vec<u32> = self
            .installation_version
            .split('.')
            .filter_map(|s| s.parse().ok())
            .collect();

        (
            parts.first().copied().unwrap_or(0),
            parts.get(1).copied().unwrap_or(0),
            parts.get(2).copied().unwrap_or(0),
            parts.get(3).copied().unwrap_or(0),
        )
    }
}

/// Finds the vswhere.exe executable path.
///
/// Checks optional override path, then standard Visual Studio Installer directories.
/// Results are cached for subsequent calls (when no override provided).
///
/// # Errors
///
/// Returns an error if `vswhere.exe` cannot be found in the override path or standard
/// locations.
pub fn find_vswhere(override_path: Option<&Path>) -> Result<PathBuf> {
    // Use override path if provided and exists
    if let Some(path) = override_path {
        if path.exists() {
            trace!(path = %path.display(), "Using config-provided vswhere");
            return Ok(path.to_path_buf());
        }
        debug!(
            path = %path.display(),
            "Config vswhere path not found, searching standard locations"
        );
    }

    // Cache standard path search
    VSWHERE_PATH
        .get_or_init(|| find_vswhere_impl().map_err(|e| e.to_string()))
        .clone()
        .map_err(|e| anyhow::anyhow!(e))
}

fn find_vswhere_impl() -> Result<PathBuf> {
    trace!("Searching for vswhere.exe in standard locations");

    for candidate in VSWHERE_PATHS {
        let path = PathBuf::from(candidate);
        if path.exists() {
            debug!(path = %path.display(), "Found vswhere");
            return Ok(path);
        }
    }

    Err(anyhow::anyhow!(
        "vswhere.exe not found in standard VS Installer directories"
    ))
}

/// Finds all Visual Studio installations with C++ tools.
///
/// Returns sorted by version (newest first), filtered to complete, non-prerelease only.
///
/// # Errors
///
/// Returns an error if:
/// - `vswhere.exe` cannot be found.
/// - The `vswhere` command fails to execute.
/// - The JSON output from `vswhere` cannot be parsed.
pub fn find_installations(vswhere_override: Option<&Path>) -> Result<Vec<VsInstallation>> {
    debug!("Finding Visual Studio installations via vswhere");

    let vswhere =
        find_vswhere(vswhere_override).context("Cannot find Visual Studio installations")?;

    let output = Command::new(&vswhere)
        .args([
            "-format",
            "json",
            "-utf8",
            "-products",
            "*",
            "-requires",
            "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .context("Failed to run vswhere")?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "vswhere failed with exit code {:?}",
            output.status.code()
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut installations = parse_vswhere_json(&stdout)?;

    installations.retain(|vs| vs.is_complete && !vs.is_prerelease);
    installations.sort_by_key(|vs| std::cmp::Reverse(vs.version_tuple()));

    debug!(
        count = installations.len(),
        "Found Visual Studio installations"
    );

    Ok(installations)
}

/// Finds the latest Visual Studio installation with C++ tools.
///
/// Results are cached for subsequent calls (when no override provided).
///
/// # Errors
///
/// Returns an error if no Visual Studio installations with C++ tools are found.
pub fn find_latest(vswhere_override: Option<&Path>) -> Result<VsInstallation> {
    if vswhere_override.is_some() {
        return find_installations(vswhere_override)?
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No Visual Studio installations found with C++ tools"));
    }

    LATEST_INSTALLATION
        .get_or_init(|| find_latest_impl().map_err(|e| e.to_string()))
        .clone()
        .map_err(|e| anyhow::anyhow!(e))
}

fn find_latest_impl() -> Result<VsInstallation> {
    find_installations(None)?
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("No Visual Studio installations found with C++ tools"))
}

/// Parses vswhere JSON output into `VsInstallation` structs.
fn parse_vswhere_json(json: &str) -> Result<Vec<VsInstallation>> {
    let installations: Vec<VsInstallation> =
        serde_json::from_str(json).context("Failed to parse vswhere JSON output")?;
    Ok(installations)
}

#[cfg(test)]
mod tests;
