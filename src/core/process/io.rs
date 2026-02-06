// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! I/O streaming and output capture for processes.
//!
//! ```text
//! run_child() / run_child_with_cancellation()
//!   stdin task (optional)
//!   stdout/stderr reader tasks
//!   mpsc channels buffer lines
//!   wait (or cancel/timeout)
//!   --> ProcessOutput { stdout, stderr, exit_code, interrupted }
//!
//! read_stream()
//!   Utf8/Unknown --> BufReader.lines()
//!   Other        --> EncodedBuffer (CP1252, UTF-16LE, ...)
//! ```

use crate::error::Result;
use anyhow::Context;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, ChildStderr, ChildStdout};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, trace, warn};

use super::builder::{ProcessBuilder, ProcessOutput, StreamFlags};
use crate::utility::encoding::{EncodedBuffer, Encoding};

/// Configuration for spawning a stream reader task.
struct StreamReaderConfig {
    encoding: Encoding,
    flags: StreamFlags,
    process_name: String,
}

/// Spawns a reader task for stdout if needed.
fn spawn_stdout_reader(
    stdout: Option<ChildStdout>,
    config: &StreamReaderConfig,
    tx: mpsc::Sender<String>,
) -> Option<JoinHandle<()>> {
    if !config
        .flags
        .intersects(StreamFlags::FORWARD_TO_LOG | StreamFlags::KEEP_IN_STRING)
    {
        return None;
    }
    stdout.map(|stdout| {
        let encoding = config.encoding;
        let flags = config.flags;
        let name = config.process_name.clone();
        tokio::spawn(async move {
            read_stream(stdout, encoding, flags, &name, "stdout", tx).await;
        })
    })
}

/// Spawns a reader task for stderr if needed.
fn spawn_stderr_reader(
    stderr: Option<ChildStderr>,
    config: &StreamReaderConfig,
    tx: mpsc::Sender<String>,
) -> Option<JoinHandle<()>> {
    if !config
        .flags
        .intersects(StreamFlags::FORWARD_TO_LOG | StreamFlags::KEEP_IN_STRING)
    {
        return None;
    }
    stderr.map(|stderr| {
        let encoding = config.encoding;
        let flags = config.flags;
        let name = config.process_name.clone();
        tokio::spawn(async move {
            read_stream(stderr, encoding, flags, &name, "stderr", tx).await;
        })
    })
}

/// Collects output from a channel into a string.
fn collect_output(rx: &mut mpsc::Receiver<String>, flags: StreamFlags) -> String {
    if !flags.contains(StreamFlags::KEEP_IN_STRING) {
        return String::new();
    }
    let mut output = String::new();
    while let Ok(line) = rx.try_recv() {
        if !output.is_empty() {
            output.push('\n');
        }
        output.push_str(&line);
    }
    output
}

/// Waits for reader tasks to complete.
async fn await_readers(
    stdout_handle: Option<JoinHandle<()>>,
    stderr_handle: Option<JoinHandle<()>>,
) {
    if let Some(handle) = stdout_handle {
        let _ = handle.await;
    }
    if let Some(handle) = stderr_handle {
        let _ = handle.await;
    }
}

/// Terminates a child process gracefully (Windows: `CTRL_BREAK` first, then kill).
async fn terminate_process(child: &mut Child) {
    #[cfg(windows)]
    if let Some(pid) = child.id() {
        if let Err(e) = super::windows::send_ctrl_break(pid) {
            debug!(pid = pid, error = %e, "CTRL_BREAK failed, will force kill");
        } else {
            // Give the process a moment to handle CTRL_BREAK
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }
    child.kill().await.ok();
}

impl ProcessBuilder {
    /// Runs the child process, handling I/O streaming and waiting for completion.
    pub(super) async fn run_child(&self, name: &str, child: &mut Child) -> Result<ProcessOutput> {
        let (stdout_tx, mut stdout_rx) = mpsc::channel::<String>(100);
        let (stderr_tx, mut stderr_rx) = mpsc::channel::<String>(100);

        let stdout_config = StreamReaderConfig {
            encoding: self.stdout_config().encoding(),
            flags: self.stdout_config().flags(),
            process_name: name.to_string(),
        };
        let stderr_config = StreamReaderConfig {
            encoding: self.stderr_config().encoding(),
            flags: self.stderr_config().flags(),
            process_name: name.to_string(),
        };

        let stdout_handle = spawn_stdout_reader(child.stdout.take(), &stdout_config, stdout_tx);
        let stderr_handle = spawn_stderr_reader(child.stderr.take(), &stderr_config, stderr_tx);

        self.write_stdin(name, child).await?;

        let exit_status = if let Some(timeout_duration) = self.timeout_duration() {
            tokio::select! {
                status = child.wait() => status?,
                () = tokio::time::sleep(timeout_duration) => {
                    warn!(process = %name, timeout = ?timeout_duration, "Process timed out");
                    child.kill().await.with_context(|| format!("failed to kill process {name}"))?;
                    child.wait().await?
                }
            }
        } else {
            child.wait().await?
        };

        await_readers(stdout_handle, stderr_handle).await;

        Ok(ProcessOutput::new(
            exit_status.code().unwrap_or(-1),
            collect_output(&mut stdout_rx, self.stdout_config().flags()),
            collect_output(&mut stderr_rx, self.stderr_config().flags()),
            false,
        ))
    }

    /// Runs the child process with cancellation support.
    pub(super) async fn run_child_with_cancellation(
        &self,
        name: &str,
        child: &mut Child,
        token: CancellationToken,
    ) -> Result<ProcessOutput> {
        let (stdout_tx, mut stdout_rx) = mpsc::channel::<String>(100);
        let (stderr_tx, mut stderr_rx) = mpsc::channel::<String>(100);

        let stdout_config = StreamReaderConfig {
            encoding: self.stdout_config().encoding(),
            flags: self.stdout_config().flags(),
            process_name: name.to_string(),
        };
        let stderr_config = StreamReaderConfig {
            encoding: self.stderr_config().encoding(),
            flags: self.stderr_config().flags(),
            process_name: name.to_string(),
        };

        let stdout_handle = spawn_stdout_reader(child.stdout.take(), &stdout_config, stdout_tx);
        let stderr_handle = spawn_stderr_reader(child.stderr.take(), &stderr_config, stderr_tx);

        self.write_stdin(name, child).await?;

        let (exit_status, interrupted) = tokio::select! {
            status = child.wait() => (status?, false),
            () = token.cancelled() => {
                warn!(process = %name, "Cancellation requested, terminating process");
                terminate_process(child).await;
                let status = child.wait().await
                    .with_context(|| format!("failed waiting for process {name} to exit"))?;
                (status, true)
            }
        };

        await_readers(stdout_handle, stderr_handle).await;

        Ok(ProcessOutput::new(
            exit_status.code().unwrap_or(-1),
            collect_output(&mut stdout_rx, self.stdout_config().flags()),
            collect_output(&mut stderr_rx, self.stderr_config().flags()),
            interrupted,
        ))
    }

    /// Writes stdin content to the child process if configured.
    async fn write_stdin(&self, name: &str, child: &mut Child) -> Result<()> {
        if let Some(stdin_content) = self.stdin_content()
            && let Some(mut stdin) = child.stdin.take()
        {
            use tokio::io::AsyncWriteExt;
            stdin
                .write_all(stdin_content.as_bytes())
                .await
                .with_context(|| format!("failed to write to stdin for process {name}"))?;
        }
        Ok(())
    }
}

/// Reads from a stream and processes lines.
async fn read_stream<R>(
    reader: R,
    encoding: Encoding,
    flags: StreamFlags,
    process_name: &str,
    stream_name: &str,
    tx: mpsc::Sender<String>,
) where
    R: tokio::io::AsyncRead + Unpin,
{
    let buf_reader = BufReader::new(reader);

    // For non-UTF8 encodings, we need to handle raw bytes
    // For simplicity, we use line-based reading for UTF-8/unknown
    match encoding {
        Encoding::Utf8 | Encoding::Unknown => {
            let mut lines = buf_reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if flags.contains(StreamFlags::FORWARD_TO_LOG) {
                    trace!(process = %process_name, stream = %stream_name, line = %line, "output");
                }
                if flags.contains(StreamFlags::KEEP_IN_STRING) {
                    let _ = tx.send(line).await;
                }
            }
        }
        _ => {
            // For other encodings, read raw bytes and use EncodedBuffer
            use tokio::io::AsyncReadExt;
            let mut buf_reader = buf_reader;
            let mut buffer = EncodedBuffer::new(encoding);
            let mut read_buf = [0u8; 4096];

            loop {
                match buf_reader.read(&mut read_buf).await {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        buffer.add(&read_buf[..n]);
                        // Process complete lines
                        for line in buffer.next_utf8_lines(false) {
                            if flags.contains(StreamFlags::FORWARD_TO_LOG) {
                                trace!(process = %process_name, stream = %stream_name, line = %line, "output");
                            }
                            if flags.contains(StreamFlags::KEEP_IN_STRING) {
                                let _ = tx.send(line).await;
                            }
                        }
                    }
                    Err(e) => {
                        warn!(
                            process = %process_name,
                            stream = %stream_name,
                            error = %e,
                            "error reading stream"
                        );
                        break;
                    }
                }
            }

            for line in buffer.next_utf8_lines(true) {
                if flags.contains(StreamFlags::FORWARD_TO_LOG) {
                    trace!(process = %process_name, stream = %stream_name, line = %line, "output");
                }
                if flags.contains(StreamFlags::KEEP_IN_STRING) {
                    let _ = tx.send(line).await;
                }
            }
        }
    }
}
