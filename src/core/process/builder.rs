// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Process builder with configuration options.
//!
//! ```text
//! ProcessBuilder
//!  • new/which/raw/exists/find
//!  • args/cwd/env/flags/timeout/success_codes/name
//!  • capture_stdout/stderr/output, quiet, inherit_stdio, stdin
//!
//! ProcessFlags: ALLOW_FAILURE, TERMINATE_ON_INTERRUPT, IGNORE_OUTPUT_ON_SUCCESS
//! StreamFlags: FORWARD_TO_LOG (default), BIT_BUCKET, KEEP_IN_STRING, INHERIT
//! ```

use bitflags::bitflags;
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};
use std::time::Duration;

use crate::core::env::container::Env;
use crate::utility::encoding::Encoding;

/// Static cache for executable paths resolved via `which`.
static EXECUTABLE_CACHE: OnceLock<RwLock<BTreeMap<String, PathBuf>>> = OnceLock::new();

/// Get the executable cache, initializing if needed.
fn exe_cache() -> &'static RwLock<BTreeMap<String, PathBuf>> {
    EXECUTABLE_CACHE.get_or_init(|| RwLock::new(BTreeMap::new()))
}

bitflags! {
    /// Flags controlling process execution behavior.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct ProcessFlags: u32 {
        /// Don't fail if the process exits with a non-zero status
        const ALLOW_FAILURE = 0x01;
        /// Force-terminate process on interrupt instead of graceful shutdown
        const TERMINATE_ON_INTERRUPT = 0x02;
        /// Don't log output if the process succeeds
        const IGNORE_OUTPUT_ON_SUCCESS = 0x04;
    }
}

bitflags! {
    /// Flags controlling stream handling for stdout/stderr.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct StreamFlags: u32 {
        /// Forward output to tracing logs
        const FORWARD_TO_LOG = 0x01;
        /// Discard output (send to /dev/null)
        const BIT_BUCKET = 0x02;
        /// Keep output in a string for later retrieval
        const KEEP_IN_STRING = 0x04;
        /// Inherit from parent process
        const INHERIT = 0x08;
    }
}

impl Default for StreamFlags {
    fn default() -> Self {
        Self::FORWARD_TO_LOG
    }
}

/// Output from a completed process.
#[derive(Debug, Clone, Default)]
pub struct ProcessOutput {
    exit_code: i32,
    stdout: String,
    stderr: String,
    interrupted: bool,
}

impl ProcessOutput {
    /// Creates a new `ProcessOutput` (for internal use).
    pub(super) const fn new(
        exit_code: i32,
        stdout: String,
        stderr: String,
        interrupted: bool,
    ) -> Self {
        Self {
            exit_code,
            stdout,
            stderr,
            interrupted,
        }
    }

    /// Returns the process exit code (0 = success).
    #[must_use]
    pub const fn exit_code(&self) -> i32 {
        self.exit_code
    }

    /// Returns captured stdout (if `KEEP_IN_STRING` was set).
    #[must_use]
    pub fn stdout(&self) -> &str {
        &self.stdout
    }

    /// Returns captured stderr (if `KEEP_IN_STRING` was set).
    #[must_use]
    pub fn stderr(&self) -> &str {
        &self.stderr
    }

    /// Returns whether the process was interrupted.
    #[must_use]
    pub const fn is_interrupted(&self) -> bool {
        self.interrupted
    }

    /// Returns true if the process exited successfully (code 0).
    #[must_use]
    pub const fn success(&self) -> bool {
        self.exit_code == 0
    }
}

/// Configuration for a stream (stdout or stderr).
#[derive(Debug, Clone)]
pub(super) struct StreamConfig {
    flags: StreamFlags,
    encoding: Encoding,
}

impl StreamConfig {
    /// Returns the stream flags.
    pub(super) const fn flags(&self) -> StreamFlags {
        self.flags
    }

    /// Returns the stream encoding.
    pub(super) const fn encoding(&self) -> Encoding {
        self.encoding
    }

    /// Sets the stream flags.
    pub(super) const fn set_flags(&mut self, flags: StreamFlags) {
        self.flags = flags;
    }

    /// Sets the stream encoding.
    pub(super) const fn set_encoding(&mut self, encoding: Encoding) {
        self.encoding = encoding;
    }
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            flags: StreamFlags::FORWARD_TO_LOG,
            encoding: Encoding::Unknown,
        }
    }
}

/// Builder for configuring and running a process.
///
/// Uses the builder pattern to configure process options before spawning.
#[derive(Debug)]
pub struct ProcessBuilder {
    /// Path to the executable
    program: PathBuf,
    /// Command-line arguments
    args: Vec<String>,
    /// Working directory
    cwd: Option<PathBuf>,
    /// Environment variables
    env: Option<Env>,
    /// Process flags
    flags: ProcessFlags,
    /// Stdout configuration
    stdout: StreamConfig,
    /// Stderr configuration
    stderr: StreamConfig,
    /// Stdin content (if any)
    stdin: Option<String>,
    /// Exit codes considered successful (default: {0})
    success_codes: BTreeSet<i32>,
    /// Display name for logging
    name: Option<String>,
    /// Timeout for the process
    timeout: Option<Duration>,
}

impl ProcessBuilder {
    /// Creates a new `ProcessBuilder` for the given program.
    ///
    /// The program can be an absolute path, relative path, or just the executable name.
    /// If just a name is given, it will be resolved via PATH when `run()` is called.
    pub fn new(program: impl AsRef<Path>) -> Self {
        let mut success_codes = BTreeSet::new();
        success_codes.insert(0);

        Self {
            program: program.as_ref().to_path_buf(),
            args: Vec::new(),
            cwd: None,
            env: None,
            flags: ProcessFlags::empty(),
            stdout: StreamConfig::default(),
            stderr: StreamConfig::default(),
            stdin: None,
            success_codes,
            name: None,
            timeout: None,
        }
    }

    /// Creates a `ProcessBuilder` after resolving the program via PATH.
    ///
    /// Uses the `which` crate to find the executable in PATH.
    /// Results are cached for subsequent lookups of the same program.
    ///
    /// # Errors
    ///
    /// Returns a `ProcessError::ExecutableNotFound` if the executable is not found in PATH.
    ///
    /// # Example
    /// ```ignore
    /// let builder = ProcessBuilder::which("cargo")?;
    /// ```
    pub fn which(program: &str) -> std::result::Result<Self, crate::error::ProcessError> {
        // Check cache first (read lock)
        {
            let cache = exe_cache()
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(path) = cache.get(program) {
                return Ok(Self::new(path.clone()));
            }
        }

        // Not in cache, resolve via which
        which::which(program).map_or_else(
            |_| {
                Err(crate::error::ProcessError::ExecutableNotFound {
                    name: program.to_string(),
                })
            },
            |path| {
                // Cache the result (write lock)
                {
                    let mut cache = exe_cache()
                        .write()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    cache.insert(program.to_string(), path.clone());
                }
                Ok(Self::new(path))
            },
        )
    }

    /// Checks if an executable exists in PATH.
    ///
    /// Uses the cache if available, otherwise resolves and caches.
    ///
    /// # Example
    /// ```ignore
    /// if ProcessBuilder::exists("cargo") {
    ///     println!("cargo is available");
    /// }
    /// ```
    #[must_use]
    pub fn exists(program: &str) -> bool {
        Self::find(program).is_some()
    }

    /// Finds the full path to an executable in PATH.
    ///
    /// Results are cached for subsequent lookups.
    /// Returns `None` if the executable is not found.
    #[must_use]
    pub fn find(program: &str) -> Option<PathBuf> {
        // Check cache first (read lock)
        {
            let cache = exe_cache()
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(path) = cache.get(program) {
                return Some(path.clone());
            }
        }

        // Not in cache, resolve via which
        which::which(program).map_or(None, |path| {
            // Cache the result (write lock)
            {
                let mut cache = exe_cache()
                    .write()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                cache.insert(program.to_string(), path.clone());
            }
            Some(path)
        })
    }

    /// Finds all matching executables in PATH.
    ///
    /// Returns an iterator over all matching paths.
    pub fn find_all(program: &str) -> impl Iterator<Item = PathBuf> {
        which::which_all(program).into_iter().flatten()
    }

    /// Creates a `ProcessBuilder` from a raw command string.
    ///
    /// On Windows, this executes the command via `PowerShell` (`pwsh -NoProfile -Command`).
    /// On Unix, this executes via `/bin/sh -c`.
    pub fn raw(command: impl Into<String>) -> Self {
        let cmd = command.into();
        #[cfg(windows)]
        {
            let mut builder = Self::new("pwsh");
            builder.args = vec![
                "-NoProfile".to_string(),
                "-NonInteractive".to_string(),
                "-Command".to_string(),
                cmd,
            ];
            builder
        }
        #[cfg(not(windows))]
        {
            let mut builder = Self::new("/bin/sh");
            builder.args = vec!["-c".to_string(), cmd];
            builder
        }
    }

    /// Adds an argument to the command.
    #[must_use]
    pub fn arg(mut self, arg: impl AsRef<OsStr>) -> Self {
        self.args.push(arg.as_ref().to_string_lossy().into_owned());
        self
    }

    /// Adds multiple arguments to the command.
    #[must_use]
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        for arg in args {
            self.args.push(arg.as_ref().to_string_lossy().into_owned());
        }
        self
    }

    /// Sets the working directory for the process.
    #[must_use]
    pub fn cwd(mut self, dir: impl AsRef<Path>) -> Self {
        self.cwd = Some(dir.as_ref().to_path_buf());
        self
    }

    /// Sets the environment variables for the process.
    #[must_use]
    pub fn env(mut self, env: Env) -> Self {
        self.env = Some(env);
        self
    }

    /// Sets process flags.
    #[must_use]
    pub const fn flags(mut self, flags: ProcessFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Adds a process flag.
    #[must_use]
    pub fn flag(mut self, flag: ProcessFlags) -> Self {
        self.flags |= flag;
        self
    }

    /// Configures stdout handling.
    #[must_use]
    pub const fn stdout_flags(mut self, flags: StreamFlags) -> Self {
        self.stdout.set_flags(flags);
        self
    }

    /// Configures stderr handling.
    #[must_use]
    pub const fn stderr_flags(mut self, flags: StreamFlags) -> Self {
        self.stderr.set_flags(flags);
        self
    }

    /// Sets the encoding for stdout.
    #[must_use]
    pub const fn stdout_encoding(mut self, encoding: Encoding) -> Self {
        self.stdout.set_encoding(encoding);
        self
    }

    /// Sets the encoding for stderr.
    #[must_use]
    pub const fn stderr_encoding(mut self, encoding: Encoding) -> Self {
        self.stderr.set_encoding(encoding);
        self
    }

    /// Convenience: capture stdout to string.
    #[must_use]
    pub const fn capture_stdout(mut self) -> Self {
        self.stdout.set_flags(StreamFlags::KEEP_IN_STRING);
        self
    }

    /// Convenience: capture stderr to string.
    #[must_use]
    pub const fn capture_stderr(mut self) -> Self {
        self.stderr.set_flags(StreamFlags::KEEP_IN_STRING);
        self
    }

    /// Convenience: capture both stdout and stderr to strings.
    #[must_use]
    pub const fn capture_output(self) -> Self {
        self.capture_stdout().capture_stderr()
    }

    /// Convenience: discard all output.
    #[must_use]
    pub const fn quiet(mut self) -> Self {
        self.stdout.set_flags(StreamFlags::BIT_BUCKET);
        self.stderr.set_flags(StreamFlags::BIT_BUCKET);
        self
    }

    /// Convenience: inherit stdout/stderr from parent.
    #[must_use]
    pub const fn inherit_stdio(mut self) -> Self {
        self.stdout.set_flags(StreamFlags::INHERIT);
        self.stderr.set_flags(StreamFlags::INHERIT);
        self
    }

    /// Sets stdin content.
    #[must_use]
    pub fn stdin(mut self, content: impl Into<String>) -> Self {
        self.stdin = Some(content.into());
        self
    }

    /// Sets the exit codes considered successful.
    #[must_use]
    pub fn success_codes(mut self, codes: impl IntoIterator<Item = i32>) -> Self {
        self.success_codes = codes.into_iter().collect();
        self
    }

    /// Sets a display name for logging.
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets a timeout for the process.
    #[must_use]
    pub const fn timeout(mut self, duration: Duration) -> Self {
        self.timeout = Some(duration);
        self
    }

    // Getters for field access within the process module

    /// Returns a reference to the program path.
    #[must_use]
    pub const fn program(&self) -> &PathBuf {
        &self.program
    }

    /// Returns a slice of the arguments.
    pub(super) fn args_slice(&self) -> &[String] {
        &self.args
    }

    /// Returns a reference to the working directory, if set.
    pub(super) const fn working_dir(&self) -> Option<&PathBuf> {
        self.cwd.as_ref()
    }

    /// Returns a reference to the environment, if set.
    pub(super) const fn environment(&self) -> Option<&Env> {
        self.env.as_ref()
    }

    /// Returns the process flags.
    pub(super) const fn process_flags(&self) -> ProcessFlags {
        self.flags
    }

    /// Returns a reference to the stdout configuration.
    pub(super) const fn stdout_config(&self) -> &StreamConfig {
        &self.stdout
    }

    /// Returns a reference to the stderr configuration.
    pub(super) const fn stderr_config(&self) -> &StreamConfig {
        &self.stderr
    }

    /// Returns the stdin content, if set.
    pub(super) fn stdin_content(&self) -> Option<&str> {
        self.stdin.as_deref()
    }

    /// Returns a reference to the success codes set.
    pub(super) const fn success_code_set(&self) -> &BTreeSet<i32> {
        &self.success_codes
    }

    /// Returns the display name, if set.
    pub(super) fn name_override(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Returns the timeout duration, if set.
    pub(super) const fn timeout_duration(&self) -> Option<Duration> {
        self.timeout
    }
}
