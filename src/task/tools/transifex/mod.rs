// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Transifex CLI tool for translation management.
//!
//! ```text
//! TransifexTool
//! Operations: Init → Config → Pull
//! root/.tx/config created by tx init + tx add remote
//! Builder: root/api_key/url/minimum/force
//! ```
//!
//! This module provides the `TransifexTool` struct for executing Transifex CLI operations
//! like init, config, and pull with cancellation support.
//!
//! # Architecture
//!
//! The Transifex CLI (`tx`) is used to:
//! - Initialize a transifex directory (`tx init`)
//! - Configure the API URL and remote (`tx add remote`)
//! - Pull translations (`tx pull`)
//!
//! The tool expects the `tx` executable to be available in the configured tools path.

use std::path::{Path, PathBuf};

use crate::error::Result;
use anyhow::Context;
use tracing::{debug, info};

use super::{BoxFuture, Tool, ToolContext};
use crate::core::env::current_env;
use crate::core::process::builder::ProcessBuilder;

/// Operation to perform with the Transifex CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TransifexOperation {
    #[default]
    Init,
    Config,
    Pull,
}

/// Transifex CLI tool for translation management.
///
/// # Example
///
/// ```ignore
/// // Initialize transifex directory
/// let tool = TransifexTool::new()
///     .root("./build/transifex-translations")
///     .init_op();
/// tool.run(&ctx).await?;
///
/// // Configure the remote URL
/// let tool = TransifexTool::new()
///     .root("./build/transifex-translations")
///     .api_key("your-api-key")
///     .url("https://app.transifex.com/org/project/dashboard")
///     .config_op();
/// tool.run(&ctx).await?;
///
/// // Pull translations
/// let tool = TransifexTool::new()
///     .root("./build/transifex-translations")
///     .api_key("your-api-key")
///     .minimum(60)
///     .force(true)
///     .pull_op();
/// tool.run(&ctx).await?;
/// ```
#[derive(Debug, Clone)]
pub struct TransifexTool {
    root: Option<PathBuf>,
    api_key: Option<String>,
    url: Option<String>,
    minimum: u8,
    force: bool,
    operation: TransifexOperation,
    tx_binary: Option<PathBuf>,
}

impl Default for TransifexTool {
    fn default() -> Self {
        Self::new()
    }
}

impl TransifexTool {
    /// Creates a new `TransifexTool` with default settings.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            root: None,
            api_key: None,
            url: None,
            minimum: 100,
            force: false,
            operation: TransifexOperation::Init,
            tx_binary: None,
        }
    }

    #[must_use]
    pub fn root(mut self, root: impl AsRef<Path>) -> Self {
        self.root = Some(root.as_ref().to_path_buf());
        self
    }

    #[must_use]
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    #[must_use]
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    #[must_use]
    pub fn minimum(mut self, percent: u8) -> Self {
        self.minimum = percent.min(100);
        self
    }

    #[must_use]
    pub const fn force(mut self, force: bool) -> Self {
        self.force = force;
        self
    }

    #[must_use]
    pub fn tx_binary(mut self, path: impl AsRef<Path>) -> Self {
        self.tx_binary = Some(path.as_ref().to_path_buf());
        self
    }

    #[must_use]
    pub const fn init_op(mut self) -> Self {
        self.operation = TransifexOperation::Init;
        self
    }

    #[must_use]
    pub const fn config_op(mut self) -> Self {
        self.operation = TransifexOperation::Config;
        self
    }

    #[must_use]
    pub const fn pull_op(mut self) -> Self {
        self.operation = TransifexOperation::Pull;
        self
    }

    fn get_tx_binary(&self, ctx: &ToolContext) -> Result<PathBuf> {
        if let Some(ref binary) = self.tx_binary {
            return Ok(binary.clone());
        }

        let config_path = &ctx.config().tools.tx;
        if config_path.is_absolute() && config_path.exists() {
            return Ok(config_path.clone());
        }

        ProcessBuilder::find("tx")
            .or_else(|| ProcessBuilder::find("tx.exe"))
            .context("tx executable not found in PATH or config")
    }

    async fn do_init(&self, ctx: &ToolContext) -> Result<()> {
        let root = self
            .root
            .as_ref()
            .context("TransifexTool: root is required for init")?;

        if ctx.is_dry_run() {
            info!(
                path = %root.display(),
                "[dry-run] Would initialize transifex directory"
            );
            return Ok(());
        }

        if !root.exists() {
            tokio::fs::create_dir_all(root)
                .await
                .with_context(|| format!("Failed to create directory: {}", root.display()))?;
        }

        let tx_binary = self.get_tx_binary(ctx)?;

        // tx init - exit code 2 means directory already initialized
        debug!(path = %root.display(), "Initializing transifex directory");

        let output = ProcessBuilder::new(&tx_binary)
            .arg("init")
            .cwd(root)
            .success_codes([0, 2])
            .run_with_cancellation(ctx.cancel_token().clone())
            .await
            .context("Failed to run tx init")?;

        if output.is_interrupted() {
            anyhow::bail!("tx init was interrupted");
        }

        info!(path = %root.display(), "Transifex directory initialized");

        Ok(())
    }

    async fn do_config(&self, ctx: &ToolContext) -> Result<()> {
        let root = self
            .root
            .as_ref()
            .context("TransifexTool: root is required for config")?;
        let url = self
            .url
            .as_ref()
            .context("TransifexTool: url is required for config")?;

        if ctx.is_dry_run() {
            info!(
                path = %root.display(),
                url = %url,
                "[dry-run] Would configure transifex remote"
            );
            return Ok(());
        }

        if !root.exists() {
            tokio::fs::create_dir_all(root)
                .await
                .with_context(|| format!("Failed to create directory: {}", root.display()))?;
        }

        let tx_binary = self.get_tx_binary(ctx)?;

        let mut builder = ProcessBuilder::new(&tx_binary)
            .arg("add")
            .arg("remote")
            .arg(url)
            .cwd(root);

        if let Some(ref key) = self.api_key {
            let mut env = current_env();
            env.set("TX_TOKEN", key);
            builder = builder.env(env);
        }

        debug!(path = %root.display(), url = %url, "Configuring transifex remote");

        let output = builder
            .run_with_cancellation(ctx.cancel_token().clone())
            .await
            .context("Failed to run tx add remote")?;

        if output.is_interrupted() {
            anyhow::bail!("tx config was interrupted");
        }

        info!(path = %root.display(), "Transifex remote configured");

        Ok(())
    }

    async fn do_pull(&self, ctx: &ToolContext) -> Result<()> {
        let root = self
            .root
            .as_ref()
            .context("TransifexTool: root is required for pull")?;

        if ctx.is_dry_run() {
            info!(
                path = %root.display(),
                minimum = self.minimum,
                force = self.force,
                "[dry-run] Would pull translations"
            );
            return Ok(());
        }

        if !root.exists() {
            tokio::fs::create_dir_all(root)
                .await
                .with_context(|| format!("Failed to create directory: {}", root.display()))?;
        }

        let tx_binary = self.get_tx_binary(ctx)?;

        let mut builder = ProcessBuilder::new(&tx_binary)
            .arg("pull")
            .arg("--all")
            .arg("--minimum-perc")
            .arg(self.minimum.to_string())
            .cwd(root);

        if self.force {
            builder = builder.arg("--force");
        }

        if let Some(ref key) = self.api_key {
            let mut env = current_env();
            env.set("TX_TOKEN", key);
            builder = builder.env(env);
        }

        debug!(
            path = %root.display(),
            minimum = self.minimum,
            force = self.force,
            "Pulling translations"
        );

        let output = builder
            .run_with_cancellation(ctx.cancel_token().clone())
            .await
            .context("Failed to run tx pull")?;

        if output.is_interrupted() {
            anyhow::bail!("tx pull was interrupted");
        }

        info!(path = %root.display(), "Translations pulled successfully");

        Ok(())
    }
}

impl Tool for TransifexTool {
    fn name(&self) -> &'static str {
        "transifex"
    }

    fn run<'a>(&'a self, ctx: &'a ToolContext) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            match self.operation {
                TransifexOperation::Init => self.do_init(ctx).await,
                TransifexOperation::Config => self.do_config(ctx).await,
                TransifexOperation::Pull => self.do_pull(ctx).await,
            }
        })
    }
}

#[cfg(test)]
mod tests;
