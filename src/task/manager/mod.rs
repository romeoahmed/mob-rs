// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Task manager for orchestrating task execution.
//!
//! ```text
//! TaskManager::new(config)
//!   .add_task()  .with_cancel_token()  .dry_run()
//!   .run().await
//!       per task: Clean --> Fetch --> Build
//!       parallel tasks share a global semaphore
//! ```

use std::sync::Arc;

use crate::error::Result;
use anyhow::Context;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

use crate::config::Config;

use super::{CleanFlags, PhaseControl, Task, TaskContext, Taskable};

/// Manager for orchestrating task execution.
///
/// Tasks are executed sequentially in the order they were added.
/// Parallel execution within tasks respects the global concurrency limit.
pub struct TaskManager {
    /// Tasks to execute.
    tasks: Vec<Task>,

    /// Cancellation token for cooperative cancellation.
    cancel_token: CancellationToken,

    /// Shared configuration.
    config: Arc<Config>,

    /// Semaphore for limiting concurrent parallel tasks.
    /// This is shared across all parallel task groups.
    concurrency_semaphore: Arc<Semaphore>,

    /// Whether to run in dry-run mode.
    dry_run: bool,

    /// Clean flags for controlling what gets cleaned.
    clean_flags: CleanFlags,

    /// Phase control toggles.
    phases: PhaseControl,
}

impl TaskManager {
    /// Creates a new `TaskManager` with the given configuration.
    ///
    /// The default concurrency limit is the number of CPU cores.
    #[must_use]
    pub fn new(config: Arc<Config>) -> Self {
        let max_concurrent = std::thread::available_parallelism()
            .map(std::num::NonZeroUsize::get)
            .unwrap_or(4); // Fallback to 4 if unavailable
        Self {
            tasks: Vec::new(),
            cancel_token: CancellationToken::new(),
            config,
            concurrency_semaphore: Arc::new(Semaphore::new(max_concurrent)),
            dry_run: false,
            clean_flags: CleanFlags::empty(),
            phases: PhaseControl::new(),
        }
    }

    /// Creates a `TaskManager` with a specific concurrency limit.
    #[must_use]
    pub fn with_concurrency(config: Arc<Config>, max_concurrent: usize) -> Self {
        Self {
            tasks: Vec::new(),
            cancel_token: CancellationToken::new(),
            config,
            concurrency_semaphore: Arc::new(Semaphore::new(max_concurrent)),
            dry_run: false,
            clean_flags: CleanFlags::empty(),
            phases: PhaseControl::new(),
        }
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

    /// Adds a task to be executed.
    pub fn add(&mut self, task: Task) {
        self.tasks.push(task);
    }

    /// Returns the number of tasks.
    #[must_use]
    pub const fn task_count(&self) -> usize {
        self.tasks.len()
    }

    /// Returns whether cancellation has been requested.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancel_token.is_cancelled()
    }

    /// Returns the cancellation token for sharing with subtasks.
    #[must_use]
    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel_token.clone()
    }

    /// Returns the concurrency semaphore for parallel task groups.
    #[must_use]
    pub fn concurrency_semaphore(&self) -> Arc<Semaphore> {
        Arc::clone(&self.concurrency_semaphore)
    }

    /// Returns whether dry-run mode is enabled.
    #[must_use]
    pub const fn is_dry_run(&self) -> bool {
        self.dry_run
    }

    /// Returns the clean flags.
    #[must_use]
    pub const fn clean_flags(&self) -> CleanFlags {
        self.clean_flags
    }

    /// Returns the phase control toggles.
    #[must_use]
    pub const fn phases(&self) -> &PhaseControl {
        &self.phases
    }

    /// Triggers cancellation for all tasks.
    ///
    /// This signals all running tasks to stop gracefully.
    /// Tasks should check `is_cancelled()` and exit early.
    pub fn interrupt_all(&self) {
        tracing::info!("Interrupting all tasks");
        self.cancel_token.cancel();
    }

    /// Creates a `TaskContext` for task execution.
    fn create_context(&self) -> TaskContext {
        TaskContext::new(Arc::clone(&self.config), self.cancel_token.clone())
            .with_dry_run(self.dry_run)
            .with_clean_flags(self.clean_flags)
            .with_do_clean(self.phases.do_clean())
            .with_do_fetch(self.phases.do_fetch())
            .with_do_build(self.phases.do_build())
    }

    /// Runs all tasks sequentially.
    ///
    /// Tasks are executed in the order they were added.
    /// Checks for cancellation between tasks.
    ///
    /// # Errors
    ///
    /// Returns an error if any task fails or if cancellation is requested.
    pub async fn run_all(&self) -> Result<()> {
        if self.tasks.is_empty() {
            tracing::debug!("No tasks to run");
            return Ok(());
        }

        tracing::info!(task_count = self.tasks.len(), "Starting task execution");

        let ctx = self.create_context();

        for (i, task) in self.tasks.iter().enumerate() {
            // Check for cancellation before each task
            if self.is_cancelled() {
                anyhow::bail!("Task execution interrupted before task {}", i + 1);
            }

            tracing::info!(
                task = %task.name(),
                index = i + 1,
                total = self.tasks.len(),
                "Running task"
            );

            task.run(&ctx)
                .await
                .with_context(|| format!("Task '{}' failed", task.name()))?;
        }

        tracing::info!("All tasks completed successfully");
        Ok(())
    }
}

#[cfg(test)]
mod tests;
