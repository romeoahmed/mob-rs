// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Error handling module.
//!
//! ```text
//!              MobError (~24 bytes)
//!                     |
//!   +---------+-------+-------+---------+
//!   |    |    |    |    |    |    |    | |
//!   v    v    v    v    v    v    v    v v
//! Bail  Git  Net  Cfg  Task Proc  Fs Job Io/Other
//!       Box  Box  Box  Box  Box  Box Box Box<str>
//!
//! Sub-errors (unboxed internally):
//!   Git     Gix, CommandFailed, BranchNotFound
//!   Network Reqwest, HttpError, Timeout
//!   Config  ParseError, MissingKey, InvalidValue
//!   Task    NotFound, ExecutionFailed, Interrupted
//!   Process SpawnFailed, NonZeroExit, Timeout
//!   Fs      NotFound, PermissionDenied, IoError
//!   Job     CreateFailed, AssignFailed (Windows)
//!
//! All variants boxed => MobError fits in 24 bytes.
//! ```

use thiserror::Error;

/// Convenience alias for `anyhow::Result`.
pub type Result<T> = anyhow::Result<T>;

/// Result type using [`MobError`].
pub type MobResult<T> = std::result::Result<T, MobError>;

/// Top-level application error type.
///
/// All sub-errors are boxed to keep this enum at ~24 bytes on the stack.
#[derive(Debug, Error)]
pub enum MobError {
    /// Fatal error that should terminate the application.
    #[error("fatal error: {0}")]
    Bailed(Box<str>),

    /// Git operation failed.
    #[error("git error: {0}")]
    Git(#[from] Box<GitError>),

    /// Network operation failed.
    #[error("network error: {0}")]
    Network(#[from] Box<NetworkError>),

    /// Configuration error.
    #[error("config error: {0}")]
    Config(#[from] Box<ConfigError>),

    /// Task execution error.
    #[error("task error: {0}")]
    Task(#[from] Box<TaskError>),

    /// Process execution error.
    #[error("process error: {0}")]
    Process(#[from] Box<ProcessError>),

    /// Filesystem error.
    #[error("filesystem error: {0}")]
    Fs(#[from] Box<FsError>),

    /// Job Object error (Windows).
    #[error("job error: {0}")]
    Job(#[from] Box<JobError>),

    /// I/O error.
    #[error("io error: {0}")]
    Io(Box<std::io::Error>),

    /// Generic error with message.
    #[error("{0}")]
    Other(Box<str>),
}

/// Create a fatal [`MobError::Bailed`] that terminates the application.
pub fn bail_out(message: impl Into<String>) -> MobError {
    MobError::Bailed(message.into().into_boxed_str())
}

// --- From implementations for boxing ---

/// Macro to generate `From` implementations that box the source error.
macro_rules! impl_from_boxed {
    ($($error:ty => $variant:ident),+ $(,)?) => {
        $(
            impl From<$error> for MobError {
                fn from(err: $error) -> Self {
                    MobError::$variant(Box::new(err))
                }
            }
        )+
    };
}

impl_from_boxed! {
    GitError => Git,
    NetworkError => Network,
    ConfigError => Config,
    TaskError => Task,
    ProcessError => Process,
    FsError => Fs,
    JobError => Job,
    std::io::Error => Io,
}

// --- Gix Errors ---

/// Wrapper for gix-specific errors.
///
/// gix has multiple error types that are converted through this enum.
/// Large error types are boxed to keep enum size manageable.
#[derive(Debug, Error)]
pub enum GixError {
    /// Failed to discover repository from path.
    #[error("failed to discover repository: {0}")]
    Discover(#[from] Box<gix::discover::Error>),

    /// Failed to open repository.
    #[error("failed to open repository: {0}")]
    Open(#[from] Box<gix::open::Error>),

    /// Failed to access repository index.
    #[error("failed to access index: {0}")]
    Index(#[from] gix::worktree::open_index::Error),

    /// Failed to get HEAD reference.
    #[error("failed to get head reference: {0}")]
    Head(#[from] gix::reference::find::existing::Error),

    /// Repository has no worktree (bare repository).
    #[error("repository has no worktree (bare repository)")]
    BareRepository,
}

// --- Git Errors ---

/// Git operation errors.
#[derive(Debug, Error)]
pub enum GitError {
    /// Repository not found at the specified path.
    #[error("repository not found: {path}")]
    RepoNotFound { path: String },

    /// Git command execution failed.
    #[error("git command failed: {command} - {message}")]
    CommandFailed { command: String, message: String },

    /// Error from gix library.
    #[error("gix error: {0}")]
    Gix(#[from] GixError),

    /// Uncommitted changes detected when clean working tree required.
    #[error("uncommitted changes in {path}")]
    UncommittedChanges { path: String },

    /// Branch not found.
    #[error("branch not found: {branch}")]
    BranchNotFound { branch: String },

    /// Remote not found.
    #[error("remote not found: {remote}")]
    RemoteNotFound { remote: String },

    /// Clone operation failed.
    #[error("failed to clone {url}: {message}")]
    CloneFailed { url: String, message: String },

    /// Checkout operation failed.
    #[error("failed to checkout {what}: {message}")]
    CheckoutFailed { what: String, message: String },
}

// --- Network Errors ---

/// Network operation errors.
#[derive(Debug, Error)]
pub enum NetworkError {
    /// Download failed.
    #[error("download failed: {url} - {message}")]
    DownloadFailed { url: String, message: String },

    /// Download was interrupted by user or signal.
    #[error("download interrupted")]
    Interrupted,

    /// HTTP error response.
    #[error("http error {status}: {url}")]
    HttpError { status: u16, url: String },

    /// Error from reqwest library.
    #[error("reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),

    /// Invalid URL.
    #[error("invalid url: {0}")]
    InvalidUrl(String),

    /// Connection timeout.
    #[error("connection timeout: {url}")]
    Timeout { url: String },

    /// I/O error during download.
    #[error("io error during download: {0}")]
    Io(#[from] std::io::Error),
}

// --- Config Errors ---

/// Configuration-related errors.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Failed to read configuration file.
    #[error("failed to read config file '{path}': {source}")]
    ReadError {
        path: String,
        #[source]
        source: std::io::Error,
    },

    /// Failed to parse configuration file.
    #[error("failed to parse config file '{path}': {message}")]
    ParseError { path: String, message: String },

    /// Missing required configuration key.
    #[error("missing required config key '{key}' in section '[{section}]'")]
    MissingKey { section: String, key: String },

    /// Invalid configuration value.
    #[error("invalid value for '{key}' in section '[{section}]': {message}")]
    InvalidValue {
        section: String,
        key: String,
        message: String,
    },

    /// Configuration file not found.
    #[error("config file not found: {0}")]
    NotFound(String),
}

// --- Task Errors ---

/// Task execution errors.
#[derive(Debug, Error)]
pub enum TaskError {
    /// Task was not found.
    #[error("task '{0}' not found")]
    NotFound(String),

    /// Task execution failed.
    #[error("task '{name}' failed: {message}")]
    ExecutionFailed { name: String, message: String },

    /// Task was interrupted.
    #[error("task '{0}' was interrupted")]
    Interrupted(String),

    /// Task dependency failed.
    #[error("task '{task}' failed because dependency '{dependency}' failed")]
    DependencyFailed { task: String, dependency: String },
}

// --- Process Errors ---

/// Process execution errors.
#[derive(Debug, Error)]
pub enum ProcessError {
    /// Executable not found in PATH.
    #[error("executable not found: '{name}' (not in PATH)")]
    ExecutableNotFound { name: String },

    /// Failed to spawn process.
    #[error("failed to spawn process '{command}': {source}")]
    SpawnFailed {
        command: String,
        #[source]
        source: std::io::Error,
    },

    /// Process exited with non-zero status.
    #[error("process '{command}' exited with code {code}")]
    NonZeroExit { command: String, code: i32 },

    /// Process timed out.
    #[error("process '{command}' timed out after {timeout_secs} seconds")]
    Timeout { command: String, timeout_secs: u64 },

    /// Failed to read process output.
    #[error("failed to read output from process '{command}': {message}")]
    OutputError { command: String, message: String },
}

// --- Filesystem Errors ---

/// Filesystem operation errors.
#[derive(Debug, Error)]
pub enum FsError {
    /// Path not found.
    #[error("path not found: {0}")]
    NotFound(String),

    /// Permission denied.
    #[error("permission denied: {0}")]
    PermissionDenied(String),

    /// General I/O error.
    #[error("I/O error on '{path}': {source}")]
    IoError {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

// --- Job Object Errors (Windows) ---

/// Windows Job Object errors.
#[derive(Debug, Error)]
pub enum JobError {
    /// Failed to create a Job Object.
    #[error("failed to create job object")]
    CreateFailed(#[source] std::io::Error),

    /// Failed to configure a Job Object.
    #[error("failed to configure job object")]
    ConfigureFailed(#[source] std::io::Error),

    /// Failed to assign a process to a Job Object.
    #[error("failed to assign process (PID {pid}) to job")]
    AssignFailed {
        pid: u32,
        #[source]
        source: std::io::Error,
    },

    /// Failed to assign a process handle to a Job Object.
    #[error("failed to assign process handle to job")]
    AssignHandleFailed(#[source] std::io::Error),

    /// Failed to open a process for job assignment.
    #[error("failed to open process (PID {pid})")]
    OpenProcessFailed {
        pid: u32,
        #[source]
        source: std::io::Error,
    },

    /// Failed to terminate a Job Object.
    #[error("failed to terminate job")]
    TerminateFailed(#[source] std::io::Error),

    /// Failed to query job information.
    #[error("failed to query job information")]
    QueryFailed(#[source] std::io::Error),
}

#[cfg(test)]
mod tests;
