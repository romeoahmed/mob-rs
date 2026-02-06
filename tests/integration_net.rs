// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Integration tests for network module using wiremock.
//!
//! Tests the Downloader with HTTP mocking, covering:
//! - String downloads
//! - File downloads
//! - Error handling (HTTP errors, missing params)
//! - Progress callbacks
//! - Interrupt support
//! - Custom headers

use mob_rs::error::{MobError, NetworkError};
use mob_rs::net::Downloader;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tempfile::TempDir;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn temp_dir() -> TempDir {
    tempfile::tempdir().expect("failed to create temp dir")
}

// =============================================================================
// download_string tests
// =============================================================================

#[tokio::test]
async fn test_download_string_success() {
    // Start mock server
    let mock_server = MockServer::start().await;

    // Configure mock response
    Mock::given(method("GET"))
        .and(path("/test.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string("Hello, World!"))
        .mount(&mock_server)
        .await;

    // Execute download
    let url = format!("{}/test.txt", mock_server.uri());
    let downloader = Downloader::new().url(&url);
    let result = downloader.download_string().await;

    // Verify
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Hello, World!");
}

#[tokio::test]
async fn test_download_string_http_errors() {
    for (status, file) in [(404, "missing.txt"), (500, "error.txt")] {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path(format!("/{file}")))
            .respond_with(ResponseTemplate::new(status))
            .mount(&mock_server)
            .await;

        let url = format!("{}/{file}", mock_server.uri());
        let downloader = Downloader::new().url(&url);
        let result = downloader.download_string().await;

        assert!(result.is_err());
        match result.unwrap_err() {
            MobError::Network(boxed) => match *boxed {
                NetworkError::HttpError {
                    status: actual_status,
                    ..
                } => {
                    assert_eq!(actual_status, status);
                }
                other => panic!("Expected NetworkError::HttpError for {status}, got {other:?}"),
            },
            other => panic!("Expected MobError::Network for {status}, got {other:?}"),
        }
    }
}

// =============================================================================
// download (file) tests
// =============================================================================

#[tokio::test]
async fn test_download_file_success() {
    // Start mock server
    let mock_server = MockServer::start().await;

    // Configure mock response
    let body_content = "Test file content\nLine 2\nLine 3";
    Mock::given(method("GET"))
        .and(path("/file.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string(body_content))
        .mount(&mock_server)
        .await;

    // Create temp directory
    let temp_dir = temp_dir();
    let output_file = temp_dir.path().join("downloaded.txt");

    // Execute download (silent mode for tests)
    let url = format!("{}/file.txt", mock_server.uri());
    let downloader = Downloader::new().url(&url).file(&output_file).silent();
    let result = downloader.download().await;

    // Verify download succeeded
    assert!(result.is_ok(), "Download failed: {:?}", result.err());

    // Verify file content
    let content = std::fs::read_to_string(&output_file).unwrap();
    assert_eq!(content, body_content);
}

#[tokio::test]
async fn test_download_file_creates_parent_dirs() {
    // Start mock server
    let mock_server = MockServer::start().await;

    // Configure mock response
    Mock::given(method("GET"))
        .and(path("/data.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string("nested content"))
        .mount(&mock_server)
        .await;

    // Create temp directory with nested path
    let temp_dir = temp_dir();
    let output_file = temp_dir
        .path()
        .join("deeply")
        .join("nested")
        .join("path")
        .join("file.txt");

    // Verify parent dirs don't exist yet
    assert!(!output_file.parent().unwrap().exists());

    // Execute download (silent mode for tests)
    let url = format!("{}/data.txt", mock_server.uri());
    let downloader = Downloader::new().url(&url).file(&output_file).silent();
    let result = downloader.download().await;

    // Verify download succeeded
    assert!(result.is_ok(), "Download failed: {:?}", result.err());

    // Verify parent directories were created
    assert!(output_file.parent().unwrap().exists());

    // Verify file content
    let content = std::fs::read_to_string(&output_file).unwrap();
    assert_eq!(content, "nested content");
}

#[tokio::test]
async fn test_download_file_404() {
    // Start mock server
    let mock_server = MockServer::start().await;

    // Configure 404 response
    Mock::given(method("GET"))
        .and(path("/notfound.txt"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    // Create temp directory
    let temp_dir = temp_dir();
    let output_file = temp_dir.path().join("should_not_exist.txt");

    // Execute download (silent mode for tests)
    let url = format!("{}/notfound.txt", mock_server.uri());
    let downloader = Downloader::new().url(&url).file(&output_file).silent();
    let result = downloader.download().await;

    // Verify we get an HTTP error
    assert!(result.is_err());
    match result.unwrap_err() {
        MobError::Network(boxed) => match *boxed {
            NetworkError::HttpError { status, .. } => {
                assert_eq!(status, 404);
            }
            other => panic!("Expected NetworkError::HttpError, got {other:?}"),
        },
        other => panic!("Expected MobError::Network, got {other:?}"),
    }

    // Verify file was not created
    assert!(!output_file.exists());
}

// =============================================================================
// Progress callback tests
// =============================================================================

#[tokio::test]
async fn test_progress_callback_invoked() {
    // Start mock server
    let mock_server = MockServer::start().await;

    // Configure mock response with known size
    let body_content = "x".repeat(1024); // 1KB of data
    Mock::given(method("GET"))
        .and(path("/progress.txt"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(&body_content)
                .append_header("Content-Length", body_content.len().to_string()),
        )
        .mount(&mock_server)
        .await;

    // Create temp directory
    let temp_dir = temp_dir();
    let output_file = temp_dir.path().join("progress.txt");

    // Track progress callbacks
    let progress_calls = Arc::new(AtomicU64::new(0));
    let last_downloaded = Arc::new(AtomicU64::new(0));
    let last_total = Arc::new(AtomicU64::new(0));

    let calls_clone = Arc::clone(&progress_calls);
    let downloaded_clone = Arc::clone(&last_downloaded);
    let total_clone = Arc::clone(&last_total);

    // Execute download with progress callback
    let url = format!("{}/progress.txt", mock_server.uri());
    let downloader = Downloader::new().url(&url).file(&output_file);
    let result = downloader
        .download_with_callback(move |downloaded, total| {
            calls_clone.fetch_add(1, Ordering::SeqCst);
            downloaded_clone.store(downloaded, Ordering::SeqCst);
            total_clone.store(total, Ordering::SeqCst);
        })
        .await;

    // Verify download succeeded
    assert!(result.is_ok(), "Download failed: {:?}", result.err());

    // Verify progress callback was invoked
    let call_count = progress_calls.load(Ordering::SeqCst);
    assert!(call_count > 0, "Progress callback was not invoked");

    // Verify final progress shows all bytes downloaded
    let final_downloaded = last_downloaded.load(Ordering::SeqCst);
    let final_total = last_total.load(Ordering::SeqCst);
    assert_eq!(final_downloaded, 1024);
    assert_eq!(final_total, 1024);
}

#[tokio::test]
async fn test_progress_callback_multiple_calls() {
    // Start mock server
    let mock_server = MockServer::start().await;

    // Configure mock response with body
    let body_content = "data for multiple callbacks";
    Mock::given(method("GET"))
        .and(path("/multi.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string(body_content))
        .mount(&mock_server)
        .await;

    // Create temp directory
    let temp_dir = temp_dir();
    let output_file = temp_dir.path().join("multi.txt");

    // Track progress callbacks
    let progress_calls = Arc::new(AtomicU64::new(0));
    let last_downloaded = Arc::new(AtomicU64::new(0));

    let calls_clone = Arc::clone(&progress_calls);
    let downloaded_clone = Arc::clone(&last_downloaded);

    // Execute download with progress callback
    let url = format!("{}/multi.txt", mock_server.uri());
    let downloader = Downloader::new().url(&url).file(&output_file);
    let result = downloader
        .download_with_callback(move |downloaded, _total| {
            calls_clone.fetch_add(1, Ordering::SeqCst);
            downloaded_clone.store(downloaded, Ordering::SeqCst);
        })
        .await;

    // Verify download succeeded
    assert!(result.is_ok(), "Download failed: {:?}", result.err());

    // Verify progress callback was invoked at least once
    let call_count = progress_calls.load(Ordering::SeqCst);
    assert!(call_count > 0, "Progress callback was not invoked");

    // Verify final downloaded bytes matches content length
    let final_downloaded = last_downloaded.load(Ordering::SeqCst);
    assert_eq!(final_downloaded, body_content.len() as u64);
}

// =============================================================================
// Interrupt tests
// =============================================================================

#[tokio::test]
async fn test_download_interrupted() {
    // Start mock server
    let mock_server = MockServer::start().await;

    // Configure mock response with large body to allow time for interrupt
    let body_content = "x".repeat(1024 * 1024); // 1MB
    Mock::given(method("GET"))
        .and(path("/large.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string(body_content))
        .mount(&mock_server)
        .await;

    // Create temp directory
    let temp_dir = temp_dir();
    let output_file = temp_dir.path().join("large.txt");

    // Create downloader and get interrupt handle
    let url = format!("{}/large.txt", mock_server.uri());
    let downloader = Downloader::new().url(&url).file(&output_file).silent();
    let interrupt_handle = downloader.interrupt_handle();

    // Set interrupt flag immediately
    interrupt_handle.store(true, Ordering::SeqCst);

    // Execute download (should be interrupted)
    let result = downloader.download().await;

    // Verify we get interrupted error
    assert!(result.is_err());
    match result.unwrap_err() {
        MobError::Network(boxed) => match *boxed {
            NetworkError::Interrupted => {
                // Expected
            }
            other => panic!("Expected NetworkError::Interrupted, got {other:?}"),
        },
        other => panic!("Expected MobError::Network, got {other:?}"),
    }

    // Verify file was cleaned up (deleted)
    assert!(!output_file.exists(), "Partial file should be cleaned up");
}

#[tokio::test]
async fn test_download_string_interrupted() {
    // Start mock server
    let mock_server = MockServer::start().await;

    // Configure mock response
    Mock::given(method("GET"))
        .and(path("/string.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string("test"))
        .mount(&mock_server)
        .await;

    // Create downloader and get interrupt handle
    let url = format!("{}/string.txt", mock_server.uri());
    let downloader = Downloader::new().url(&url);
    let interrupt_handle = downloader.interrupt_handle();

    // Set interrupt flag before download
    interrupt_handle.store(true, Ordering::SeqCst);

    // Execute download_string (should be interrupted)
    let result = downloader.download_string().await;

    // Verify we get interrupted error
    assert!(result.is_err());
    match result.unwrap_err() {
        MobError::Network(boxed) => match *boxed {
            NetworkError::Interrupted => {
                // Expected
            }
            other => panic!("Expected NetworkError::Interrupted, got {other:?}"),
        },
        other => panic!("Expected MobError::Network, got {other:?}"),
    }
}

// =============================================================================
// Custom headers tests
// =============================================================================

#[tokio::test]
async fn test_custom_header() {
    // Start mock server
    let mock_server = MockServer::start().await;

    // Configure mock that expects custom header
    Mock::given(method("GET"))
        .and(path("/auth.txt"))
        .and(header("Authorization", "Bearer test-token"))
        .and(header("X-Custom", "custom-value"))
        .respond_with(ResponseTemplate::new(200).set_body_string("authenticated"))
        .mount(&mock_server)
        .await;

    // Execute download with custom headers
    let url = format!("{}/auth.txt", mock_server.uri());
    let downloader = Downloader::new()
        .url(&url)
        .header("Authorization", "Bearer test-token")
        .header("X-Custom", "custom-value");
    let result = downloader.download_string().await;

    // Verify
    assert!(result.is_ok(), "Download failed: {:?}", result.err());
    assert_eq!(result.unwrap(), "authenticated");
}

#[tokio::test]
async fn test_user_agent_set() {
    // Start mock server
    let mock_server = MockServer::start().await;

    // Configure mock that expects User-Agent
    Mock::given(method("GET"))
        .and(path("/ua.txt"))
        .and(header(
            "User-Agent",
            format!("ModOrganizer's mob-rs/{}", env!("CARGO_PKG_VERSION")),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
        .mount(&mock_server)
        .await;

    // Execute download
    let url = format!("{}/ua.txt", mock_server.uri());
    let downloader = Downloader::new().url(&url);
    let result = downloader.download_string().await;

    // Verify
    assert!(result.is_ok(), "Download failed: {:?}", result.err());
    assert_eq!(result.unwrap(), "ok");
}

// =============================================================================
// Error handling tests
// =============================================================================

#[tokio::test]
async fn test_download_no_url_errors() {
    // Test download (file) without URL
    let temp_dir = temp_dir();
    let output_file = temp_dir.path().join("test.txt");
    let downloader = Downloader::new().file(&output_file).silent();
    let result = downloader.download().await;

    assert!(result.is_err());
    match result.unwrap_err() {
        MobError::Network(boxed) => match *boxed {
            NetworkError::InvalidUrl(msg) => {
                assert!(msg.contains("no URL provided"));
            }
            other => panic!("Expected NetworkError::InvalidUrl for download, got {other:?}"),
        },
        other => panic!("Expected MobError::Network for download, got {other:?}"),
    }

    // Test download_string without URL
    let downloader = Downloader::new();
    let result = downloader.download_string().await;

    assert!(result.is_err());
    match result.unwrap_err() {
        MobError::Network(boxed) => match *boxed {
            NetworkError::InvalidUrl(msg) => {
                assert!(msg.contains("no URL provided"));
            }
            other => {
                panic!("Expected NetworkError::InvalidUrl for download_string, got {other:?}")
            }
        },
        other => panic!("Expected MobError::Network for download_string, got {other:?}"),
    }
}

#[tokio::test]
async fn test_download_no_file_error() {
    // Start mock server
    let mock_server = MockServer::start().await;

    // Configure mock response
    Mock::given(method("GET"))
        .and(path("/test.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string("content"))
        .mount(&mock_server)
        .await;

    // Create downloader without output file
    let url = format!("{}/test.txt", mock_server.uri());
    let downloader = Downloader::new().url(&url).silent();
    let result = downloader.download().await;

    // Verify we get DownloadFailed error
    assert!(result.is_err());
    match result.unwrap_err() {
        MobError::Network(boxed) => match *boxed {
            NetworkError::DownloadFailed { message, .. } => {
                assert!(message.contains("no output file specified"));
            }
            other => panic!("Expected NetworkError::DownloadFailed, got {other:?}"),
        },
        other => panic!("Expected MobError::Network, got {other:?}"),
    }
}
