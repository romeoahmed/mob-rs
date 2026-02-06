// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Logging infrastructure using the `tracing` ecosystem.
//!
//! ```text
//! init_logging(&LogConfig)
//!        |
//!        v
//!    registry
//!    |       |
//!    v       v
//! Console   File (optional)
//! EnvFilter EnvFilter
//! ANSI      non_blocking
//! timestamps FmtSpan::CLOSE
//!        |
//!        v
//!    LogGuard (flush on drop)
//!
//! LogLevel:  0=OFF  1=ERROR  2=WARN  3=INFO
//!            4=DEBUG  5=TRACE  6=DUMP(+libs)
//! ```

use anyhow::Context;
use bon::Builder;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::path::Path;
use tracing::Level;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    EnvFilter, Layer,
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

use crate::error::{ConfigError, Result};

/// Log level (0-6) for configuration.
///
/// Original mob levels:
/// - 0: Silent - no output at all
/// - 1: Error - only errors
/// - 2: Warn - errors and warnings
/// - 3: Info - default, general information
/// - 4: Debug - detailed debugging information
/// - 5: Trace - very verbose tracing
/// - 6: Dump - extremely verbose (includes curl debug, etc)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogLevel(u8);

impl Default for LogLevel {
    fn default() -> Self {
        Self::INFO
    }
}

impl LogLevel {
    pub const SILENT: Self = Self(0);
    pub const ERROR: Self = Self(1);
    pub const WARN: Self = Self(2);
    pub const INFO: Self = Self(3);
    pub const DEBUG: Self = Self(4);
    pub const TRACE: Self = Self(5);
    pub const DUMP: Self = Self(6);

    /// Create a new `LogLevel` from a u8 value (0-6).
    ///
    /// # Errors
    ///
    /// Returns a `ConfigError::InvalidValue` if the level is greater than 6.
    pub fn new(level: u8) -> std::result::Result<Self, ConfigError> {
        if level <= 6 {
            Ok(Self(level))
        } else {
            Err(ConfigError::InvalidValue {
                section: "global".to_string(),
                key: "log_level".to_string(),
                message: format!("log level must be 0-6, got {level}"),
            })
        }
    }

    /// Get the raw u8 value.
    #[must_use]
    pub const fn as_u8(&self) -> u8 {
        self.0
    }

    /// Convert from integer value (saturating at DUMP level).
    #[must_use]
    pub const fn from_int(level: i32) -> Self {
        match level {
            0 => Self::SILENT,
            1 => Self::ERROR,
            2 => Self::WARN,
            3 => Self::INFO,
            4 => Self::DEBUG,
            5 => Self::TRACE,
            _ => Self::DUMP,
        }
    }

    /// Convert from u8 value, returning None if out of range.
    #[must_use]
    pub const fn from_u8(level: u8) -> Option<Self> {
        if level <= 6 { Some(Self(level)) } else { None }
    }

    /// Alias for INFO level.
    #[must_use]
    pub const fn info() -> Self {
        Self::INFO
    }

    /// Convert to tracing Level.
    #[must_use]
    pub const fn to_tracing_level(self) -> Option<Level> {
        match self.0 {
            0 => None,
            1 => Some(Level::ERROR),
            2 => Some(Level::WARN),
            3 => Some(Level::INFO),
            4 => Some(Level::DEBUG),
            _ => Some(Level::TRACE),
        }
    }

    /// Convert to `EnvFilter` directive string.
    #[must_use]
    pub const fn to_filter_string(self) -> &'static str {
        match self.0 {
            0 => "off",
            1 => "error",
            2 => "warn",
            3 => "info",
            4 => "debug",
            _ => "trace",
        }
    }
}

impl TryFrom<u8> for LogLevel {
    type Error = ConfigError;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<LogLevel> for u8 {
    fn from(level: LogLevel) -> Self {
        level.0
    }
}

impl Serialize for LogLevel {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u8(self.0)
    }
}

impl<'de> Deserialize<'de> for LogLevel {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = u8::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

/// Configuration for the logging system.
#[derive(Debug, Clone, Builder)]
pub struct LogConfig {
    #[builder(setters(name = with_console_level), default = LogLevel::INFO)]
    console_level: LogLevel,
    #[builder(setters(name = with_file_level), default = LogLevel::TRACE)]
    file_level: LogLevel,
    #[builder(setters(name = with_log_file))]
    log_file: Option<String>,
    #[builder(setters(name = with_show_timestamps), default = true)]
    show_timestamps: bool,
    #[builder(setters(name = with_show_target), default = false)]
    show_target: bool,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self::builder().build()
    }
}

impl LogConfig {
    /// Get the console log level.
    #[must_use]
    pub const fn console_level(&self) -> LogLevel {
        self.console_level
    }

    /// Get the file log level.
    #[must_use]
    pub const fn file_level(&self) -> LogLevel {
        self.file_level
    }

    /// Get the log file path if set.
    #[must_use]
    pub fn log_file(&self) -> Option<&str> {
        self.log_file.as_deref()
    }

    /// Check if timestamps should be shown in console output.
    #[must_use]
    pub const fn show_timestamps(&self) -> bool {
        self.show_timestamps
    }

    /// Check if target (module path) should be shown in output.
    #[must_use]
    pub const fn show_target(&self) -> bool {
        self.show_target
    }
}

/// RAII guard that keeps the logging system alive.
/// When dropped, flushes all pending log writes.
pub struct LogGuard {
    _file_guard: Option<WorkerGuard>,
}

/// Initialize the logging system with the given configuration.
///
/// Returns a guard that must be kept alive for the duration of the program.
/// When the guard is dropped, pending logs are flushed.
///
/// # Arguments
///
/// * `config` - Logging configuration
///
/// # Errors
///
/// Returns an error if the log directory or file cannot be created.
///
/// # Example
///
/// ```no_run
/// use mob_rs::logging::{init_logging, LogConfig, LogLevel};
///
/// let config = LogConfig::builder()
///     .with_console_level(LogLevel::INFO)
///     .with_file_level(LogLevel::DEBUG)
///     .with_log_file("build.log".to_string())
///     .build();
///
/// let _guard = init_logging(&config).expect("Failed to initialize logging");
/// tracing::info!("Logging initialized");
/// ```
pub fn init_logging(config: &LogConfig) -> Result<LogGuard> {
    // Build console layer
    let console_filter = EnvFilter::new(config.console_level().to_filter_string());

    let console_layer = fmt::layer()
        .with_target(config.show_target())
        .with_level(true)
        .with_ansi(true)
        .with_filter(console_filter);

    // Build file layer if log file is specified
    let (file_layer, file_guard) = if let Some(log_path) = config.log_file() {
        let log_path = Path::new(log_path);

        // Create parent directory if needed
        if let Some(parent) = log_path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create log directory {}", parent.display()))?;
        }

        let file = std::fs::File::create(log_path)
            .with_context(|| format!("failed to create log file {}", log_path.display()))?;
        let (non_blocking, guard) = tracing_appender::non_blocking(file);

        let file_filter = EnvFilter::new(config.file_level().to_filter_string());

        let layer = fmt::layer()
            .with_writer(non_blocking)
            .with_target(true)
            .with_level(true)
            .with_ansi(false)
            .with_span_events(FmtSpan::CLOSE)
            .with_filter(file_filter);

        (Some(layer), Some(guard))
    } else {
        (None, None)
    };

    // Initialize the subscriber
    tracing_subscriber::registry()
        .with(console_layer)
        .with(file_layer)
        .init();

    Ok(LogGuard {
        _file_guard: file_guard,
    })
}

/// Logging context that tracks the current task and tool.
///
/// This replaces the C++ `context` class for providing context in log messages.
#[derive(Debug, Clone, Default)]
pub struct LogContext {
    /// Current task name.
    task: Option<String>,
    /// Current tool name.
    tool: Option<String>,
}

impl LogContext {
    /// Create a new context with a task name.
    pub fn with_task(task: impl Into<String>) -> Self {
        Self {
            task: Some(task.into()),
            tool: None,
        }
    }

    /// Set the current tool.
    pub fn set_tool(&mut self, tool: impl Into<String>) {
        self.tool = Some(tool.into());
    }

    /// Clear the current tool.
    #[cfg(test)]
    pub(crate) fn clear_tool(&mut self) {
        self.tool = None;
    }

    /// Get the current task name.
    #[must_use]
    pub fn task(&self) -> Option<&str> {
        self.task.as_deref()
    }

    /// Get the current tool name.
    #[must_use]
    pub fn tool(&self) -> Option<&str> {
        self.tool.as_deref()
    }

    /// Get the context prefix for log messages.
    #[must_use]
    pub fn prefix(&self) -> String {
        match (&self.task, &self.tool) {
            (Some(task), Some(tool)) => format!("[{task}/{tool}] "),
            (Some(task), None) => format!("[{task}] "),
            (None, Some(tool)) => format!("[{tool}] "),
            (None, None) => String::new(),
        }
    }
}

/// Reason for a log entry (matches original C++ enum).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogReason {
    /// Generic log entry.
    Generic,
    /// Configuration-related.
    Config,
    /// Operation was bypassed.
    Bypass,
    /// Redownload triggered.
    Redownload,
    /// Rebuild triggered.
    Rebuild,
    /// Reextract triggered.
    Reextract,
    /// Interruption-related.
    Interruption,
    /// Command line of a process.
    Command,
    /// Process stdout.
    StdOut,
    /// Process stderr.
    StdErr,
    /// Filesystem operation.
    Filesystem,
    /// Network operation.
    Network,
}

impl LogReason {
    /// Get a short string representation for log output.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Generic => "generic",
            Self::Config => "config",
            Self::Bypass => "bypass",
            Self::Redownload => "redownload",
            Self::Rebuild => "rebuild",
            Self::Reextract => "reextract",
            Self::Interruption => "interrupt",
            Self::Command => "cmd",
            Self::StdOut => "stdout",
            Self::StdErr => "stderr",
            Self::Filesystem => "fs",
            Self::Network => "net",
        }
    }
}

#[cfg(test)]
mod tests;
