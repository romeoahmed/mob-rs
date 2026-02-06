// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! USVFS task implementation.
//!
//! ```text
//! UsvfsTask (dual-arch)
//! Pipeline: Fetch → CMake configure → MSBuild → Install
//! build/usvfs/vsbuild64 + vsbuild32 (usvfs.sln)
//! ```
//!
//! This task builds the USVFS (User-Space Virtual File System) component,
//! which is responsible for file system virtualization in Mod Organizer.
//!
//! USVFS must be built for both x86 and x64 architectures, as the 32-bit
//! version is needed for 32-bit applications and the 64-bit version for
//! 64-bit applications.
//!
//! # Build Process
//!
//! 1. **Fetch**: Clone the usvfs repository
//! 2. **Configure**: Run `CMake` with VS generator for both architectures
//! 3. **Build**: Use `MSBuild` to build both architectures
//!
//! # Phases
//!
//! - **Clean**: Remove build directories or source directory
//! - **Fetch**: Git clone/pull the repository
//! - **`BuildAndInstall`**: `CMake` configure + `MSBuild` for x86 and x64

use std::path::PathBuf;

use crate::error::Result;
use anyhow::Context;
use futures_util::future::BoxFuture;
use tracing::{debug, info};

use crate::config::Config;
use crate::core::env::types::Arch;
use crate::task::helpers::safe_remove_source;
use crate::task::tools::Tool;
use crate::task::tools::cmake::{CmakeGenerator, CmakeTool};
use crate::task::tools::git::GitTool;
use crate::task::tools::msbuild::MsBuildTool;
use crate::task::{CleanFlags, TaskContext, Taskable};

/// USVFS task for building the User-Space Virtual File System.
///
/// This task builds USVFS for both x86 and x64 architectures.
/// It uses `CMake` to generate Visual Studio projects and then
/// builds them using `MSBuild`.
///
/// # Example
///
/// ```ignore
/// let task = UsvfsTask::new();
/// task.run(&ctx).await?;
/// ```
#[derive(Debug, Clone)]
pub struct UsvfsTask {
    /// The task name
    name: String,
}

impl Default for UsvfsTask {
    fn default() -> Self {
        Self::new()
    }
}

impl UsvfsTask {
    /// Create a new USVFS task.
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: "usvfs".to_string(),
        }
    }

    /// Returns the task name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the git URL for the USVFS repository.
    fn git_url(config: &Config) -> String {
        format!(
            "{}{}/usvfs.git",
            config.task.git_url_prefix, config.task.mo_org
        )
    }

    /// Returns the source directory path.
    fn source_path(config: &Config) -> Result<PathBuf> {
        let build_dir = config
            .paths
            .build
            .as_ref()
            .context("paths.build not configured")?;
        Ok(build_dir.join("usvfs"))
    }

    /// Returns the build directory for a specific architecture.
    fn build_dir(config: &Config, arch: Arch) -> Result<PathBuf> {
        let source = Self::source_path(config)?;
        let dir_name = match arch {
            Arch::X64 => "vsbuild64",
            Arch::X86 => "vsbuild32",
        };
        Ok(source.join(dir_name))
    }

    /// Returns the solution file path for a specific architecture.
    fn solution_path(config: &Config, arch: Arch) -> Result<PathBuf> {
        let build_dir = Self::build_dir(config, arch)?;
        Ok(build_dir.join("usvfs.sln"))
    }

    /// Returns the version/branch to use.
    fn version(config: &Config) -> String {
        config.versions.usvfs.clone()
    }

    /// Returns the `CMake` preset for a specific architecture.
    const fn cmake_preset(arch: Arch) -> &'static str {
        match arch {
            Arch::X64 => "vs2022-windows-x64",
            Arch::X86 => "vs2022-windows-x86",
        }
    }

    /// Execute the clean phase.
    ///
    /// # Errors
    ///
    /// Returns an error if the source directory cannot be removed, or if the
    /// `CMake` or `MSBuild` clean operations fail.
    pub async fn do_clean(&self, ctx: &TaskContext, flags: CleanFlags) -> Result<()> {
        let config = ctx.config();
        let source_path = Self::source_path(config)?;

        if flags.contains(CleanFlags::REEXTRACT) {
            // Remove entire source directory (reclone)
            safe_remove_source(ctx, &source_path, "source directory").await?;
            return Ok(());
        }

        let tool_ctx = ctx.tool_context();

        // Reconfigure: clean cmake cache for both architectures
        if flags.contains(CleanFlags::RECONFIGURE) {
            for arch in [Arch::X64, Arch::X86] {
                let build_dir = Self::build_dir(config, arch)?;
                if build_dir.exists() {
                    let cmake = CmakeTool::new()
                        .source_dir(&source_path)
                        .build_dir(&build_dir)
                        .clean_op();

                    if ctx.is_dry_run() {
                        info!(
                            arch = ?arch,
                            path = %build_dir.display(),
                            "[DRY-RUN] would clean cmake cache"
                        );
                    } else {
                        cmake
                            .run(&tool_ctx)
                            .await
                            .with_context(|| format!("failed to clean cmake cache for {arch:?}"))?;
                    }
                }
            }
        }

        // Rebuild: clean with MSBuild for both architectures
        if flags.contains(CleanFlags::REBUILD) {
            let task_config = config.task_config(&self.name);

            for arch in [Arch::X64, Arch::X86] {
                let solution = Self::solution_path(config, arch)?;
                if solution.exists() {
                    let msbuild = MsBuildTool::new()
                        .solution(&solution)
                        .architecture(arch)
                        .configuration(task_config.configuration)
                        .max_cpu_count(true)
                        .clean_op();

                    msbuild
                        .run(&tool_ctx)
                        .await
                        .with_context(|| format!("failed to clean {arch:?} build"))?;
                }
            }
        }

        Ok(())
    }

    /// Execute the fetch phase.
    ///
    /// # Errors
    ///
    /// Returns an error if the repository cannot be cloned or pulled, or if
    /// submodules cannot be updated.
    pub async fn do_fetch(&self, ctx: &TaskContext) -> Result<()> {
        let config = ctx.config();
        let task_config = config.task_config(&self.name);
        let source_path = Self::source_path(config)?;
        let git_url = Self::git_url(config);
        let branch = Self::version(config);

        let tool_ctx = ctx.tool_context();

        if source_path.exists() {
            // Pull existing repo
            if task_config.git_behavior.no_pull {
                debug!(path = %source_path.display(), "Skipping pull (no_pull=true)");
                return Ok(());
            }

            info!(
                repo = "usvfs",
                branch = %branch,
                "Pulling updates"
            );

            let git = GitTool::new().path(&source_path).branch(&branch).pull_op();

            git.run(&tool_ctx).await.context("failed to pull usvfs")?;
        } else {
            // Clone new repo
            info!(
                repo = "usvfs",
                url = %git_url,
                branch = %branch,
                "Cloning repository"
            );

            let mut git = GitTool::new()
                .url(&git_url)
                .path(&source_path)
                .branch(&branch)
                .clone_op();

            if task_config.git_clone.git_shallow {
                git = git.shallow(true);
            }

            git.run(&tool_ctx).await.context("failed to clone usvfs")?;
        }

        // Update submodules if present
        let gitmodules = source_path.join(".gitmodules");
        if gitmodules.exists() {
            debug!(repo = "usvfs", "Updating submodules");

            let git = GitTool::new().path(&source_path).submodule_update_op();

            git.run(&tool_ctx)
                .await
                .context("failed to update submodules for usvfs")?;
        }

        Ok(())
    }

    /// Execute the build and install phase.
    ///
    /// # Errors
    ///
    /// Returns an error if the `CMake` configuration fails or if the `MSBuild`
    /// build fails for either architecture.
    pub async fn do_build_and_install(&self, ctx: &TaskContext) -> Result<()> {
        let config = ctx.config();
        let task_config = config.task_config(&self.name);
        let source_path = Self::source_path(config)?;
        let install_prefix = config
            .paths
            .install
            .as_ref()
            .context("paths.install not configured")?;

        let tool_ctx = ctx.tool_context();

        // Configure and build for both architectures
        for arch in [Arch::X64, Arch::X86] {
            let build_dir = Self::build_dir(config, arch)?;
            let preset = Self::cmake_preset(arch);

            // CMake configure
            info!(
                repo = "usvfs",
                arch = ?arch,
                preset = preset,
                "Configuring with CMake"
            );

            let cmake_configure = CmakeTool::new()
                .source_dir(&source_path)
                .build_dir(&build_dir)
                .generator(CmakeGenerator::VisualStudio)
                .preset(preset)
                .definition("CMAKE_INSTALL_PREFIX", install_prefix.display().to_string())
                .definition("BUILD_TESTING", "OFF")
                .configure_op();

            cmake_configure
                .run(&tool_ctx)
                .await
                .with_context(|| format!("failed to configure usvfs for {arch:?}"))?;
        }

        // Build with MSBuild for both architectures
        for arch in [Arch::X64, Arch::X86] {
            let solution = Self::solution_path(config, arch)?;

            info!(
                repo = "usvfs",
                arch = ?arch,
                config = %task_config.configuration,
                "Building with MSBuild"
            );

            let msbuild = MsBuildTool::new()
                .solution(&solution)
                .architecture(arch)
                .configuration(task_config.configuration)
                .max_cpu_count(true)
                .build_op();

            msbuild
                .run(&tool_ctx)
                .await
                .with_context(|| format!("failed to build usvfs for {arch:?}"))?;
        }

        Ok(())
    }
}

impl Taskable for UsvfsTask {
    fn name(&self) -> &str {
        &self.name
    }

    fn enabled(&self, ctx: &TaskContext) -> bool {
        ctx.config().task_config(&self.name).enabled
    }

    fn do_clean<'a>(&'a self, ctx: &'a TaskContext) -> BoxFuture<'a, Result<()>> {
        Box::pin(self.do_clean(ctx, ctx.clean_flags()))
    }

    fn do_fetch<'a>(&'a self, ctx: &'a TaskContext) -> BoxFuture<'a, Result<()>> {
        Box::pin(self.do_fetch(ctx))
    }

    fn do_build_and_install<'a>(&'a self, ctx: &'a TaskContext) -> BoxFuture<'a, Result<()>> {
        Box::pin(self.do_build_and_install(ctx))
    }
}

#[cfg(test)]
mod tests;
