// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! `MSBuild` tool for Visual Studio solution/project builds.
//!
//! ```text
//! MsBuildTool
//! Operations: Build | Clean
//! Builder: solution/configuration/architecture/targets/properties
//! Env: VsHelper::get_env(arch)
//! Toolset: 14.3 → v143, 14.2 → v142
//! ```

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::error::Result;
use anyhow::Context;
use tracing::{debug, info};

use super::vs::VsHelper;
use super::{BoxFuture, Tool, ToolContext};
use crate::config::types::BuildConfiguration;
use crate::core::env::types::Arch;
use crate::core::process::builder::ProcessBuilder;

/// `MSBuild` operation to perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MsBuildOperation {
    /// Build the solution/project.
    #[default]
    Build,
    /// Clean the solution/project.
    Clean,
}

/// `MSBuild` tool for Visual Studio solution/project builds.
#[derive(Debug, Clone)]
pub struct MsBuildTool {
    solution: Option<PathBuf>,
    targets: Vec<String>,
    properties: BTreeMap<String, String>,
    configuration: Option<BuildConfiguration>,
    platform: Option<String>,
    architecture: Option<Arch>,
    max_cpu_count: bool,
    operation: MsBuildOperation,
}

impl MsBuildTool {
    /// Creates a new `MsBuildTool` with default settings.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            solution: None,
            targets: Vec::new(),
            properties: BTreeMap::new(),
            configuration: None,
            platform: None,
            architecture: None,
            max_cpu_count: false,
            operation: MsBuildOperation::Build,
        }
    }

    #[must_use]
    pub fn solution(mut self, path: impl Into<PathBuf>) -> Self {
        self.solution = Some(path.into());
        self
    }

    #[must_use]
    pub fn target(mut self, target: impl Into<String>) -> Self {
        self.targets.push(target.into());
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
    pub fn property(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.properties.insert(key.into(), value.into());
        self
    }

    #[must_use]
    pub const fn configuration(mut self, configuration: BuildConfiguration) -> Self {
        self.configuration = Some(configuration);
        self
    }

    #[must_use]
    pub fn platform(mut self, platform: impl Into<String>) -> Self {
        self.platform = Some(platform.into());
        self
    }

    #[must_use]
    pub const fn architecture(mut self, architecture: Arch) -> Self {
        self.architecture = Some(architecture);
        self
    }

    #[must_use]
    pub const fn max_cpu_count(mut self, enabled: bool) -> Self {
        self.max_cpu_count = enabled;
        self
    }

    #[must_use]
    pub const fn build_op(mut self) -> Self {
        self.operation = MsBuildOperation::Build;
        self
    }

    #[must_use]
    pub const fn clean_op(mut self) -> Self {
        self.operation = MsBuildOperation::Clean;
        self
    }

    fn solution_required(&self) -> Result<&Path> {
        self.solution
            .as_deref()
            .context("MsBuildTool: solution is required")
    }

    /// Converts toolset version (e.g., "14.3") to `MSBuild` format (e.g., "v143").
    ///
    /// Maps MSVC compiler versions to `PlatformToolset` values:
    /// - 14.3 -> v143 (Visual Studio 2022 17.3+)
    /// - 14.2 -> v142 (Visual Studio 2019 16.0+)
    /// - 14.1 -> v141 (Visual Studio 2017 15.0+)
    /// - 14.0 -> v140 (Visual Studio 2015)
    /// - 13.0 -> v130 (Visual Studio 2013)
    fn convert_toolset_version(version: &str) -> String {
        let parts: Vec<&str> = version.split('.').collect();
        if let Some(major) = parts.first()
            && let Ok(major_num) = major.parse::<u32>()
        {
            let minor = parts
                .get(1)
                .and_then(|m| m.parse::<u32>().ok())
                .unwrap_or(0);
            return format!("v{major_num}{minor}");
        }
        version.to_string()
    }

    /// Determines the platform to use for `MSBuild`.
    fn determine_platform(&self) -> String {
        self.platform.as_ref().map_or_else(
            || {
                self.architecture.map_or_else(
                    || "x64".to_string(),
                    |arch| match arch {
                        Arch::X86 => "Win32".to_string(),
                        Arch::X64 => "x64".to_string(),
                    },
                )
            },
            std::clone::Clone::clone,
        )
    }

    async fn do_build(&self, ctx: &ToolContext) -> Result<()> {
        if ctx.is_dry_run() {
            info!(
                solution = ?self.solution,
                configuration = ?self.configuration,
                platform = %self.determine_platform(),
                targets = ?self.targets,
                max_cpu_count = self.max_cpu_count,
                "[dry-run] Would build with MSBuild"
            );
            return Ok(());
        }

        let solution = self.solution_required()?;
        let platform = self.determine_platform();

        let arch = self.architecture.unwrap_or(Arch::X64);
        let env = VsHelper::get_env(arch)?;

        let msbuild = if ctx.config().tools.msbuild.as_os_str().is_empty() {
            VsHelper::find_msbuild().context("MSBuild executable not found")?
        } else {
            ctx.config().tools.msbuild.clone()
        };

        let mut builder = ProcessBuilder::new(&msbuild).arg("-nologo").arg(solution);

        if self.max_cpu_count {
            builder = builder
                .arg("-maxCpuCount")
                .arg("-property:UseMultiToolTask=true")
                .arg("-property:EnforceProcessCountAcrossBuilds=true");
        }

        if let Some(config) = self.configuration {
            builder = builder.arg(format!("-property:Configuration={config}"));
        }

        builder = builder.arg(format!("-property:Platform={platform}"));

        if !ctx.config().versions.vs_toolset.is_empty() {
            let toolset = Self::convert_toolset_version(&ctx.config().versions.vs_toolset);
            builder = builder.arg(format!("-property:PlatformToolset={toolset}"));
        }

        if !ctx.config().versions.sdk.is_empty() {
            builder = builder.arg(format!(
                "-property:WindowsTargetPlatformVersion={}",
                ctx.config().versions.sdk
            ));
        }

        for target in &self.targets {
            builder = builder.arg(format!("-target:{target}"));
        }

        for (key, value) in &self.properties {
            builder = builder.arg(format!("-property:{key}={value}"));
        }

        builder = builder.env(env);

        debug!("Building with MSBuild");

        let output = builder
            .run_with_cancellation(ctx.cancel_token().clone())
            .await
            .context("Failed to run MSBuild build")?;

        if output.is_interrupted() {
            anyhow::bail!("MSBuild build was interrupted");
        }

        info!("MSBuild build completed successfully");
        Ok(())
    }

    async fn do_clean(&self, ctx: &ToolContext) -> Result<()> {
        if ctx.is_dry_run() {
            info!(
                solution = ?self.solution,
                configuration = ?self.configuration,
                platform = %self.determine_platform(),
                "[dry-run] Would clean with MSBuild"
            );
            return Ok(());
        }

        let solution = self.solution_required()?;
        let platform = self.determine_platform();

        let arch = self.architecture.unwrap_or(Arch::X64);
        let env = VsHelper::get_env(arch)?;

        let msbuild = if ctx.config().tools.msbuild.as_os_str().is_empty() {
            VsHelper::find_msbuild().context("MSBuild executable not found")?
        } else {
            ctx.config().tools.msbuild.clone()
        };

        let mut builder = ProcessBuilder::new(&msbuild).arg("-nologo").arg(solution);

        if self.max_cpu_count {
            builder = builder
                .arg("-maxCpuCount")
                .arg("-property:UseMultiToolTask=true")
                .arg("-property:EnforceProcessCountAcrossBuilds=true");
        }

        if let Some(config) = self.configuration {
            builder = builder.arg(format!("-property:Configuration={config}"));
        }

        builder = builder.arg(format!("-property:Platform={platform}"));

        if !ctx.config().versions.vs_toolset.is_empty() {
            let toolset = Self::convert_toolset_version(&ctx.config().versions.vs_toolset);
            builder = builder.arg(format!("-property:PlatformToolset={toolset}"));
        }

        if !ctx.config().versions.sdk.is_empty() {
            builder = builder.arg(format!(
                "-property:WindowsTargetPlatformVersion={}",
                ctx.config().versions.sdk
            ));
        }

        if self.targets.is_empty() {
            builder = builder.arg("-target:Clean");
        } else {
            for target in &self.targets {
                builder = builder.arg(format!("-target:{target}:Clean"));
            }
        }

        for (key, value) in &self.properties {
            builder = builder.arg(format!("-property:{key}={value}"));
        }

        builder = builder.env(env);

        debug!("Cleaning with MSBuild");

        let output = builder
            .run_with_cancellation(ctx.cancel_token().clone())
            .await
            .context("Failed to run MSBuild clean")?;

        if output.is_interrupted() {
            anyhow::bail!("MSBuild clean was interrupted");
        }

        info!("MSBuild clean completed successfully");
        Ok(())
    }
}

impl Default for MsBuildTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for MsBuildTool {
    fn name(&self) -> &'static str {
        "msbuild"
    }

    fn run<'a>(&'a self, ctx: &'a ToolContext) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            match self.operation {
                MsBuildOperation::Build => self.do_build(ctx).await,
                MsBuildOperation::Clean => self.do_clean(ctx).await,
            }
        })
    }
}

#[cfg(test)]
mod tests;
