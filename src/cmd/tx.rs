// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Transifex command implementation for mob-rs.

use std::sync::Arc;

use crate::cli::tx::TxSubcommand;
use crate::cli::tx::{TxArgs, TxBuildArgs, TxGetArgs};
use crate::config::Config;
use crate::error::Result;
use crate::task::tasks::translations::discover_projects;
use crate::task::tools::lrelease::LreleaseTool;
use crate::task::tools::transifex::TransifexTool;
use crate::task::tools::{Tool, ToolContext};
use anyhow::bail;
use tokio_util::sync::CancellationToken;
use tracing::info;

/// Main handler for tx command.
///
/// # Errors
///
/// Returns an error if configuration fails or the tx tool fails.
pub async fn run_tx_command(args: &TxArgs, config: &Config, dry_run: bool) -> Result<()> {
    let config = Arc::new(config.clone());
    let cancel_token = CancellationToken::new();
    let ctx = ToolContext::new(Arc::clone(&config), cancel_token, dry_run);

    match &args.subcommand {
        TxSubcommand::Get(get_args) => run_tx_get(get_args, &config, &ctx).await,
        TxSubcommand::Build(build_args) => run_tx_build(build_args, &ctx).await,
    }
}

async fn run_tx_get(get_args: &TxGetArgs, config: &Config, ctx: &ToolContext) -> Result<()> {
    let key = if let Some(k) = &get_args.key {
        k.clone()
    } else {
        eprintln!("TX API key required (use -k/--key or TX_TOKEN environment variable)");
        bail!("transifex api key missing");
    };

    let team = if let Some(t) = &get_args.team {
        t.clone()
    } else {
        eprintln!("Transifex team required (use -t/--team)");
        bail!("transifex team missing");
    };

    let project = if let Some(p) = &get_args.project {
        p.clone()
    } else {
        eprintln!("Transifex project required (use -p/--project)");
        bail!("transifex project missing");
    };

    let url = get_args.url.as_ref().map_or_else(
        || format!("{}/{}/{}", config.transifex.url, team, project),
        std::clone::Clone::clone,
    );
    let minimum = get_args.minimum.unwrap_or(100);

    if ctx.is_dry_run() {
        info!(
            path = %get_args.path.display(),
            "[DRY-RUN] would initialize transifex directory"
        );
    } else {
        TransifexTool::new()
            .root(&get_args.path)
            .init_op()
            .run(ctx)
            .await
            .map_err(|e| {
                eprintln!("Failed to initialize transifex: {e}");
                e
            })?;
    }

    if ctx.is_dry_run() {
        info!(
            path = %get_args.path.display(),
            url = %url,
            "[DRY-RUN] would configure transifex remote"
        );
    } else {
        TransifexTool::new()
            .root(&get_args.path)
            .api_key(&key)
            .url(&url)
            .config_op()
            .run(ctx)
            .await
            .map_err(|e| {
                eprintln!("Failed to configure transifex: {e}");
                e
            })?;
    }

    if ctx.is_dry_run() {
        info!(
            path = %get_args.path.display(),
            minimum = minimum,
            force = get_args.force,
            "[DRY-RUN] would pull translations"
        );
    } else {
        TransifexTool::new()
            .root(&get_args.path)
            .api_key(&key)
            .minimum(minimum)
            .force(get_args.force)
            .pull_op()
            .run(ctx)
            .await
            .map_err(|e| {
                eprintln!("Failed to pull translations: {e}");
                e
            })?;
    }

    Ok(())
}

async fn run_tx_build(build_args: &TxBuildArgs, ctx: &ToolContext) -> Result<()> {
    let mut source = build_args.source.clone();
    let tx_dir = source.join(".tx");
    let translations_dir = source.join("translations");
    if tx_dir.exists() && translations_dir.exists() {
        source = translations_dir;
    }

    let projects = discover_projects(&source).await.map_err(|e| {
        eprintln!("Failed to discover translation projects: {e}");
        e
    })?;

    if projects.is_empty() {
        if ctx.is_dry_run() {
            info!(
                path = %source.display(),
                "[DRY-RUN] no translation projects found"
            );
        } else {
            eprintln!("No translation projects found in {}", source.display());
        }
        return Ok(());
    }

    if !ctx.is_dry_run() && !build_args.destination.exists() {
        tokio::fs::create_dir_all(&build_args.destination)
            .await
            .map_err(|e| {
                eprintln!(
                    "Failed to create destination directory {}: {}",
                    build_args.destination.display(),
                    e
                );
                e
            })?;
    }

    for project in projects {
        if ctx.is_dry_run() {
            info!(
                project = %project.name(),
                files = project.ts_files().len(),
                "[DRY-RUN] would compile translations"
            );
        } else {
            LreleaseTool::new()
                .project(project.name())
                .sources(project.ts_files())
                .output_dir(&build_args.destination)
                .run(ctx)
                .await
                .map_err(|e| {
                    eprintln!(
                        "Failed to compile translations for project {}: {}",
                        project.name(),
                        e
                    );
                    e
                })?;
        }
    }

    Ok(())
}
