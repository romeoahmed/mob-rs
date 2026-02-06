// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Version detection for release builds.
//!
//! Supports two sources:
//! - **exe**: Extract version from `ModOrganizer.exe` via Windows API (Windows only)
//! - **rc**: Parse `VER_FILEVERSION_STR` from `version.rc`

use std::path::{Path, PathBuf};

use anyhow::Context;
use regex::Regex;
use tokio::fs;
use tracing::debug;

use crate::cli::release::DevbuildArgs;
use crate::config::Config;
use crate::error::Result;

#[cfg(windows)]
use windows::Win32::Storage::FileSystem::{
    FILE_VER_GET_NEUTRAL, GetFileVersionInfoExW, GetFileVersionInfoSizeExW, VS_FIXEDFILEINFO,
    VerQueryValueW,
};
#[cfg(windows)]
use windows::core::{HSTRING, w};

/// Determines the version string for a devbuild release.
///
/// Resolution order:
/// 1. Explicit `--version` flag
/// 2. `--version-from-exe` flag
/// 3. `--version-from-rc` flag
/// 4. Auto-detect: try exe first (Windows), then rc
pub(super) async fn determine_version(args: &DevbuildArgs, config: &Config) -> Result<String> {
    if let Some(version) = &args.version {
        return Ok(version.clone());
    }

    if args.version_source.version_from_exe {
        return version_from_exe(config).await;
    }

    if args.version_source.version_from_rc {
        return version_from_rc(args, config).await;
    }

    if cfg!(windows) {
        match version_from_exe(config).await {
            Ok(version) => Ok(version),
            Err(exe_err) => match version_from_rc(args, config).await {
                Ok(version) => Ok(version),
                Err(rc_err) => Err(anyhow::anyhow!(
                    "failed to determine version; exe error: {exe_err:#}; rc error: {rc_err:#}"
                )),
            },
        }
    } else {
        version_from_rc(args, config)
            .await
            .context("failed to determine version from version.rc")
    }
}

/// Determines the version string for an official release.
///
/// Tries exe extraction first (Windows), then falls back to `version.rc`
/// at the standard modorganizer source path.
pub(super) async fn determine_official_version(config: &Config) -> Result<String> {
    // Try to get version from exe first (Windows), fall back to version.rc
    if cfg!(windows) {
        match version_from_exe(config).await {
            Ok(version) => return Ok(version),
            Err(e) => {
                debug!(error = %e, "Failed to get version from exe, trying version.rc");
            }
        }
    }

    // Fall back to version.rc
    let rc_path = default_rc_path(config)?;

    let content = fs::read_to_string(&rc_path)
        .await
        .with_context(|| format!("failed to read {}", rc_path.display()))?;

    parse_version_from_rc_content(&content, &rc_path)
}

/// Reads the version string from a `version.rc` file.
///
/// Uses `args.rc_path` if provided, otherwise falls back to
/// the default rc path derived from `config.paths.build`.
pub(super) async fn version_from_rc(args: &DevbuildArgs, config: &Config) -> Result<String> {
    let rc_path = match &args.rc_path {
        Some(path) => path.clone(),
        None => default_rc_path(config)?,
    };

    let content = fs::read_to_string(&rc_path)
        .await
        .with_context(|| format!("failed to read {}", rc_path.display()))?;

    parse_version_from_rc_content(&content, &rc_path)
}

/// Parses `VER_FILEVERSION_STR` from the content of a `version.rc` file.
fn parse_version_from_rc_content(content: &str, rc_path: &Path) -> Result<String> {
    let regex = Regex::new(r#"(?m)^#define\s+VER_FILEVERSION_STR\s+\"(.+)\\0\""#)
        .with_context(|| "failed to compile version.rc regex")?;

    let captures = regex
        .captures(content)
        .with_context(|| format!("version string not found in {}", rc_path.display()))?;
    let version = captures
        .get(1)
        .with_context(|| format!("version capture missing in {}", rc_path.display()))?
        .as_str()
        .to_string();

    Ok(version)
}

/// Returns the default path to `version.rc` under the build directory.
pub(super) fn default_rc_path(config: &Config) -> Result<PathBuf> {
    let build_dir = config
        .paths
        .build
        .as_ref()
        .context("paths.build not configured")?;
    Ok(build_dir
        .join("modorganizer_super")
        .join("modorganizer")
        .join("src")
        .join("version.rc"))
}

#[cfg(windows)]
async fn version_from_exe(config: &Config) -> Result<String> {
    let install_bin = config
        .paths
        .install_bin
        .as_ref()
        .context("paths.install_bin not configured")?;
    let exe_path = install_bin.join("ModOrganizer.exe");

    // Use spawn_blocking for synchronous Windows API calls
    tokio::task::spawn_blocking(move || version_from_exe_sync(&exe_path))
        .await
        .context("version extraction task panicked")?
}

#[cfg(windows)]
fn version_from_exe_sync(exe_path: &std::path::Path) -> Result<String> {
    use std::ffi::c_void;

    if !exe_path.exists() {
        anyhow::bail!("ModOrganizer.exe not found at {}", exe_path.display());
    }

    let file_path = HSTRING::from(exe_path.as_os_str());

    // Step 1: Get version info size using Ex API
    let mut handle = 0u32;
    let size =
        unsafe { GetFileVersionInfoSizeExW(FILE_VER_GET_NEUTRAL, &file_path, &raw mut handle) };
    if size == 0 {
        return Err(anyhow::Error::new(std::io::Error::last_os_error()))
            .with_context(|| format!("failed to query version size for {}", exe_path.display()));
    }

    // Step 2: Get version info using Ex API
    let mut buffer = vec![0u8; size as usize];
    unsafe {
        GetFileVersionInfoExW(
            FILE_VER_GET_NEUTRAL,
            &file_path,
            None,
            size,
            buffer.as_mut_ptr().cast::<c_void>(),
        )
    }
    .with_context(|| format!("failed to query version info for {}", exe_path.display()))?;

    // Step 3: Query fixed file info
    let mut value_ptr: *mut c_void = std::ptr::null_mut();
    let mut value_len = 0u32;
    let ok = unsafe {
        VerQueryValueW(
            buffer.as_ptr().cast::<c_void>(),
            w!("\\"),
            &raw mut value_ptr,
            &raw mut value_len,
        )
    };

    if !ok.as_bool() || value_ptr.is_null() {
        return Err(anyhow::Error::new(std::io::Error::last_os_error()))
            .with_context(|| format!("failed to read version info for {}", exe_path.display()));
    }

    // Step 4: Extract version numbers from official VS_FIXEDFILEINFO struct
    let info = unsafe { &*(value_ptr.cast::<VS_FIXEDFILEINFO>()) };

    let major = (info.dwFileVersionMS >> 16) & 0xFFFF;
    let minor = info.dwFileVersionMS & 0xFFFF;
    let patch = (info.dwFileVersionLS >> 16) & 0xFFFF;
    let build = info.dwFileVersionLS & 0xFFFF;

    // Format version, trimming trailing zero segments (keep at least major.minor.patch)
    Ok(if build == 0 {
        format!("{major}.{minor}.{patch}")
    } else {
        format!("{major}.{minor}.{patch}.{build}")
    })
}

#[cfg(not(windows))]
async fn version_from_exe(_config: &Config) -> Result<String> {
    anyhow::bail!("version-from-exe is only supported on Windows");
}
