// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! `CMake` tool for configure/build/install operations.
//!
//! ```text
//! CmakeTool
//! Operations: Configure | Build | Install | Clean
//! Builder: source_dir/build_dir/generator/architecture/definition
//! Generators: VisualStudio 17, Ninja, NMake JOM
//! Architectures: X86 (Win32), X64
//! ```

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::error::Result;
use anyhow::Context;
use tokio::fs;
use tracing::{debug, info};

use super::{BoxFuture, Tool, ToolContext};
use crate::config::types::BuildConfiguration;
use crate::core::process::builder::ProcessBuilder;

/// `CMake` generator to use for configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmakeGenerator {
    /// Visual Studio generator.
    VisualStudio,
    /// Ninja generator.
    Ninja,
    /// `NMake` JOM generator.
    NMakeJom,
}

impl CmakeGenerator {
    const fn as_str(self) -> &'static str {
        match self {
            Self::VisualStudio => "Visual Studio 17 2022",
            Self::Ninja => "Ninja",
            Self::NMakeJom => "NMake Makefiles JOM",
        }
    }
}

/// Target architecture for `CMake` (-A option).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmakeArchitecture {
    /// 32-bit x86 (Win32).
    X86,
    /// 64-bit x86-64.
    X64,
}

impl CmakeArchitecture {
    const fn as_str(self) -> &'static str {
        match self {
            Self::X86 => "Win32",
            Self::X64 => "x64",
        }
    }
}

/// `CMake` operation to perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CmakeOperation {
    /// Configure a `CMake` build directory.
    #[default]
    Configure,
    /// Build targets in a configured build directory.
    Build,
    /// Install artifacts from a build directory.
    Install,
    /// Clean the build directory.
    Clean,
}

/// `CMake` tool for configure/build/install operations.
#[derive(Debug, Clone)]
pub struct CmakeTool {
    source_dir: Option<PathBuf>,
    build_dir: Option<PathBuf>,
    install_prefix: Option<PathBuf>,
    generator: Option<CmakeGenerator>,
    architecture: Option<CmakeArchitecture>,
    configuration: Option<BuildConfiguration>,
    definitions: BTreeMap<String, String>,
    prefix_paths: Vec<PathBuf>,
    target: Option<String>,
    targets: Vec<String>,
    preset: Option<String>,
    operation: CmakeOperation,
}

impl CmakeTool {
    /// Creates a new `CmakeTool` with default settings.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            source_dir: None,
            build_dir: None,
            install_prefix: None,
            generator: None,
            architecture: None,
            configuration: None,
            definitions: BTreeMap::new(),
            prefix_paths: Vec::new(),
            target: None,
            targets: Vec::new(),
            preset: None,
            operation: CmakeOperation::Configure,
        }
    }

    #[must_use]
    pub fn source_dir(mut self, path: impl AsRef<Path>) -> Self {
        self.source_dir = Some(path.as_ref().to_path_buf());
        self
    }

    #[must_use]
    pub fn build_dir(mut self, path: impl AsRef<Path>) -> Self {
        self.build_dir = Some(path.as_ref().to_path_buf());
        self
    }

    #[must_use]
    pub fn install_prefix(mut self, path: impl AsRef<Path>) -> Self {
        self.install_prefix = Some(path.as_ref().to_path_buf());
        self
    }

    #[must_use]
    pub const fn generator(mut self, generator: CmakeGenerator) -> Self {
        self.generator = Some(generator);
        self
    }

    #[must_use]
    pub const fn architecture(mut self, architecture: CmakeArchitecture) -> Self {
        self.architecture = Some(architecture);
        self
    }

    #[must_use]
    pub const fn configuration(mut self, configuration: BuildConfiguration) -> Self {
        self.configuration = Some(configuration);
        self
    }

    #[must_use]
    pub fn definition(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.definitions.insert(key.into(), value.into());
        self
    }

    #[must_use]
    pub fn prefix_path<I, P>(mut self, paths: I) -> Self
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        self.prefix_paths = paths
            .into_iter()
            .map(|p| p.as_ref().to_path_buf())
            .collect();
        self
    }

    #[must_use]
    pub fn target(mut self, target: impl Into<String>) -> Self {
        self.target = Some(target.into());
        self
    }

    #[must_use]
    pub fn targets<I, S>(mut self, targets: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.targets = targets.into_iter().map(Into::into).collect();
        self
    }

    #[must_use]
    pub fn preset(mut self, preset: impl Into<String>) -> Self {
        self.preset = Some(preset.into());
        self
    }

    #[must_use]
    pub const fn configure_op(mut self) -> Self {
        self.operation = CmakeOperation::Configure;
        self
    }

    #[must_use]
    pub const fn build_op(mut self) -> Self {
        self.operation = CmakeOperation::Build;
        self
    }

    #[must_use]
    pub const fn install_op(mut self) -> Self {
        self.operation = CmakeOperation::Install;
        self
    }

    #[must_use]
    pub const fn clean_op(mut self) -> Self {
        self.operation = CmakeOperation::Clean;
        self
    }

    fn build_dir_required(&self) -> Result<&Path> {
        self.build_dir
            .as_deref()
            .context("CmakeTool: build_dir is required")
    }

    fn source_dir_required(&self) -> Result<&Path> {
        self.source_dir
            .as_deref()
            .context("CmakeTool: source_dir is required")
    }

    fn prefix_path_value(&self) -> Option<String> {
        if self.prefix_paths.is_empty() {
            return None;
        }

        let separator = if cfg!(target_os = "windows") {
            ";"
        } else {
            ":"
        };
        let value = self
            .prefix_paths
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(separator);
        Some(value)
    }

    fn cmake_builder(ctx: &ToolContext) -> Result<ProcessBuilder> {
        if ctx.config().tools.cmake.as_os_str().is_empty() {
            ProcessBuilder::which("cmake").context("cmake executable not found")
        } else {
            Ok(ProcessBuilder::new(&ctx.config().tools.cmake))
        }
    }

    fn combined_targets(&self) -> Vec<String> {
        let mut targets = BTreeSet::new();
        if let Some(ref target) = self.target {
            targets.insert(target.clone());
        }
        for target in &self.targets {
            targets.insert(target.clone());
        }
        targets.into_iter().collect()
    }

    async fn do_configure(&self, ctx: &ToolContext) -> Result<()> {
        let mut definitions = self.definitions.clone();

        definitions
            .entry("CMAKE_INSTALL_MESSAGE".to_string())
            .or_insert_with(|| ctx.config().cmake.install_message.to_string());

        if let Some(ref prefix) = self.install_prefix {
            definitions
                .entry("CMAKE_INSTALL_PREFIX".to_string())
                .or_insert_with(|| prefix.display().to_string());
        }

        if let Some(prefix_path) = self.prefix_path_value() {
            definitions
                .entry("CMAKE_PREFIX_PATH".to_string())
                .or_insert(prefix_path);
        }

        if ctx.is_dry_run() {
            info!(
                source = ?self.source_dir,
                build = ?self.build_dir,
                generator = self.generator.map(CmakeGenerator::as_str),
                architecture = self.architecture.map(CmakeArchitecture::as_str),
                preset = ?self.preset,
                definitions = ?definitions,
                "[dry-run] Would configure CMake"
            );
            return Ok(());
        }

        let mut builder = Self::cmake_builder(ctx)?;

        if let Some(ref preset) = self.preset {
            builder = builder.arg("--preset").arg(preset);
        } else {
            let source = self.source_dir_required()?;
            let build = self.build_dir_required()?;

            builder = builder.arg("-S").arg(source).arg("-B").arg(build);

            if let Some(generator) = self.generator {
                builder = builder.arg("-G").arg(generator.as_str());
            }

            if let Some(architecture) = self.architecture {
                builder = builder.arg("-A").arg(architecture.as_str());
            }

            if !ctx.config().cmake.host.is_empty() {
                builder = builder
                    .arg("-T")
                    .arg(format!("host={}", ctx.config().cmake.host));
            }
        }

        for (key, value) in definitions {
            builder = builder.arg(format!("-D{key}={value}"));
        }

        debug!("Configuring CMake");

        let output = builder
            .run_with_cancellation(ctx.cancel_token().clone())
            .await
            .context("Failed to run CMake configure")?;

        if output.is_interrupted() {
            anyhow::bail!("CMake configure was interrupted");
        }

        info!("CMake configure completed successfully");
        Ok(())
    }

    async fn do_build(&self, ctx: &ToolContext) -> Result<()> {
        let targets = self.combined_targets();

        if ctx.is_dry_run() {
            info!(
                build = ?self.build_dir,
                configuration = ?self.configuration,
                preset = ?self.preset,
                targets = ?targets,
                "[dry-run] Would build with CMake"
            );
            return Ok(());
        }

        let mut builder = Self::cmake_builder(ctx)?.arg("--build");

        if let Some(ref preset) = self.preset {
            builder = builder.arg("--preset").arg(preset);
        } else {
            let build = self.build_dir_required()?;
            builder = builder.arg(build);
        }

        if let Some(configuration) = self.configuration {
            builder = builder.arg("--config").arg(configuration.to_string());
        }

        for target in targets {
            builder = builder.arg("--target").arg(target);
        }

        builder = builder.arg("--parallel");

        debug!("Building with CMake");

        let output = builder
            .run_with_cancellation(ctx.cancel_token().clone())
            .await
            .context("Failed to run CMake build")?;

        if output.is_interrupted() {
            anyhow::bail!("CMake build was interrupted");
        }

        info!("CMake build completed successfully");
        Ok(())
    }

    async fn do_install(&self, ctx: &ToolContext) -> Result<()> {
        if ctx.is_dry_run() {
            info!(
                build = ?self.build_dir,
                configuration = ?self.configuration,
                preset = ?self.preset,
                prefix = ?self.install_prefix,
                "[dry-run] Would install with CMake"
            );
            return Ok(());
        }

        let mut builder = Self::cmake_builder(ctx)?.arg("--install");

        if let Some(ref preset) = self.preset {
            builder = builder.arg("--preset").arg(preset);
        } else {
            let build = self.build_dir_required()?;
            builder = builder.arg(build);
        }

        if let Some(configuration) = self.configuration {
            builder = builder.arg("--config").arg(configuration.to_string());
        }

        if let Some(ref prefix) = self.install_prefix {
            builder = builder.arg("--prefix").arg(prefix);
        }

        debug!("Installing with CMake");

        let output = builder
            .run_with_cancellation(ctx.cancel_token().clone())
            .await
            .context("Failed to run CMake install")?;

        if output.is_interrupted() {
            anyhow::bail!("CMake install was interrupted");
        }

        info!("CMake install completed successfully");
        Ok(())
    }

    async fn do_clean(&self, ctx: &ToolContext) -> Result<()> {
        let build = self.build_dir_required()?;

        if ctx.is_dry_run() {
            info!(build = %build.display(), "[dry-run] Would clean build directory");
            return Ok(());
        }

        if build.exists() {
            fs::remove_dir_all(build)
                .await
                .with_context(|| format!("Failed to clean build directory: {}", build.display()))?;
        }

        info!(build = %build.display(), "Build directory cleaned");
        Ok(())
    }
}

impl Default for CmakeTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for CmakeTool {
    fn name(&self) -> &'static str {
        "cmake"
    }

    fn run<'a>(&'a self, ctx: &'a ToolContext) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            match self.operation {
                CmakeOperation::Configure => self.do_configure(ctx).await,
                CmakeOperation::Build => self.do_build(ctx).await,
                CmakeOperation::Install => self.do_install(ctx).await,
                CmakeOperation::Clean => self.do_clean(ctx).await,
            }
        })
    }
}

#[cfg(test)]
mod tests;
