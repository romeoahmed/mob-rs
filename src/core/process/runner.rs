// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Process execution and lifecycle management.
//!
//! ```text
//! run() / run_with_cancellation(token)
//!              |
//!              v
//!     build_command()
//!     args, cwd, env, stdio
//!              |
//!              v
//!          spawn()
//!         /       \
//!        v         v
//!   (Windows)   (other)
//!   job_object  run_child
//!        \       /
//!         v     v
//!    validate exit_code
//!    (skip if ALLOW_FAILURE)
//!              |
//!              v
//!       ProcessOutput
//!    { exit_code, stdout, stderr }
//! ```

use crate::error::Result;
use anyhow::Context;
use std::process::Stdio;
use tokio::process::{Child, Command};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, trace};

use super::builder::{ProcessBuilder, ProcessFlags, ProcessOutput, StreamFlags};

#[cfg(windows)]
use crate::core::job::JobObject;

impl ProcessBuilder {
    /// Returns the display name for this process.
    fn display_name(&self) -> String {
        self.name_override().map_or_else(
            || {
                self.program().file_stem().map_or_else(
                    || "process".to_string(),
                    |s| s.to_string_lossy().into_owned(),
                )
            },
            String::from,
        )
    }

    /// Returns the full command line as a string (for logging).
    fn command_line(&self) -> String {
        let mut cmd = format!("{}", self.program().display());
        for arg in self.args_slice() {
            if arg.contains(' ') {
                use std::fmt::Write as _;
                let _ = write!(cmd, " \"{arg}\"");
            } else {
                use std::fmt::Write as _;
                let _ = write!(cmd, " {arg}");
            }
        }
        cmd
    }

    /// Spawns and runs the process, waiting for completion.
    ///
    /// This is the main entry point for executing a process.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Spawning the child process fails.
    /// - The process exits with a non-zero status (and `ALLOW_FAILURE` flag is not set).
    /// - IO error occurs during output streaming.
    pub async fn run(self) -> Result<ProcessOutput> {
        let name = self.display_name();
        let cmd_line = self.command_line();

        if let Some(cwd) = self.working_dir() {
            debug!(cwd = %cwd.display(), "cd");
        }
        debug!(cmd = %cmd_line, "exec");

        // Build the tokio Command
        let mut command = self.build_command();

        // Spawn the process
        let mut child = command
            .spawn()
            .with_context(|| format!("Failed to spawn: {cmd_line}"))?;

        let pid = child.id();
        trace!(process = %name, pid = ?pid, "spawned");

        // Set up Job Object on Windows
        #[cfg(windows)]
        let _job = setup_job_object(&child)?;

        // Run the process with streaming output
        let output = self.run_child(&name, &mut child).await?;

        // Check exit code
        if !self.process_flags().contains(ProcessFlags::ALLOW_FAILURE)
            && !self.success_code_set().contains(&output.exit_code())
        {
            if !output.stderr().is_empty() {
                error!(process = %name, stderr = %output.stderr(), "process error output");
            }
            anyhow::bail!(
                "{} exited with code {} (expected one of {:?})",
                name,
                output.exit_code(),
                self.success_code_set()
            );
        }

        trace!(process = %name, exit_code = output.exit_code(), "completed");
        Ok(output)
    }

    /// Spawns and runs the process with cancellation support.
    ///
    /// Similar to `run()`, but accepts a `CancellationToken` that can be used
    /// to interrupt the process. When the token is cancelled:
    /// - On Windows: sends `CTRL_BREAK_EVENT` to the process
    /// - On Unix: kills the process (SIGKILL)
    /// - Returns with `interrupted = true` in the output
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Spawning the child process fails.
    /// - The process exits with a non-zero status (and `ALLOW_FAILURE` flag is not set,
    ///   and the process was not interrupted).
    /// - IO error occurs during output streaming.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use tokio_util::sync::CancellationToken;
    /// use std::time::Duration;
    /// use mob_rs::core::process::builder::ProcessBuilder;
    ///
    /// let token = CancellationToken::new();
    /// let token_clone = token.clone();
    ///
    /// // Cancel after 5 seconds
    /// tokio::spawn(async move {
    ///     tokio::time::sleep(Duration::from_secs(5)).await;
    ///     token_clone.cancel();
    /// });
    ///
    /// let output = ProcessBuilder::new("long-running-command")
    ///     .run_with_cancellation(token)
    ///     .await?;
    /// ```
    pub async fn run_with_cancellation(self, token: CancellationToken) -> Result<ProcessOutput> {
        let name = self.display_name();
        let cmd_line = self.command_line();

        // Check if already cancelled before spawning
        if token.is_cancelled() {
            return Ok(ProcessOutput::new(-1, String::new(), String::new(), true));
        }

        if let Some(cwd) = self.working_dir() {
            debug!(cwd = %cwd.display(), "cd");
        }
        debug!(cmd = %cmd_line, "exec");

        // Build the tokio Command
        let mut command = self.build_command();

        // Spawn the process
        let mut child = command
            .spawn()
            .with_context(|| format!("Failed to spawn: {cmd_line}"))?;

        let pid = child.id();
        trace!(process = %name, pid = ?pid, "spawned");

        // Set up Job Object on Windows
        #[cfg(windows)]
        let _job = setup_job_object(&child)?;

        // Run the process with cancellation support
        let output = self
            .run_child_with_cancellation(&name, &mut child, token)
            .await?;

        // Check exit code (unless interrupted or ALLOW_FAILURE)
        if !output.is_interrupted()
            && !self.process_flags().contains(ProcessFlags::ALLOW_FAILURE)
            && !self.success_code_set().contains(&output.exit_code())
        {
            if !output.stderr().is_empty() {
                error!(process = %name, stderr = %output.stderr(), "process error output");
            }
            anyhow::bail!(
                "{} exited with code {} (expected one of {:?})",
                name,
                output.exit_code(),
                self.success_code_set()
            );
        }

        trace!(
            process = %name,
            exit_code = output.exit_code(),
            interrupted = output.is_interrupted(),
            "completed"
        );
        Ok(output)
    }

    /// Builds the tokio Command from this builder's configuration.
    fn build_command(&self) -> Command {
        let mut command = Command::new(self.program());

        // Arguments
        command.args(self.args_slice());

        // Working directory
        if let Some(cwd) = self.working_dir() {
            command.current_dir(cwd);
        }

        // Environment
        if let Some(env) = self.environment() {
            command.env_clear();
            for (key, value) in env.iter() {
                command.env(key, value);
            }
        }

        // Stdin
        if self.stdin_content().is_some() {
            command.stdin(Stdio::piped());
        } else {
            command.stdin(Stdio::null());
        }

        // Stdout
        command.stdout(Self::stdio_from_flags(self.stdout_config().flags()));

        // Stderr
        command.stderr(Self::stdio_from_flags(self.stderr_config().flags()));

        // Kill on drop for safety
        command.kill_on_drop(true);

        // Windows-specific: create new process group
        #[cfg(windows)]
        {
            command.creation_flags(windows::Win32::System::Threading::CREATE_NEW_PROCESS_GROUP.0);
        }

        command
    }

    /// Converts `StreamFlags` to Stdio configuration.
    fn stdio_from_flags(flags: StreamFlags) -> Stdio {
        if flags.contains(StreamFlags::INHERIT) {
            Stdio::inherit()
        } else if flags.contains(StreamFlags::BIT_BUCKET) {
            Stdio::null()
        } else {
            Stdio::piped()
        }
    }
}

#[cfg(windows)]
fn setup_job_object(child: &Child) -> Result<Option<JobObject>> {
    super::windows::setup_job_object(child)
}
