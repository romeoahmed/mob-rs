// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::error::Result;
use anyhow::Context;
use std::path::Path;
use tokio::fs;

/// Recursively copies all contents from src directory to dst directory (async version).
///
/// Creates dst if it doesn't exist. Handles both files and directories recursively.
///
/// # Arguments
/// * `src` - Source directory path
/// * `dst` - Destination directory path
///
/// # Example
/// ```no_run
/// use mob_rs::utility::fs::copy::copy_dir_contents_async;
/// use std::path::Path;
///
/// # async fn example() -> anyhow::Result<()> {
/// copy_dir_contents_async(Path::new("/source/dir"), Path::new("/dest/dir")).await?;
/// # Ok(())
/// # }
/// ```
/// # Errors
///
/// Returns an error if any IO operation fails (creating directory, reading, copying).
pub async fn copy_dir_contents_async(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)
        .await
        .with_context(|| format!("failed to create directory {}", dst.display()))?;

    let mut entries = fs::read_dir(src)
        .await
        .with_context(|| format!("failed to read directory {}", src.display()))?;

    while let Some(entry) = entries
        .next_entry()
        .await
        .with_context(|| format!("failed to read entry from {}", src.display()))?
    {
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            Box::pin(copy_dir_contents_async(&src_path, &dst_path)).await?;
        } else {
            fs::copy(&src_path, &dst_path).await.with_context(|| {
                format!(
                    "failed to copy {} to {}",
                    src_path.display(),
                    dst_path.display()
                )
            })?;
        }
    }

    Ok(())
}

/// Copies files matching a pattern from src to dst directory (async version).
///
/// Does not recurse into subdirectories. Only copies files at the top level of src.
/// If pattern is None, all files are copied. If pattern is Some, only files whose
/// names contain the pattern string are copied.
///
/// # Arguments
/// * `src` - Source directory path
/// * `dst` - Destination directory path
/// * `pattern` - Optional filename pattern (substring match)
///
/// # Example
/// ```no_run
/// use mob_rs::utility::fs::copy::copy_files_async;
/// use std::path::Path;
///
/// # async fn example() -> anyhow::Result<()> {
/// // Copy all files
/// copy_files_async(Path::new("/source"), Path::new("/dest"), None).await?;
///
/// // Copy only files with "config" in the name
/// copy_files_async(Path::new("/source"), Path::new("/dest"), Some("config")).await?;
/// # Ok(())
/// # }
/// ```
/// # Errors
///
/// Returns an error if any IO operation fails (creating directory, reading, copying).
pub async fn copy_files_async(src: &Path, dst: &Path, pattern: Option<&str>) -> Result<()> {
    fs::create_dir_all(dst)
        .await
        .with_context(|| format!("failed to create directory {}", dst.display()))?;

    let mut entries = fs::read_dir(src)
        .await
        .with_context(|| format!("failed to read directory {}", src.display()))?;

    while let Some(entry) = entries
        .next_entry()
        .await
        .with_context(|| format!("failed to read entry from {}", src.display()))?
    {
        let src_path = entry.path();
        if src_path.is_file() {
            // If pattern provided, check if filename matches
            if let Some(pat) = pattern {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if !name_str.contains(pat) {
                    continue;
                }
            }

            let dst_path = dst.join(entry.file_name());
            fs::copy(&src_path, &dst_path).await.with_context(|| {
                format!(
                    "failed to copy {} to {}",
                    src_path.display(),
                    dst_path.display()
                )
            })?;
        }
    }

    Ok(())
}
