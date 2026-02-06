// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Configuration loading from multiple sources.
//!
//! # Loader Pipeline
//!
//! ```text
//! ConfigLoader::new()
//!   .add_toml_file(req)
//!   .add_toml_file(opt)
//!   .add_toml_str()
//!   .with_env_prefix()
//!   .set()
//!        |
//!        v
//!    build() --> Config
//! ```

use std::path::PathBuf;

use super::Config;
use crate::error::Result;

/// Builder for loading configuration from multiple sources.
pub struct ConfigLoader {
    builder: config::ConfigBuilder<config::builder::DefaultState>,
    env_prefix: Option<String>,
    files: Vec<(String, PathBuf)>,
}

impl ConfigLoader {
    #[must_use]
    pub fn new() -> Self {
        Self {
            builder: config::Config::builder(),
            env_prefix: None,
            files: Vec::new(),
        }
    }

    /// Adds a TOML configuration file to the loader.
    ///
    /// The file will be read when `build()` is called. If the file doesn't exist
    /// or contains invalid TOML, `build()` will return an error.
    #[must_use]
    pub fn add_toml_file<P: AsRef<std::path::Path>>(mut self, path: P) -> Self {
        use config::{File, FileFormat};
        let p = path.as_ref();
        self.builder = self
            .builder
            .add_source(File::from(p).format(FileFormat::Toml).required(true));
        self.files.push(("file".to_string(), p.to_path_buf()));
        self
    }

    #[must_use]
    pub fn add_toml_file_optional<P: AsRef<std::path::Path>>(mut self, path: P) -> Self {
        use config::{File, FileFormat};
        let p = path.as_ref();
        self.builder = self
            .builder
            .add_source(File::from(p).format(FileFormat::Toml).required(false));
        if p.exists() {
            self.files.push(("optional".to_string(), p.to_path_buf()));
        }
        self
    }

    #[must_use]
    pub fn add_toml_str(mut self, content: &str) -> Self {
        use config::{File, FileFormat};
        self.builder = self
            .builder
            .add_source(File::from_str(content, FileFormat::Toml));
        self.files
            .push(("string".to_string(), PathBuf::from("<string>")));
        self
    }

    #[must_use]
    pub fn with_env_prefix(mut self, prefix: &str) -> Self {
        self.env_prefix = Some(prefix.to_string());
        self
    }

    /// Sets a configuration override.
    ///
    /// # Errors
    ///
    /// Returns an error if the key is invalid or if the value cannot be converted
    /// to a configuration value.
    pub fn set<T: Into<config::Value>>(mut self, key: &str, value: T) -> Result<Self> {
        self.builder = self
            .builder
            .set_override(key, value)
            .map_err(|e| anyhow::anyhow!("Config error: {e}"))?;
        Ok(self)
    }

    /// Builds the configuration from all added sources.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Required configuration files are missing.
    /// - Configuration files have invalid TOML syntax.
    /// - Environment variables cannot be parsed.
    /// - The merged configuration cannot be deserialized into the `Config` struct.
    pub fn build(self) -> Result<Config> {
        let builder = match &self.env_prefix {
            Some(prefix) => self.builder.add_source(
                config::Environment::with_prefix(prefix)
                    .separator("_")
                    .try_parsing(true),
            ),
            None => self.builder,
        };
        let cfg = builder.build()?;
        let mut config: Config = cfg.try_deserialize()?;
        config.resolve_and_validate()?;
        Ok(config)
    }

    #[must_use]
    pub fn loaded_files(&self) -> Vec<(String, PathBuf)> {
        self.files.clone()
    }

    #[must_use]
    pub fn format_loaded_files(&self) -> Vec<String> {
        self.files
            .iter()
            .enumerate()
            .map(|(i, (source, path))| format!("{}. [{}] {}", i + 1, source, path.display()))
            .collect()
    }
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}
