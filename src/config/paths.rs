// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Path configuration.
//!
//! ```text
//! prefix/
//!   downloads/   (cache)
//!   build/
//!   install/
//!     bin/       (stylesheets, licenses, translations)
//!     lib/
//!     pdb/
//!     installer/
//! ```
//!
//! All paths are optional and resolved from `prefix` if not set.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::error::{ConfigError, Result};

/// Build and installation paths configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PathsConfig {
    /// Main build prefix (all other paths relative to this).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<PathBuf>,
    /// Download cache directory (default: prefix/downloads).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache: Option<PathBuf>,
    /// Licenses directory.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub licenses: Option<PathBuf>,
    /// Build directory (default: prefix/build).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build: Option<PathBuf>,
    /// Installation root (default: prefix/install).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install: Option<PathBuf>,
    /// Binary installation directory (default: install/bin).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_bin: Option<PathBuf>,
    /// Installer output directory (default: install/installer).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_installer: Option<PathBuf>,
    /// Library installation directory (default: install/lib).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_libs: Option<PathBuf>,
    /// PDB installation directory (default: install/pdb).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_pdbs: Option<PathBuf>,
    /// Stylesheet installation directory (default: `install_bin/stylesheets`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_stylesheets: Option<PathBuf>,
    /// License installation directory (default: `install_bin/licenses`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_licenses: Option<PathBuf>,
    /// Translation installation directory (default: `install_bin/translations`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_translations: Option<PathBuf>,
    /// vcpkg installation path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vcpkg: Option<PathBuf>,
    /// Qt installation directory.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qt_install: Option<PathBuf>,
    /// Qt bin directory (default: `qt_install/bin`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qt_bin: Option<PathBuf>,
    /// Qt translations directory (default: `qt_install/translations`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qt_translations: Option<PathBuf>,
}

impl PathsConfig {
    /// Resolve all relative paths against prefix and fill in defaults.
    ///
    /// # Errors
    ///
    /// Returns a `ConfigError::MissingKey` if the `prefix` path is not set.
    pub fn resolve(&mut self) -> Result<()> {
        let prefix = self.prefix.clone().ok_or_else(|| ConfigError::MissingKey {
            section: "paths".to_string(),
            key: "prefix".to_string(),
        })?;

        let resolve = |path: &mut Option<PathBuf>, parent: &Path, default: &str| match path {
            Some(p) if p.is_relative() => {
                *path = Some(parent.join(p.clone()));
            }
            None => {
                *path = Some(parent.join(default));
            }
            _ => {}
        };

        resolve(&mut self.cache, &prefix, "downloads");
        resolve(&mut self.build, &prefix, "build");
        resolve(&mut self.install, &prefix, "install");

        let install = self
            .install
            .clone()
            .unwrap_or_else(|| prefix.join("install"));

        resolve(&mut self.install_installer, &install, "installer");
        resolve(&mut self.install_bin, &install, "bin");
        resolve(&mut self.install_libs, &install, "lib");
        resolve(&mut self.install_pdbs, &install, "pdb");

        let install_bin = self
            .install_bin
            .clone()
            .unwrap_or_else(|| install.join("bin"));

        resolve(&mut self.install_stylesheets, &install_bin, "stylesheets");
        resolve(&mut self.install_licenses, &install_bin, "licenses");
        resolve(&mut self.install_translations, &install_bin, "translations");

        if let Some(qt_install) = &self.qt_install {
            resolve(&mut self.qt_bin, qt_install, "bin");
            resolve(&mut self.qt_translations, qt_install, "translations");
        }

        Ok(())
    }

    /// Get the prefix path, returning an error if not set.
    ///
    /// # Errors
    ///
    /// Returns a `ConfigError::MissingKey` if the `prefix` path is not set.
    pub fn prefix(&self) -> Result<&Path> {
        self.prefix.as_deref().ok_or_else(|| {
            ConfigError::MissingKey {
                section: "paths".to_string(),
                key: "prefix".to_string(),
            }
            .into()
        })
    }

    /// Returns `CMAKE_PREFIX_PATH` value by joining relevant paths.
    /// Uses semicolon on Windows, colon on Unix.
    #[must_use]
    pub fn cmake_prefix_path(&self) -> String {
        let separator = if cfg!(target_os = "windows") {
            ";"
        } else {
            ":"
        };

        [&self.qt_install, &self.vcpkg, &self.install_libs]
            .iter()
            .filter_map(|p| p.as_ref())
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(separator)
    }

    /// Returns `CMAKE_INSTALL_PREFIX` value.
    #[must_use]
    pub fn cmake_install_prefix(&self) -> Option<String> {
        self.install.as_ref().map(|p| p.display().to_string())
    }
}
