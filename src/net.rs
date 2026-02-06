// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Network module with async downloads.
//!
//! ```text
//! Downloader::new()
//!   .url() .file() .header()
//!   .progress() .silent()
//!        |
//!        +----------+------------+
//!        v          v            v
//!   download()  download_    download_
//!               with_cb()    string()
//!        |
//!        v
//!   Progress display
//!     Bar     [=====>     ] 50MB/100MB
//!     Spinner * 50MB @ 5MB/s
//!     Silent  (none)
//!
//! Global client: OnceLock, connection pool, keep-alive
//! Interruption:  AtomicBool -> cleanup partial -> Interrupted
//! ```

use crate::error::{MobResult, NetworkError};
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use tokio::io::AsyncWriteExt;

/// RAII guard that removes a partial download file on Drop unless explicitly kept.
///
/// Used to ensure partial files are cleaned up on error paths, not just interrupts.
/// Uses blocking deletion which is acceptable because:
/// - File deletion is typically sub-millisecond
/// - This only runs on error paths, not normal operation
/// - Ensures cleanup completes before function returns
struct PartialFileGuard {
    path: PathBuf,
    keep: bool,
}

impl PartialFileGuard {
    const fn new(path: PathBuf) -> Self {
        Self { path, keep: false }
    }

    /// Mark the download as complete - file will NOT be deleted on drop.
    const fn keep(&mut self) {
        self.keep = true;
    }
}

impl Drop for PartialFileGuard {
    fn drop(&mut self) {
        if !self.keep {
            // Blocking delete is acceptable here:
            // - File deletion is fast (sub-millisecond typically)
            // - Only runs on error paths, not normal operation
            // - Ensures cleanup completes before function returns
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

/// Global HTTP client - initialized once, reused across all downloads.
/// Falls back to a basic client if custom configuration fails.
fn global_client() -> &'static Client {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        Client::builder()
            .user_agent(format!(
                "ModOrganizer's mob-rs/{}",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .unwrap_or_else(|_| Client::new())
    })
}

/// Pre-validated progress bar style for known file sizes.
fn bar_style() -> ProgressStyle {
    static STYLE: OnceLock<ProgressStyle> = OnceLock::new();
    STYLE
        .get_or_init(|| {
            ProgressStyle::with_template(
                "{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} @ {binary_bytes_per_sec} ({eta})",
            )
            .unwrap_or_else(|_| ProgressStyle::default_bar())
            .progress_chars("#>-")
        })
        .clone()
}

/// Pre-validated spinner style for unknown file sizes.
fn spinner_style() -> ProgressStyle {
    static STYLE: OnceLock<ProgressStyle> = OnceLock::new();
    STYLE
        .get_or_init(|| {
            ProgressStyle::with_template(
                "{spinner:.green} [{elapsed_precise}] {bytes} @ {binary_bytes_per_sec}",
            )
            .unwrap_or_else(|_| ProgressStyle::default_spinner())
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
        })
        .clone()
}

/// Progress display style for downloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProgressDisplay {
    /// Show a visual progress bar with speed and ETA
    #[default]
    Bar,
    /// Show a spinner (when total size is unknown)
    Spinner,
    /// No visual progress (silent mode)
    Silent,
}

/// Async HTTP downloader with builder pattern.
///
/// Supports visual progress bars and interruptible downloads.
///
/// # Example
/// ```ignore
/// use mob_rs::net::Downloader;
///
/// Downloader::new()
///     .url("https://example.com/file.zip")
///     .file("/tmp/file.zip")
///     .download()
///     .await?;
/// ```
pub struct Downloader {
    client: Client,
    url: Option<String>,
    output_file: Option<PathBuf>,
    headers: Vec<(String, String)>,
    interrupt: Arc<AtomicBool>,
    progress_display: ProgressDisplay,
}

impl Default for Downloader {
    fn default() -> Self {
        Self::new()
    }
}

impl Downloader {
    /// Create a new downloader with default settings.
    /// User-Agent is set to "`ModOrganizer`'s mob-rs/VERSION"
    #[must_use]
    pub fn new() -> Self {
        Self {
            client: global_client().clone(),
            url: None,
            output_file: None,
            headers: Vec::new(),
            interrupt: Arc::new(AtomicBool::new(false)),
            progress_display: ProgressDisplay::default(),
        }
    }

    /// Set the URL to download from.
    #[must_use]
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Set the output file path.
    #[must_use]
    pub fn file(mut self, path: impl Into<PathBuf>) -> Self {
        self.output_file = Some(path.into());
        self
    }

    /// Add a custom header.
    #[must_use]
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// Set the progress display style.
    #[must_use]
    pub const fn progress(mut self, style: ProgressDisplay) -> Self {
        self.progress_display = style;
        self
    }

    /// Disable progress display (silent mode).
    #[must_use]
    pub const fn silent(mut self) -> Self {
        self.progress_display = ProgressDisplay::Silent;
        self
    }

    /// Get a handle to the interrupt flag.
    /// Set to true to interrupt an in-progress download.
    #[must_use]
    pub fn interrupt_handle(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.interrupt)
    }

    /// Create a progress bar for the download.
    fn create_progress_bar(&self, total_size: u64) -> Option<ProgressBar> {
        match self.progress_display {
            ProgressDisplay::Silent => None,
            ProgressDisplay::Spinner | ProgressDisplay::Bar if total_size == 0 => {
                // Unknown size - use spinner
                let pb = ProgressBar::new_spinner();
                pb.set_style(spinner_style());
                Some(pb)
            }
            ProgressDisplay::Bar => {
                // Known size - use progress bar
                let pb = ProgressBar::new(total_size);
                pb.set_style(bar_style());
                Some(pb)
            }
            ProgressDisplay::Spinner => {
                let pb = ProgressBar::new_spinner();
                pb.set_style(spinner_style());
                Some(pb)
            }
        }
    }

    /// Download to the configured file with visual progress bar.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No URL or output file is configured.
    /// - The network request fails or returns a non-success status code.
    /// - Parent directories cannot be created.
    /// - The output file cannot be created or written to.
    /// - The download is interrupted.
    pub async fn download(&self) -> MobResult<()> {
        let url = self
            .url
            .as_ref()
            .ok_or_else(|| NetworkError::InvalidUrl("no URL provided".to_string()))?;
        let output = self
            .output_file
            .as_ref()
            .ok_or_else(|| NetworkError::DownloadFailed {
                url: url.clone(),
                message: "no output file specified".to_string(),
            })?;

        let mut request = self.client.get(url);
        for (name, value) in &self.headers {
            request = request.header(name.as_str(), value.as_str());
        }

        let response = request.send().await.map_err(NetworkError::Reqwest)?;

        if !response.status().is_success() {
            return Err(NetworkError::HttpError {
                status: response.status().as_u16(),
                url: url.clone(),
            }
            .into());
        }

        let total_size = response.content_length().unwrap_or(0);
        let progress_bar = self.create_progress_bar(total_size);

        // Create parent directories if needed
        if let Some(parent) = output.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| NetworkError::DownloadFailed {
                    url: url.clone(),
                    message: format!(
                        "failed to create parent directory {}: {}",
                        parent.display(),
                        e
                    ),
                })?;
        }

        let mut file =
            tokio::fs::File::create(output)
                .await
                .map_err(|e| NetworkError::DownloadFailed {
                    url: url.clone(),
                    message: format!("failed to create output file {}: {}", output.display(), e),
                })?;

        // RAII guard ensures partial file cleanup on any error path
        let mut guard = PartialFileGuard::new(output.clone());

        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            // Check for interrupt
            if self.interrupt.load(Ordering::Relaxed) {
                if let Some(pb) = &progress_bar {
                    pb.abandon_with_message("interrupted");
                }
                // Guard will clean up the partial file on drop
                return Err(NetworkError::Interrupted.into());
            }

            let chunk = chunk.map_err(NetworkError::Reqwest)?;
            file.write_all(&chunk)
                .await
                .map_err(|e| NetworkError::DownloadFailed {
                    url: url.clone(),
                    message: format!("failed to write to {}: {}", output.display(), e),
                })?;

            if let Some(pb) = &progress_bar {
                pb.inc(chunk.len() as u64);
            }
        }

        file.flush()
            .await
            .map_err(|e| NetworkError::DownloadFailed {
                url: url.clone(),
                message: format!("failed to flush {}: {}", output.display(), e),
            })?;

        // Download successful - keep the file
        guard.keep();

        if let Some(pb) = progress_bar {
            pb.finish_with_message("done");
        }

        Ok(())
    }

    /// Download to the configured file with a custom progress callback.
    ///
    /// The callback receives (`bytes_downloaded`, `total_bytes`).
    /// `total_bytes` may be 0 if Content-Length is not provided.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No URL or output file is configured.
    /// - The network request fails or returns a non-success status code.
    /// - Parent directories cannot be created.
    /// - The output file cannot be created or written to.
    /// - The download is interrupted.
    pub async fn download_with_callback<F>(&self, progress: F) -> MobResult<()>
    where
        F: Fn(u64, u64),
    {
        let url = self
            .url
            .as_ref()
            .ok_or_else(|| NetworkError::InvalidUrl("no URL provided".to_string()))?;
        let output = self
            .output_file
            .as_ref()
            .ok_or_else(|| NetworkError::DownloadFailed {
                url: url.clone(),
                message: "no output file specified".to_string(),
            })?;

        let mut request = self.client.get(url);
        for (name, value) in &self.headers {
            request = request.header(name.as_str(), value.as_str());
        }

        let response = request.send().await.map_err(NetworkError::Reqwest)?;

        if !response.status().is_success() {
            return Err(NetworkError::HttpError {
                status: response.status().as_u16(),
                url: url.clone(),
            }
            .into());
        }

        let total_size = response.content_length().unwrap_or(0);
        let mut downloaded: u64 = 0;

        // Create parent directories if needed
        if let Some(parent) = output.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| NetworkError::DownloadFailed {
                    url: url.clone(),
                    message: format!(
                        "failed to create parent directory {}: {}",
                        parent.display(),
                        e
                    ),
                })?;
        }

        let mut file =
            tokio::fs::File::create(output)
                .await
                .map_err(|e| NetworkError::DownloadFailed {
                    url: url.clone(),
                    message: format!("failed to create output file {}: {}", output.display(), e),
                })?;

        // RAII guard ensures partial file cleanup on any error path
        let mut guard = PartialFileGuard::new(output.clone());

        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            // Check for interrupt
            if self.interrupt.load(Ordering::Relaxed) {
                // Guard will clean up the partial file on drop
                return Err(NetworkError::Interrupted.into());
            }

            let chunk = chunk.map_err(NetworkError::Reqwest)?;
            file.write_all(&chunk)
                .await
                .map_err(|e| NetworkError::DownloadFailed {
                    url: url.clone(),
                    message: format!("failed to write to {}: {}", output.display(), e),
                })?;
            downloaded += chunk.len() as u64;

            progress(downloaded, total_size);
        }

        file.flush()
            .await
            .map_err(|e| NetworkError::DownloadFailed {
                url: url.clone(),
                message: format!("failed to flush {}: {}", output.display(), e),
            })?;

        // Download successful - keep the file
        guard.keep();

        Ok(())
    }

    /// Download and return content as string.
    ///
    /// # Errors
    ///
    /// Returns an error if the network request fails, returns a non-success status code,
    /// or if the download is interrupted.
    pub async fn download_string(&self) -> MobResult<String> {
        let url = self
            .url
            .as_ref()
            .ok_or_else(|| NetworkError::InvalidUrl("no URL provided".to_string()))?;

        let mut request = self.client.get(url);
        for (name, value) in &self.headers {
            request = request.header(name.as_str(), value.as_str());
        }

        let response = request.send().await.map_err(NetworkError::Reqwest)?;

        if !response.status().is_success() {
            return Err(NetworkError::HttpError {
                status: response.status().as_u16(),
                url: url.clone(),
            }
            .into());
        }

        // Check for interrupt before reading body
        if self.interrupt.load(Ordering::Relaxed) {
            return Err(NetworkError::Interrupted.into());
        }

        let text = response.text().await.map_err(NetworkError::Reqwest)?;
        Ok(text)
    }
}
