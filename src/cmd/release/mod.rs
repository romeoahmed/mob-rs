// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Release command — packaging and distribution.
//!
//! ```text
//! devbuild --> bin/pdbs/src (.7z)
//! official --> bin/pdbs + installer
//! ```
//!
//! # Archive Contents
//!
//! | Archive | Source      | Excludes                       |
//! |---------|-------------|--------------------------------|
//! | bin     | install/bin | `__pycache__`                  |
//! | pdbs    | install/pdb | `__pycache__`                  |
//! | src     | build/*     | `.git`, `*.dll`, `*.exe`, etc. |

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::error::Result;
use anyhow::Context;
use tokio::fs;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use crate::cli::release::{DevbuildArgs, OfficialArgs, ReleaseArgs, ReleaseMode};
use crate::config::Config;
use crate::git::cmd::checkout;
use crate::git::discovery::get_repos;
use crate::git::ops::remote_branch_exists;
use crate::task::Task;
use crate::task::manager::TaskManager;
use crate::task::tasks::explorerpp::ExplorerPPTask;
use crate::task::tasks::installer::InstallerTask;
use crate::task::tasks::licenses::LicensesTask;
use crate::task::tasks::modorganizer::ModOrganizerTask;
use crate::task::tasks::stylesheets::StylesheetsTask;
use crate::task::tasks::translations::TranslationsTask;
use crate::task::tasks::usvfs::UsvfsTask;
use crate::task::tools::packer::PackerTool;
use crate::task::tools::{Tool, ToolContext};

mod version;

const BIN_EXCLUDES: &[&str] = &["__pycache__"];
const PDB_EXCLUDES: &[&str] = &["__pycache__"];
const SRC_EXCLUDES: &[&str] = &[
    r"\..*",
    "explorer++",
    "stylesheets",
    "transifex-translations",
    "*.log",
    "*.tlog",
    "*.dll",
    "*.exe",
    "*.lib",
    "*.obj",
    "*.ts",
    "*.aps",
    "bin",
    "lib",
    "vsbuild",
    "vsbuild32",
    "vsbuild64",
];

/// Main handler for release command.
///
/// # Errors
///
/// Returns an error if:
/// - Version determination fails.
/// - Output directory cannot be resolved or created.
/// - Creation of binary, PDB, or source archives fails.
/// - No repositories are found for an official release.
/// - Repository operations (git checkout, etc.) fail.
pub async fn run_release_command(args: &ReleaseArgs, config: &Config, dry_run: bool) -> Result<()> {
    match &args.mode {
        ReleaseMode::Devbuild(devbuild) => run_devbuild(devbuild, config, dry_run).await,
        ReleaseMode::Official(official) => run_official(official, config, dry_run).await,
    }
}

async fn run_devbuild(args: &DevbuildArgs, config: &Config, dry_run: bool) -> Result<()> {
    let version = version::determine_version(args, config).await?;
    let output_dir = resolve_output_dir(args, config)?;

    ensure_output_dir(&output_dir, dry_run).await?;

    let suffix = args.suffix.as_deref();
    let config = Arc::new(config.clone());
    let tool_ctx = ToolContext::new(Arc::clone(&config), CancellationToken::new(), dry_run);

    info!(version = %version, output_dir = %output_dir.display(), "Preparing devbuild release");

    if args.create_bin() {
        let install_bin = config
            .paths
            .install_bin
            .as_ref()
            .context("paths.install_bin not configured")?;
        let archive_path = output_dir.join(archive_name(&version, suffix, None));
        ensure_output_file(&archive_path, args.force)?;
        create_directory_archive(
            &tool_ctx,
            install_bin,
            &archive_path,
            BIN_EXCLUDES,
            "install/bin",
        )
        .await?;
    }

    if args.create_pdbs() {
        let install_pdbs = config
            .paths
            .install_pdbs
            .as_ref()
            .context("paths.install_pdbs not configured")?;
        let archive_path = output_dir.join(archive_name(&version, suffix, Some("pdbs")));
        ensure_output_file(&archive_path, args.force)?;
        create_directory_archive(
            &tool_ctx,
            install_pdbs,
            &archive_path,
            PDB_EXCLUDES,
            "install/pdbs",
        )
        .await?;
    }

    if args.create_src() {
        let source_root = modorganizer_super_dir(config.as_ref())?;
        let archive_path = output_dir.join(archive_name(&version, suffix, Some("src")));
        ensure_output_file(&archive_path, args.force)?;
        create_directory_archive(
            &tool_ctx,
            &source_root,
            &archive_path,
            SRC_EXCLUDES,
            "modorganizer_super",
        )
        .await?;
    }

    if args.copy_installer() {
        let installer_dir = config
            .paths
            .install_installer
            .as_ref()
            .context("paths.install_installer not configured")?;
        copy_installer_files(installer_dir, &output_dir, args.force, dry_run).await?;
    }

    Ok(())
}

async fn run_official(args: &OfficialArgs, config: &Config, dry_run: bool) -> Result<()> {
    let repos = get_repos(config).context("failed to discover repositories")?;
    let repo_count = repos.len();

    if repos.is_empty() {
        anyhow::bail!("no repositories found under paths.build; run build/fetch first");
    }

    validate_official_branch(&repos, repo_count, args, config)?;
    checkout_official_repos(&repos, args, dry_run)?;
    run_official_build_pipeline(config, dry_run, args.build_installer()).await?;
    create_official_archives(args, config, dry_run).await
}

fn validate_official_branch(
    repos: &[PathBuf],
    repo_count: usize,
    args: &OfficialArgs,
    config: &Config,
) -> Result<()> {
    info!(
        branch = %args.branch,
        repos = repo_count,
        "Validating branch exists on all repositories"
    );

    let mut missing = Vec::new();
    for repo in repos {
        let repo_name = repo
            .file_name()
            .and_then(|name| name.to_str())
            .with_context(|| format!("invalid repo path: {}", repo.display()))?;

        let url = format!(
            "{}{}/{}.git",
            config.task.git_url_prefix, config.task.mo_org, repo_name
        );

        debug!(repo = %repo_name, branch = %args.branch, "checking remote branch");

        if !remote_branch_exists(&url, &args.branch)
            .with_context(|| format!("failed to check branch for {repo_name}"))?
        {
            missing.push(repo_name.to_string());
        }
    }

    if !missing.is_empty() {
        let missing_list = missing.join(", ");
        anyhow::bail!(
            "branch '{}' not found for repositories: {}",
            args.branch,
            missing_list
        );
    }

    info!(
        branch = %args.branch,
        count = repo_count,
        "All repos have the required branch; proceeding with official release"
    );

    Ok(())
}

fn checkout_official_repos(repos: &[PathBuf], args: &OfficialArgs, dry_run: bool) -> Result<()> {
    info!(branch = %args.branch, "Checking out all repositories to branch");

    if dry_run {
        for repo in repos {
            let repo_name = repo
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("unknown");
            info!(repo = %repo_name, branch = %args.branch, "[DRY-RUN] would checkout");
        }
        return Ok(());
    }

    for repo in repos {
        let repo_name = repo
            .file_name()
            .and_then(|name| name.to_str())
            .with_context(|| format!("invalid repo path: {}", repo.display()))?;

        debug!(repo = %repo_name, branch = %args.branch, "checking out");

        checkout(repo, &args.branch).with_context(|| {
            format!("failed to checkout {} to branch {}", repo_name, args.branch)
        })?;
    }
    info!("All repositories checked out successfully");

    Ok(())
}

async fn run_official_build_pipeline(
    config: &Config,
    dry_run: bool,
    build_installer: bool,
) -> Result<()> {
    info!("Starting full build pipeline");

    let config = Arc::new(config.clone());
    let mut manager = TaskManager::new(Arc::clone(&config))
        .with_dry_run(dry_run)
        .with_do_fetch(true)
        .with_do_build(true);

    // Add all standard build tasks in the correct order
    // These mirror the BUILTIN_TASKS from main.rs
    manager.add(Task::Usvfs(UsvfsTask::new()));
    manager.add(Task::ModOrganizer(ModOrganizerTask::new(
        "modorganizer".to_string(),
    )));
    manager.add(Task::Stylesheets(StylesheetsTask::new()));
    manager.add(Task::ExplorerPP(ExplorerPPTask::new()));
    manager.add(Task::Licenses(LicensesTask::new()));
    manager.add(Task::Translations(TranslationsTask::new()));

    // Build installer if requested
    if build_installer {
        manager.add(Task::Installer(InstallerTask::new()));
    }

    manager.run_all().await.context("build pipeline failed")?;

    info!("Build completed successfully");

    Ok(())
}

async fn create_official_archives(
    args: &OfficialArgs,
    config: &Config,
    dry_run: bool,
) -> Result<()> {
    let output_dir = resolve_official_output_dir(args, config)?;
    ensure_output_dir(&output_dir, dry_run).await?;

    let version = version::determine_official_version(config).await?;
    info!(version = %version, output_dir = %output_dir.display(), "Creating release archives");

    let config = Arc::new(config.clone());
    let tool_ctx = ToolContext::new(Arc::clone(&config), CancellationToken::new(), dry_run);

    if args.create_bin() {
        let install_bin = config
            .paths
            .install_bin
            .as_ref()
            .context("paths.install_bin not configured")?;
        let archive_path = output_dir.join(archive_name(&version, None, None));
        ensure_output_file(&archive_path, args.force)?;
        create_directory_archive(
            &tool_ctx,
            install_bin,
            &archive_path,
            BIN_EXCLUDES,
            "install/bin",
        )
        .await?;
    }

    if args.create_pdbs() {
        let install_pdbs = config
            .paths
            .install_pdbs
            .as_ref()
            .context("paths.install_pdbs not configured")?;
        let archive_path = output_dir.join(archive_name(&version, None, Some("pdbs")));
        ensure_output_file(&archive_path, args.force)?;
        create_directory_archive(
            &tool_ctx,
            install_pdbs,
            &archive_path,
            PDB_EXCLUDES,
            "install/pdbs",
        )
        .await?;
    }

    // Phase 5: Copy installer to output dir
    if args.build_installer() {
        let installer_dir = config
            .paths
            .install_installer
            .as_ref()
            .context("paths.install_installer not configured")?;
        copy_installer_files(installer_dir, &output_dir, args.force, dry_run).await?;
    }

    info!(
        version = %version,
        output_dir = %output_dir.display(),
        "Official release completed successfully"
    );

    Ok(())
}

fn resolve_official_output_dir(args: &OfficialArgs, config: &Config) -> Result<PathBuf> {
    if let Some(dir) = &args.output_dir {
        return Ok(dir.clone());
    }

    let prefix = config
        .paths
        .prefix()
        .context("paths.prefix not configured")?;
    Ok(prefix.join("releases"))
}

/// Finds `.exe` files in `installer_dir`, sorts them, and copies each to
/// `output_dir`. Warns and returns `Ok(())` when the directory is missing or
/// contains no executables.
async fn copy_installer_files(
    installer_dir: &Path,
    output_dir: &Path,
    force: bool,
    dry_run: bool,
) -> Result<()> {
    if !installer_dir.exists() {
        warn!(
            path = %installer_dir.display(),
            "Installer directory not found; skipping copy"
        );
        return Ok(());
    }

    let mut entries = fs::read_dir(installer_dir).await.with_context(|| {
        format!(
            "failed to read installer directory {}",
            installer_dir.display()
        )
    })?;

    let mut installers = Vec::new();
    while let Some(entry) = entries
        .next_entry()
        .await
        .with_context(|| format!("failed to read entry in {}", installer_dir.display()))?
    {
        let path = entry.path();
        if path.is_file() {
            let is_exe = path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"));
            if is_exe {
                installers.push(path);
            }
        }
    }

    if installers.is_empty() {
        warn!(
            path = %installer_dir.display(),
            "No installer executables found"
        );
        return Ok(());
    }

    if installers.len() > 1 {
        warn!(
            count = installers.len(),
            "Multiple installer executables found; copying all"
        );
    }

    installers.sort();

    for installer in installers {
        let filename = installer
            .file_name()
            .context("installer filename missing")?;
        let destination = output_dir.join(filename);
        ensure_output_file(&destination, force)?;

        if dry_run {
            info!(
                src = %installer.display(),
                dst = %destination.display(),
                "[DRY-RUN] would copy installer"
            );
            continue;
        }

        fs::copy(&installer, &destination).await.with_context(|| {
            format!(
                "failed to copy {} to {}",
                installer.display(),
                destination.display()
            )
        })?;

        info!(
            src = %installer.display(),
            dst = %destination.display(),
            "Copied installer"
        );
    }

    Ok(())
}

fn resolve_output_dir(args: &DevbuildArgs, config: &Config) -> Result<PathBuf> {
    if let Some(dir) = &args.output_dir {
        return Ok(dir.clone());
    }

    let prefix = config
        .paths
        .prefix()
        .context("paths.prefix not configured")?;
    Ok(prefix.join("releases"))
}

async fn ensure_output_dir(path: &Path, dry_run: bool) -> Result<()> {
    if path.exists() {
        if !path.is_dir() {
            anyhow::bail!("output path is not a directory: {}", path.display());
        }
        return Ok(());
    }

    if dry_run {
        info!(path = %path.display(), "[DRY-RUN] would create output directory");
        return Ok(());
    }

    fs::create_dir_all(path)
        .await
        .with_context(|| format!("failed to create output directory {}", path.display()))?;
    Ok(())
}

fn ensure_output_file(path: &Path, force: bool) -> Result<()> {
    if path.exists() && !force {
        anyhow::bail!(
            "output file already exists: {} (use --force to overwrite)",
            path.display()
        );
    }
    Ok(())
}

async fn create_directory_archive(
    tool_ctx: &ToolContext,
    base_dir: &Path,
    archive_path: &Path,
    excludes: &[&str],
    label: &str,
) -> Result<()> {
    if !base_dir.exists() {
        anyhow::bail!("{} directory not found: {}", label, base_dir.display());
    }

    info!(
        archive = %archive_path.display(),
        base_dir = %base_dir.display(),
        "Creating archive"
    );

    let packer = PackerTool::new()
        .archive(archive_path)
        .base_dir(base_dir)
        .exclude_patterns(excludes)
        .pack_dir_op();

    packer
        .run(tool_ctx)
        .await
        .with_context(|| format!("failed to create archive {}", archive_path.display()))?;

    Ok(())
}

fn modorganizer_super_dir(config: &Config) -> Result<PathBuf> {
    let build_dir = config
        .paths
        .build
        .as_ref()
        .context("paths.build not configured")?;
    Ok(build_dir.join("modorganizer_super"))
}

fn archive_name(version: &str, suffix: Option<&str>, what: Option<&str>) -> String {
    let mut parts = vec!["Mod.Organizer".to_string(), version.to_string()];

    if let Some(suffix) = suffix.filter(|s| !s.is_empty()) {
        parts.push(suffix.to_string());
    }

    if let Some(what) = what.filter(|s| !s.is_empty()) {
        parts.push(what.to_string());
    }

    format!("{}.7z", parts.join("-"))
}

#[cfg(test)]
mod tests;
