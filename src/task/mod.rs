// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Task execution system.
//!
//! # Architecture
//!
//! ```text
//! TaskManager
//!      |
//!      v
//!   Task enum ----> TaskContext (config, cancel token)
//!      |
//!      v
//!    Phases
//!   /  |   \
//!  v   v    v
//! Clean Fetch Build+Install
//!                  |
//!                  v
//!               Tools
//!         cmake, git, msbuild..
//!
//! Task variants: Usvfs, ModOrganizer,
//!   Stylesheets, Translations, ...
//! ```
//!
//! # Key Types
//!
//! | Type | Purpose |
//! |------|---------|
//! | [`TaskManager`] | Orchestrates task execution with cancellation |
//! | [`Task`] | Enum dispatching to concrete task implementations |
//! | [`Taskable`] | Trait defining the common task interface |
//! | [`Phase`] | Three-phase lifecycle: Clean → Fetch → `BuildAndInstall` |
//! | [`CleanFlags`] | Bitflags controlling what to clean |
//! | [`TaskContext`] | Execution context with config and cancellation token |
//!
//! # The Taskable Pattern
//!
//! All task types implement the [`Taskable`] trait, which defines the common
//! interface for task execution:
//!
//! - [`Taskable::name()`] - Returns the task name
//! - [`Taskable::enabled()`] - Whether the task should run (default: `true`)
//! - [`Taskable::do_clean()`] - Executes the clean phase
//! - [`Taskable::do_fetch()`] - Executes the fetch phase
//! - [`Taskable::do_build_and_install()`] - Executes the build and install phase
//!
//! The [`Task`] enum implements `Taskable` via the `impl_taskable_for_task!` macro,
//! which generates a match arm for each variant, delegating to the inner type's
//! `Taskable` implementation. This provides zero-cost abstraction through
//! compile-time dispatch.
//!
//! ## Adding a New Task
//!
//! 1. Create the task struct in `tasks/` module
//! 2. Implement `Taskable` for the struct
//! 3. Add a variant to the `Task` enum
//! 4. Add the variant name to `impl_taskable_for_task!` invocation

pub mod helpers;
pub mod manager;
pub mod registry;
pub mod tasks;
pub mod tools;

use bitflags::bitflags;
use futures_util::future::BoxFuture;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use crate::config::Config;
use crate::error::Result;
use crate::task::tools::ToolContext;

use tasks::explorerpp::ExplorerPPTask;
use tasks::installer::InstallerTask;
use tasks::licenses::LicensesTask;
use tasks::modorganizer::ModOrganizerTask;
use tasks::stylesheets::StylesheetsTask;
use tasks::translations::TranslationsTask;
use tasks::usvfs::UsvfsTask;

/// Task execution phase.
///
/// Each task goes through these phases in order during execution.
/// The phases match the C++ mob implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Phase {
    /// Clean phase: remove cached files, source directories, build artifacts.
    /// Controlled by `CleanFlags`.
    Clean,

    /// Fetch phase: download sources, clone repositories, extract archives.
    Fetch,

    /// Build and install phase: configure, compile, and install the component.
    BuildAndInstall,
}

/// Controls which task phases are enabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PhaseControl {
    do_clean: bool,
    do_fetch: bool,
    do_build: bool,
}

impl PhaseControl {
    /// Creates the default phase settings.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            do_clean: false,
            do_fetch: true,
            do_build: true,
        }
    }

    /// Sets whether to run the clean phase.
    #[must_use]
    pub const fn with_clean(mut self, enable: bool) -> Self {
        self.do_clean = enable;
        self
    }

    /// Sets whether to run the fetch phase.
    #[must_use]
    pub const fn with_fetch(mut self, enable: bool) -> Self {
        self.do_fetch = enable;
        self
    }

    /// Sets whether to run the build phase.
    #[must_use]
    pub const fn with_build(mut self, enable: bool) -> Self {
        self.do_build = enable;
        self
    }

    /// Returns whether the clean phase should run.
    #[must_use]
    pub const fn do_clean(&self) -> bool {
        self.do_clean
    }

    /// Returns whether the fetch phase should run.
    #[must_use]
    pub const fn do_fetch(&self) -> bool {
        self.do_fetch
    }

    /// Returns whether the build phase should run.
    #[must_use]
    pub const fn do_build(&self) -> bool {
        self.do_build
    }
}

impl Phase {
    /// Returns all phases in execution order.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[Self::Clean, Self::Fetch, Self::BuildAndInstall]
    }

    /// Returns the display name for this phase.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Clean => "clean",
            Self::Fetch => "fetch",
            Self::BuildAndInstall => "build_and_install",
        }
    }
}

bitflags! {
    /// Flags controlling what gets cleaned during the Clean phase.
    ///
    /// These flags match the C++ mob implementation.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct CleanFlags: u8 {
        /// Re-download cached archives and files.
        const REDOWNLOAD = 0x01;

        /// Re-extract source directories (or re-clone for git repos).
        const REEXTRACT = 0x02;

        /// Delete build configuration (cmake cache, etc).
        const RECONFIGURE = 0x04;

        /// Clean build artifacts without reconfiguring.
        const REBUILD = 0x08;
    }
}

/// Trait for task implementations.
///
/// This trait defines the common interface for all task types, enabling
/// compile-time dispatch via the `impl_taskable_for_task!` macro on the `Task` enum.
///
/// # Lifetime
///
/// Methods return `BoxFuture` to support async trait methods with proper
/// lifetime handling for recursive task execution (e.g., `ParallelTasks`).
///
/// # Example
///
/// ```ignore
/// impl Taskable for MyTask {
///     fn name(&self) -> &str { &self.name }
///     fn enabled(&self, ctx: &TaskContext) -> bool { true }
///     fn do_clean<'a>(&'a self, ctx: &'a TaskContext) -> BoxFuture<'a, Result<()>> {
///         Box::pin(self.do_clean(ctx, ctx.clean_flags()))
///     }
///     fn do_fetch<'a>(&'a self, ctx: &'a TaskContext) -> BoxFuture<'a, Result<()>> {
///         Box::pin(self.do_fetch(ctx))
///     }
///     fn do_build_and_install<'a>(&'a self, ctx: &'a TaskContext) -> BoxFuture<'a, Result<()>> {
///         Box::pin(self.do_build_and_install(ctx))
///     }
/// }
/// ```
pub trait Taskable {
    /// Returns the task name.
    fn name(&self) -> &str;

    /// Returns whether this task is enabled for the given context.
    ///
    /// Default implementation returns `true`. Override for tasks that can be
    /// conditionally disabled (e.g., platform-specific or config-driven).
    fn enabled(&self, _ctx: &TaskContext) -> bool {
        true
    }

    /// Executes the clean phase.
    ///
    /// Clean flags are obtained from `ctx.clean_flags()`.
    fn do_clean<'a>(&'a self, ctx: &'a TaskContext) -> BoxFuture<'a, Result<()>>;

    /// Executes the fetch phase.
    fn do_fetch<'a>(&'a self, ctx: &'a TaskContext) -> BoxFuture<'a, Result<()>>;

    /// Executes the build and install phase.
    fn do_build_and_install<'a>(&'a self, ctx: &'a TaskContext) -> BoxFuture<'a, Result<()>>;
}

/// Context provided to tasks during execution.
///
/// Contains configuration, cancellation tokens, and execution flags.
#[derive(Clone)]
pub struct TaskContext {
    /// Reference to the configuration.
    config: Arc<Config>,

    /// Cancellation token for cooperative cancellation.
    cancel_token: CancellationToken,

    /// Whether this is a dry-run execution.
    dry_run: bool,

    /// Flags controlling what gets cleaned.
    clean_flags: CleanFlags,

    /// Phase control toggles.
    phases: PhaseControl,
}

impl TaskContext {
    /// Creates a new `TaskContext`.
    #[must_use]
    pub const fn new(config: Arc<Config>, cancel_token: CancellationToken) -> Self {
        Self {
            config,
            cancel_token,
            dry_run: false,
            clean_flags: CleanFlags::empty(),
            phases: PhaseControl::new(),
        }
    }

    /// Gets a reference to the configuration.
    #[must_use]
    pub const fn config(&self) -> &Arc<Config> {
        &self.config
    }

    /// Gets a reference to the cancellation token.
    #[must_use]
    pub const fn cancel_token(&self) -> &CancellationToken {
        &self.cancel_token
    }

    /// Returns whether this is a dry-run execution.
    #[must_use]
    pub const fn is_dry_run(&self) -> bool {
        self.dry_run
    }

    /// Gets the clean flags.
    #[must_use]
    pub const fn clean_flags(&self) -> CleanFlags {
        self.clean_flags
    }

    /// Gets the phase control.
    #[must_use]
    pub const fn phases(&self) -> PhaseControl {
        self.phases
    }

    /// Sets dry-run mode.
    #[must_use]
    pub const fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    /// Sets clean flags.
    #[must_use]
    pub const fn with_clean_flags(mut self, flags: CleanFlags) -> Self {
        self.clean_flags = flags;
        self
    }

    /// Enables the clean phase.
    #[must_use]
    pub const fn with_do_clean(mut self, enable: bool) -> Self {
        self.phases = self.phases.with_clean(enable);
        self
    }

    /// Enables the fetch phase.
    #[must_use]
    pub const fn with_do_fetch(mut self, enable: bool) -> Self {
        self.phases = self.phases.with_fetch(enable);
        self
    }

    /// Enables the build phase.
    #[must_use]
    pub const fn with_do_build(mut self, enable: bool) -> Self {
        self.phases = self.phases.with_build(enable);
        self
    }

    /// Checks if cancellation has been requested.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancel_token.is_cancelled()
    }

    /// Creates a `ToolContext` from this `TaskContext`.
    #[must_use]
    pub fn tool_context(&self) -> ToolContext {
        ToolContext::new(
            Arc::clone(&self.config),
            self.cancel_token.clone(),
            self.dry_run,
        )
    }
}

/// Wrapper for parallel task execution.
///
/// All child tasks are executed concurrently when this task runs.
/// Respects global concurrency limits via semaphore.
#[derive(Debug, Clone)]
pub struct ParallelTasks {
    /// Child tasks to execute in parallel.
    children: Vec<Task>,
}

impl ParallelTasks {
    /// Creates a new `ParallelTasks` wrapper.
    #[must_use]
    pub const fn new(children: Vec<Task>) -> Self {
        Self { children }
    }

    /// Adds a child task.
    #[must_use]
    pub fn with_task(mut self, task: Task) -> Self {
        self.children.push(task);
        self
    }

    /// Returns a reference to child tasks.
    #[must_use]
    pub fn children(&self) -> &[Task] {
        &self.children
    }

    /// Consumes self and returns child tasks.
    #[must_use]
    pub fn into_children(self) -> Vec<Task> {
        self.children
    }
}

impl Taskable for ParallelTasks {
    fn name(&self) -> &'static str {
        "parallel"
    }

    fn enabled(&self, _ctx: &TaskContext) -> bool {
        !self.children.is_empty()
    }

    fn do_clean<'a>(&'a self, ctx: &'a TaskContext) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            // For parallel tasks, clean children sequentially
            // (parallel execution happens in build phase)
            for child in &self.children {
                child.do_clean(ctx).await?;
            }
            Ok(())
        })
    }

    fn do_fetch<'a>(&'a self, ctx: &'a TaskContext) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            // For parallel tasks, fetch children sequentially
            for child in &self.children {
                child.do_fetch(ctx).await?;
            }
            Ok(())
        })
    }

    fn do_build_and_install<'a>(&'a self, ctx: &'a TaskContext) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            use tokio::task::JoinSet;
            let mut set = JoinSet::new();

            for child in &self.children {
                let child = child.clone();
                let ctx = ctx.clone();
                set.spawn(async move { child.do_build_and_install_owned(ctx).await });
            }

            // Wait for all and collect errors
            let mut errors = Vec::new();
            while let Some(result) = set.join_next().await {
                match result {
                    Ok(Ok(())) => {}
                    Ok(Err(e)) => errors.push(e),
                    Err(e) => errors.push(anyhow::anyhow!("Task panicked: {e}")),
                }
            }

            if let Some(first_error) = errors.first() {
                for (i, e) in errors.iter().enumerate().skip(1) {
                    tracing::error!(error = %e, task_index = i + 1, "Additional parallel task error");
                }
                return Err(anyhow::anyhow!("{first_error}"));
            }

            Ok(())
        })
    }
}

/// A build task.
///
/// This enum uses compile-time dispatch for zero-cost abstraction.
/// New task types are added as variants.
#[derive(Debug, Clone)]
pub enum Task {
    /// Parallel execution wrapper - runs children concurrently.
    Parallel(ParallelTasks),
    /// `ModOrganizer` project build task.
    ModOrganizer(ModOrganizerTask),
    /// USVFS multi-arch build task.
    Usvfs(UsvfsTask),
    /// Stylesheets download task.
    Stylesheets(StylesheetsTask),
    /// Explorer++ download task.
    ExplorerPP(ExplorerPPTask),
    /// Licenses copy task.
    Licenses(LicensesTask),
    /// Translations task.
    Translations(TranslationsTask),
    /// Installer build task.
    Installer(InstallerTask),
}

impl Task {
    /// Runs the task through all applicable phases.
    ///
    /// Checks for cancellation between phases.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the enabled phases fail or if the task is interrupted.
    pub async fn run(&self, ctx: &TaskContext) -> Result<()> {
        if !Taskable::enabled(self, ctx) {
            tracing::debug!(task = %Taskable::name(self), "Skipping disabled task");
            return Ok(());
        }

        // Clean phase
        if ctx.phases().do_clean() && !ctx.clean_flags().is_empty() {
            if ctx.is_cancelled() {
                anyhow::bail!(
                    "Task {} interrupted before clean phase",
                    Taskable::name(self)
                );
            }
            Taskable::do_clean(self, ctx).await?;
        }

        // Fetch phase
        if ctx.phases().do_fetch() {
            if ctx.is_cancelled() {
                anyhow::bail!(
                    "Task {} interrupted before fetch phase",
                    Taskable::name(self)
                );
            }
            Taskable::do_fetch(self, ctx).await?;
        }

        // Build and install phase
        if ctx.phases().do_build() {
            if ctx.is_cancelled() {
                anyhow::bail!(
                    "Task {} interrupted before build phase",
                    Taskable::name(self)
                );
            }
            Taskable::do_build_and_install(self, ctx).await?;
        }

        Ok(())
    }

    /// Owned version of `do_build_and_install` for spawning tasks.
    /// Takes owned `TaskContext` to avoid lifetime issues with `tokio::spawn`.
    pub(crate) fn do_build_and_install_owned(
        self,
        ctx: TaskContext,
    ) -> BoxFuture<'static, Result<()>> {
        Box::pin(async move {
            tracing::debug!(task = %Taskable::name(&self), phase = "build_and_install", "Starting phase");

            match self {
                Self::Parallel(p) => {
                    // Execute children in parallel using JoinSet
                    use tokio::task::JoinSet;
                    let mut set = JoinSet::new();

                    for child in p.into_children() {
                        let ctx = ctx.clone();
                        set.spawn(async move { child.do_build_and_install_owned(ctx).await });
                    }

                    // Wait for all and collect errors
                    let mut errors = Vec::new();
                    while let Some(result) = set.join_next().await {
                        match result {
                            Ok(Ok(())) => {}
                            Ok(Err(e)) => errors.push(e),
                            Err(e) => errors.push(anyhow::anyhow!("Task panicked: {e}")),
                        }
                    }

                    if let Some(first_error) = errors.first() {
                        // Log additional errors beyond the first
                        for (i, e) in errors.iter().enumerate().skip(1) {
                            tracing::error!(error = %e, task_index = i + 1, "Additional parallel task error");
                        }
                        // Safe: we just checked errors.first() is Some
                        return Err(anyhow::anyhow!("{first_error}"));
                    }
                }
                Self::ModOrganizer(t) => {
                    Taskable::do_build_and_install(&t, &ctx).await?;
                }
                Self::Usvfs(t) => {
                    Taskable::do_build_and_install(&t, &ctx).await?;
                }
                Self::Stylesheets(t) => {
                    Taskable::do_build_and_install(&t, &ctx).await?;
                }
                Self::ExplorerPP(t) => {
                    Taskable::do_build_and_install(&t, &ctx).await?;
                }
                Self::Licenses(t) => {
                    Taskable::do_build_and_install(&t, &ctx).await?;
                }
                Self::Translations(t) => {
                    Taskable::do_build_and_install(&t, &ctx).await?;
                }
                Self::Installer(t) => {
                    Taskable::do_build_and_install(&t, &ctx).await?;
                }
            }

            Ok(())
        })
    }
}

/// Macro to implement Taskable for Task enum by delegating to inner types.
macro_rules! impl_taskable_for_task {
    ($($variant:ident),+ $(,)?) => {
        impl Taskable for Task {
            fn name(&self) -> &str {
                match self {
                    $(Task::$variant(t) => Taskable::name(t),)+
                }
            }

            fn enabled(&self, ctx: &TaskContext) -> bool {
                match self {
                    $(Task::$variant(t) => Taskable::enabled(t, ctx),)+
                }
            }

            fn do_clean<'a>(&'a self, ctx: &'a TaskContext) -> BoxFuture<'a, Result<()>> {
                match self {
                    $(Task::$variant(t) => Taskable::do_clean(t, ctx),)+
                }
            }

            fn do_fetch<'a>(&'a self, ctx: &'a TaskContext) -> BoxFuture<'a, Result<()>> {
                match self {
                    $(Task::$variant(t) => Taskable::do_fetch(t, ctx),)+
                }
            }

            fn do_build_and_install<'a>(&'a self, ctx: &'a TaskContext) -> BoxFuture<'a, Result<()>> {
                match self {
                    $(Task::$variant(t) => Taskable::do_build_and_install(t, ctx),)+
                }
            }
        }
    };
}

impl_taskable_for_task!(
    Parallel,
    ModOrganizer,
    Usvfs,
    Stylesheets,
    ExplorerPP,
    Licenses,
    Translations,
    Installer,
);

#[cfg(test)]
mod tests;
