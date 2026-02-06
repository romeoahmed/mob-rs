// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Shared test utilities for tool tests.
//!
//! Provides log-capturing infrastructure for testing dry-run output.

use std::io::Write;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use tracing::Level;
use tracing_subscriber::fmt::MakeWriter;

#[derive(Clone)]
struct BufferWriter {
    buffer: Arc<Mutex<Vec<u8>>>,
}

impl Write for BufferWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.buffer
            .lock()
            .map_err(|_| std::io::Error::other("buffer poisoned"))?
            .extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[derive(Clone)]
struct BufferMakeWriter {
    buffer: Arc<Mutex<Vec<u8>>>,
}

impl<'a> MakeWriter<'a> for BufferMakeWriter {
    type Writer = BufferWriter;

    fn make_writer(&'a self) -> Self::Writer {
        BufferWriter {
            buffer: self.buffer.clone(),
        }
    }
}

/// Runs an async closure while capturing tracing output.
///
/// Returns the captured log output as a string.
pub(super) async fn run_with_logs<F, Fut>(f: F) -> Result<String>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<()>>,
{
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .with_writer(BufferMakeWriter {
            buffer: buffer.clone(),
        })
        .with_max_level(Level::INFO)
        .with_ansi(false)
        .with_target(false)
        .with_level(false)
        .finish();

    let _guard = tracing::subscriber::set_default(subscriber);
    f().await?;

    let guard = buffer
        .lock()
        .map_err(|_| anyhow::anyhow!("log buffer poisoned"))?;
    Ok(String::from_utf8_lossy(&guard).to_string())
}
