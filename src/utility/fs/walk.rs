// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::error::Result;
use bon::Builder;
use flume::bounded;
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tracing::warn;

/// Options for parallel directory traversal.
#[derive(Debug, Clone, Builder)]
pub struct WalkOptions {
    /// Maximum depth to traverse (None = unlimited)
    #[builder(setters(name = with_max_depth))]
    max_depth: Option<usize>,
    /// Follow symbolic links
    #[builder(setters(name = with_follow_links), default = false)]
    follow_links: bool,
    /// Include hidden files/directories
    #[builder(setters(name = with_include_hidden), default = false)]
    include_hidden: bool,
    /// Respect .gitignore files
    #[builder(setters(name = with_respect_gitignore), default = true)]
    respect_gitignore: bool,
    /// Number of threads (None = auto-detect based on CPU count)
    #[builder(setters(name = with_threads))]
    threads: Option<usize>,
    /// Skip directories matching these names (exact match)
    #[builder(setters(name = with_skip_dirs), default)]
    skip_dirs: Vec<String>,
}

impl Default for WalkOptions {
    fn default() -> Self {
        Self::builder().build()
    }
}

impl WalkOptions {
    /// Returns the maximum depth to traverse.
    #[must_use]
    pub const fn max_depth(&self) -> Option<usize> {
        self.max_depth
    }

    /// Returns whether to follow symbolic links.
    #[must_use]
    pub const fn follow_links(&self) -> bool {
        self.follow_links
    }

    /// Returns whether to include hidden files/directories.
    #[must_use]
    pub const fn include_hidden(&self) -> bool {
        self.include_hidden
    }

    /// Returns whether to respect .gitignore files.
    #[must_use]
    pub const fn respect_gitignore(&self) -> bool {
        self.respect_gitignore
    }

    /// Returns the number of threads (None = auto-detect).
    #[must_use]
    pub const fn threads(&self) -> Option<usize> {
        self.threads
    }

    /// Returns the skip directories list.
    #[must_use]
    pub fn skip_dirs(&self) -> &[String] {
        &self.skip_dirs
    }

    /// Creates options optimized for build tool scanning.
    ///
    /// - Ignores hidden files
    /// - Respects .gitignore
    /// - Skips common build directories (`node_modules`, target, .git, etc.)
    #[must_use]
    pub fn for_build_tool() -> Self {
        Self::builder()
            .with_skip_dirs(vec![
                "node_modules".to_string(),
                "target".to_string(),
                ".git".to_string(),
                ".hg".to_string(),
                ".svn".to_string(),
                "__pycache__".to_string(),
                ".tox".to_string(),
                "venv".to_string(),
                ".venv".to_string(),
            ])
            .build()
    }
}

/// Result of a parallel walk operation.
#[derive(Debug)]
pub struct WalkResult {
    files: Vec<PathBuf>,
    directories: Vec<PathBuf>,
    error_count: usize,
}

impl WalkResult {
    /// Creates a new walk result.
    pub(crate) const fn new(
        files: Vec<PathBuf>,
        directories: Vec<PathBuf>,
        error_count: usize,
    ) -> Self {
        Self {
            files,
            directories,
            error_count,
        }
    }

    /// Returns the files found during traversal.
    #[must_use]
    pub fn files(&self) -> &[PathBuf] {
        &self.files
    }

    /// Returns the directories found during traversal.
    #[must_use]
    pub fn directories(&self) -> &[PathBuf] {
        &self.directories
    }

    /// Returns the number of errors encountered.
    #[must_use]
    pub const fn error_count(&self) -> usize {
        self.error_count
    }
}

/// Builds a `WalkBuilder` with the given options, using `filter_entry` for directory skipping.
pub(super) fn build_walker(root: &Path, options: &WalkOptions) -> WalkBuilder {
    let mut builder = WalkBuilder::new(root);

    // Configure depth
    if let Some(depth) = options.max_depth() {
        builder.max_depth(Some(depth));
    }

    // Configure basic options
    builder.follow_links(options.follow_links());
    builder.hidden(!options.include_hidden());

    // Configure gitignore handling
    builder.git_ignore(options.respect_gitignore());
    builder.git_global(options.respect_gitignore());
    builder.git_exclude(options.respect_gitignore());

    // Configure thread count
    if let Some(threads) = options.threads() {
        builder.threads(threads);
    }

    // Use filter_entry for efficient directory skipping (evaluated BEFORE descending)
    if !options.skip_dirs().is_empty() {
        let skip_dirs: Arc<Vec<String>> = Arc::new(options.skip_dirs().to_vec());
        builder.filter_entry(move |entry| {
            // If it's a directory, check if it should be skipped
            if entry.file_type().is_some_and(|ft| ft.is_dir())
                && let Some(name) = entry.file_name().to_str()
                && skip_dirs.iter().any(|skip| skip == name)
            {
                return false; // Don't descend into this directory
            }
            true
        });
    }

    builder
}

/// Performs parallel directory traversal using `ignore::WalkParallel`.
///
/// Uses flume channels for lock-free result collection, maximizing
/// parallel performance on Dev Drive and `NVMe` storage.
///
/// # Arguments
/// * `root` - The root directory to start traversal from
/// * `options` - Configuration options for the walk
///
/// # Returns
/// A `WalkResult` containing all files, directories, and error count.
///
/// # Errors
///
/// Returns an error if the root directory does not exist.
///
/// # Example
/// ```no_run
/// use mob_rs::utility::fs::walk::{parallel_walk, WalkOptions};
///
/// let result = parallel_walk("/path/to/project", &WalkOptions::for_build_tool())?;
/// println!("Found {} files", result.files().len());
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn parallel_walk<P: AsRef<Path>>(root: P, options: &WalkOptions) -> Result<WalkResult> {
    let root = root.as_ref();

    if !root.exists() {
        anyhow::bail!("root directory does not exist: {}", root.display());
    }

    // Use bounded channel to prevent memory exhaustion on huge directory trees
    // Buffer size of 1000 provides good throughput without excessive memory
    let (file_tx, file_rx) = bounded::<PathBuf>(1000);
    let (dir_tx, dir_rx) = bounded::<PathBuf>(1000);
    let error_count = Arc::new(AtomicUsize::new(0));

    let builder = build_walker(root, options);
    let parallel = builder.build_parallel();

    parallel.run(|| {
        let file_tx = file_tx.clone();
        let dir_tx = dir_tx.clone();
        let error_count = Arc::clone(&error_count);

        Box::new(move |entry_result| {
            match entry_result {
                Ok(entry) => {
                    let path = entry.path();

                    if entry.file_type().is_some_and(|ft| ft.is_dir()) {
                        // Send directory (ignore send errors - receiver might be dropped)
                        let _ = dir_tx.send(path.to_path_buf());
                    } else if entry.file_type().is_some_and(|ft| ft.is_file()) {
                        // Send file
                        let _ = file_tx.send(path.to_path_buf());
                    }
                }
                Err(e) => {
                    warn!(error = %e, "walk error");
                    error_count.fetch_add(1, Ordering::Relaxed);
                }
            }
            ignore::WalkState::Continue
        })
    });

    // Drop senders to signal completion
    drop(file_tx);
    drop(dir_tx);

    // Collect results from channels
    let files: Vec<PathBuf> = file_rx.iter().collect();
    let directories: Vec<PathBuf> = dir_rx.iter().collect();
    let error_count = error_count.load(Ordering::Relaxed);

    Ok(WalkResult::new(files, directories, error_count))
}

/// Performs parallel directory traversal with a callback for each entry.
///
/// This is a lower-level API that allows custom processing of each entry
/// as it's discovered, without collecting all results into memory.
///
/// # Arguments
/// * `root` - The root directory to start traversal from
/// * `options` - Configuration options for the walk
/// * `callback` - Function called for each file entry (not directories)
///
/// # Returns
/// Number of files processed, or an error.
///
/// # Errors
///
/// Returns an error if the root directory does not exist.
///
/// # Example
/// ```no_run
/// use mob_rs::utility::fs::walk::{parallel_walk_with_callback, WalkOptions};
/// use std::sync::atomic::{AtomicU64, Ordering};
///
/// let total_size = AtomicU64::new(0);
///
/// parallel_walk_with_callback(
///     "/path/to/project",
///     &WalkOptions::default(),
///     |path| {
///         if let Ok(meta) = path.metadata() {
///             total_size.fetch_add(meta.len(), Ordering::Relaxed);
///         }
///     }
/// )?;
///
/// println!("Total size: {} bytes", total_size.load(Ordering::Relaxed));
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn parallel_walk_with_callback<P, F>(
    root: P,
    options: &WalkOptions,
    callback: F,
) -> Result<usize>
where
    P: AsRef<Path>,
    F: Fn(&Path) + Send + Sync,
{
    let root = root.as_ref();

    if !root.exists() {
        anyhow::bail!("root directory does not exist: {}", root.display());
    }

    let callback = Arc::new(callback);
    let count = Arc::new(AtomicUsize::new(0));

    let builder = build_walker(root, options);
    let parallel = builder.build_parallel();

    parallel.run(|| {
        let callback = Arc::clone(&callback);
        let count = Arc::clone(&count);

        Box::new(move |entry_result| {
            if let Ok(entry) = entry_result
                && entry.file_type().is_some_and(|ft| ft.is_file())
            {
                callback(entry.path());
                count.fetch_add(1, Ordering::Relaxed);
            }
            ignore::WalkState::Continue
        })
    });

    Ok(count.load(Ordering::Relaxed))
}

/// Finds files matching a glob pattern using parallel traversal.
///
/// Uses the `wax` crate for modern, efficient glob matching combined
/// with `ignore::WalkParallel` for maximum throughput.
///
/// # Arguments
/// * `root` - The root directory to search from
/// * `pattern` - Glob pattern to match (e.g., "**/*.rs", "*.txt")
///
/// # Returns
/// A vector of matching file paths.
///
/// # Errors
///
/// Returns an error if:
/// - The root directory does not exist.
/// - The glob pattern is invalid.
///
/// # Example
/// ```no_run
/// use mob_rs::utility::fs::walk::find_files;
///
/// let rust_files = find_files("/path/to/project", "**/*.rs")?;
/// for file in rust_files {
///     println!("{}", file.display());
/// }
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn find_files<P: AsRef<Path>>(root: P, pattern: &str) -> Result<Vec<PathBuf>> {
    use wax::{Glob, Program};

    let root = root.as_ref();

    if !root.exists() {
        anyhow::bail!("root directory does not exist: {}", root.display());
    }

    let glob =
        Glob::new(pattern).map_err(|e| anyhow::anyhow!("invalid glob pattern '{pattern}': {e}"))?;

    // Use channel for lock-free collection
    let (tx, rx) = bounded::<PathBuf>(1000);
    let glob = Arc::new(glob);
    let root_path = root.to_path_buf();

    let builder = build_walker(root, &WalkOptions::default());
    let parallel = builder.build_parallel();

    parallel.run(|| {
        let tx = tx.clone();
        let glob = Arc::clone(&glob);
        let root_path = root_path.clone();

        Box::new(move |entry_result| {
            if let Ok(entry) = entry_result
                && entry.file_type().is_some_and(|ft| ft.is_file())
                && let Ok(rel_path) = entry.path().strip_prefix(&root_path)
                && glob.is_match(rel_path)
            {
                let _ = tx.send(entry.path().to_path_buf());
            }
            ignore::WalkState::Continue
        })
    });

    drop(tx);
    Ok(rx.iter().collect())
}
