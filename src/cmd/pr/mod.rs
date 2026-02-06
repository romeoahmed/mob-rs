// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! PR command — fetch GitHub PRs and checkout locally.
//!
//! ```text
//! GitHub API --> PrMatch (PR + path) --> local fetch+checkout
//! ```
//!
//! # Key Types
//!
//! | Type             | Purpose                         |
//! |------------------|---------------------------------|
//! | `PrInfo`         | GitHub PR data from API         |
//! | `PrMatch`        | Matched PR with local repo path |
//! | `SearchResponse` | GitHub search API response      |

use crate::cli::pr::{PrArgs, PrOperation};
use crate::config::Config;
use crate::error::NetworkError;
use crate::error::Result;
use crate::git::cmd::checkout;
use crate::git::ops::fetch_refspec;
use crate::git::query::is_git_repo;
use anyhow::Context;
use reqwest::Client;
use serde::Deserialize;
use tracing::{debug, info, warn};

/// GitHub PR information from API
#[derive(Debug, Deserialize)]
pub struct PrInfo {
    pub number: u64,
    pub title: String,
    pub state: String,
    pub head: PrHead,
}

/// PR head information (branch details)
#[derive(Debug, Deserialize)]
pub struct PrHead {
    pub repo: Option<PrRepo>,
    #[serde(rename = "ref")]
    pub ref_name: String,
    pub sha: String,
}

/// PR repository information
#[derive(Debug, Deserialize)]
pub struct PrRepo {
    pub clone_url: String,
    pub full_name: String,
}

/// Search result for PRs
#[derive(Debug, Deserialize)]
pub struct SearchResponse {
    pub total_count: u64,
    pub items: Vec<SearchItem>,
}

/// Individual search result item
#[derive(Debug, Deserialize)]
pub struct SearchItem {
    pub number: u64,
    pub title: String,
    pub repository_url: String,
    pub state: String,
    pub pull_request: Option<PullRequestLink>,
}

/// Pull request link in search results
#[derive(Debug, Deserialize)]
pub struct PullRequestLink {
    pub url: String,
}

/// Matched PR across repositories
#[derive(Debug)]
pub struct PrMatch {
    repo: String,
    pr_number: u64,
    title: String,
    head_ref: String,
    head_sha: String,
    clone_url: String,
    local_path: Option<std::path::PathBuf>,
}

impl PrMatch {
    /// Creates a new `PrMatch`.
    #[must_use]
    pub const fn new(
        repo: String,
        pr_number: u64,
        title: String,
        head_ref: String,
        head_sha: String,
        clone_url: String,
        local_path: Option<std::path::PathBuf>,
    ) -> Self {
        Self {
            repo,
            pr_number,
            title,
            head_ref,
            head_sha,
            clone_url,
            local_path,
        }
    }

    /// Returns the repository name.
    #[must_use]
    pub fn repo(&self) -> &str {
        &self.repo
    }

    /// Returns the PR number.
    #[must_use]
    pub const fn pr_number(&self) -> u64 {
        self.pr_number
    }

    /// Returns the PR title.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Returns the head ref (branch name).
    #[must_use]
    pub fn head_ref(&self) -> &str {
        &self.head_ref
    }

    /// Returns the head SHA.
    #[must_use]
    pub fn head_sha(&self) -> &str {
        &self.head_sha
    }

    /// Returns the clone URL.
    #[must_use]
    pub fn clone_url(&self) -> &str {
        &self.clone_url
    }

    /// Returns the local path, if the repo is found locally.
    #[must_use]
    pub fn local_path(&self) -> Option<&std::path::Path> {
        self.local_path.as_deref()
    }
}

/// Parse PR argument format: "task/123" or just "123"
///
/// # Examples
/// - "modorganizer/123" -> Ok((Some("modorganizer"), 123))
/// - "123" -> Ok((None, 123))
/// - "invalid" -> Err(...)
///
/// # Errors
///
/// Returns an error if the PR number is not a valid unsigned integer.
pub fn parse_pr_arg(pr: &str) -> Result<(Option<String>, u64)> {
    if let Some((repo, num)) = pr.split_once('/') {
        let number = num
            .parse::<u64>()
            .with_context(|| format!("invalid PR number: {num}"))?;
        Ok((Some(repo.to_string()), number))
    } else {
        let number = pr
            .parse::<u64>()
            .with_context(|| format!("invalid PR number: {pr}"))?;
        Ok((None, number))
    }
}

/// Get PR info from GitHub API
///
/// # Errors
///
/// Returns an error if:
/// - The network request fails.
/// - The GitHub API returns a non-success status code.
/// - The response body cannot be parsed as `PrInfo`.
pub async fn get_pr_info(
    client: &Client,
    token: &str,
    org: &str,
    repo: &str,
    pr: u64,
) -> Result<PrInfo> {
    let url = format!("https://api.github.com/repos/{org}/{repo}/pulls/{pr}");

    debug!(org, repo, pr, "fetching PR info from GitHub API");

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {token}"))
        .header("Accept", "application/vnd.github.v3+json")
        .header(
            "User-Agent",
            format!("mob-rs/{}", env!("CARGO_PKG_VERSION")),
        )
        .send()
        .await
        .with_context(|| format!("failed to request PR info from {url}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(NetworkError::HttpError {
            status: status.as_u16(),
            url: format!("{url} (error: {body})"),
        }
        .into());
    }

    let pr_info = response
        .json::<PrInfo>()
        .await
        .with_context(|| "failed to parse PR info from GitHub API")?;

    Ok(pr_info)
}

/// Search for matching PRs across repos
///
/// # Errors
///
/// Returns an error if:
/// - The network request fails.
/// - The GitHub API returns a non-success status code.
/// - The response body cannot be parsed as `SearchResponse`.
pub async fn search_prs(
    client: &Client,
    token: &str,
    org: &str,
    author: Option<&str>,
    pr_number: Option<u64>,
) -> Result<Vec<SearchItem>> {
    let mut query_parts = vec![format!("org:{}", org), "type:pr".to_string()];

    if let Some(author) = author {
        query_parts.push(format!("author:{author}"));
    }

    if let Some(number) = pr_number {
        // Search by PR number in title/body (GitHub search limitation)
        query_parts.push(format!("{number} in:title,body"));
    }

    let query = query_parts.join(" ");
    // URL-encode the query manually
    let encoded_query = query.replace(' ', "+").replace(':', "%3A");
    let url = format!("https://api.github.com/search/issues?q={encoded_query}&per_page=100");

    debug!(query, "searching GitHub for PRs");

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {token}"))
        .header("Accept", "application/vnd.github.v3+json")
        .header(
            "User-Agent",
            format!("mob-rs/{}", env!("CARGO_PKG_VERSION")),
        )
        .send()
        .await
        .with_context(|| format!("failed to search GitHub for PRs: {url}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(NetworkError::HttpError {
            status: status.as_u16(),
            url: format!("{url} (error: {body})"),
        }
        .into());
    }

    let search_result = response
        .json::<SearchResponse>()
        .await
        .with_context(|| "failed to parse GitHub search results")?;

    debug!(
        total = search_result.total_count,
        found = search_result.items.len(),
        "GitHub search completed"
    );

    Ok(search_result.items)
}

/// Find local repository path for a given repo name
fn find_local_repo(config: &Config, repo_name: &str) -> Option<std::path::PathBuf> {
    let build_path = config.paths.build.as_ref()?;

    // Check usvfs
    if repo_name == "usvfs" {
        let usvfs_path = build_path.join("usvfs");
        if usvfs_path.exists() && is_git_repo(&usvfs_path) {
            return Some(usvfs_path);
        }
    }

    // Check modorganizer_super
    let super_path = build_path.join("modorganizer_super").join(repo_name);
    if super_path.exists() && is_git_repo(&super_path) {
        return Some(super_path);
    }

    None
}

/// Convert search items to `PrMatch` with local paths
async fn items_to_matches(
    client: &Client,
    token: &str,
    items: Vec<SearchItem>,
    config: &Config,
) -> Result<Vec<PrMatch>> {
    let mut matches = Vec::new();

    for item in items {
        // Extract org/repo from repository_url
        // Format: https://api.github.com/repos/{org}/{repo}
        let parts: Vec<&str> = item.repository_url.split('/').collect();
        if parts.len() < 2 {
            warn!(url = %item.repository_url, "invalid repository URL format");
            continue;
        }
        let org = parts[parts.len() - 2].to_string();
        let repo = parts[parts.len() - 1].to_string();

        // Get detailed PR info to get head ref
        let pr_info = match get_pr_info(client, token, &org, &repo, item.number).await {
            Ok(info) => info,
            Err(e) => {
                warn!(org, repo, pr = item.number, error = %e, "failed to get PR details");
                continue;
            }
        };

        let clone_url = pr_info
            .head
            .repo
            .as_ref()
            .map(|r| r.clone_url.clone())
            .unwrap_or_default();

        matches.push(PrMatch::new(
            repo.clone(),
            item.number,
            item.title,
            pr_info.head.ref_name,
            pr_info.head.sha,
            clone_url,
            find_local_repo(config, &repo),
        ));
    }

    Ok(matches)
}

/// Main handler for PR command
///
/// # Errors
///
/// Returns an error if:
/// - The GitHub token is missing.
/// - The PR argument is invalid.
/// - Any GitHub API request fails.
/// - Any git operation (fetch, checkout) fails.
pub async fn run_pr_command(args: &PrArgs, config: &Config) -> Result<()> {
    // Require GitHub token
    let token = args
        .github_token
        .as_ref()
        .context("GitHub token required (use --github-token or GITHUB_TOKEN env)")?;

    let (repo_filter, pr_number) = parse_pr_arg(&args.pr)
        .with_context(|| format!("failed to parse PR argument: {}", args.pr))?;

    let client = reqwest::Client::new();

    match args.operation {
        PrOperation::Find => run_pr_find(&client, token, repo_filter, pr_number, config).await,
        PrOperation::Pull => run_pr_pull(&client, token, repo_filter, pr_number, config).await,
        PrOperation::Revert => run_pr_revert(&client, token, repo_filter, pr_number, config).await,
    }
}

async fn run_pr_find(
    client: &Client,
    token: &str,
    repo_filter: Option<String>,
    pr_number: u64,
    config: &Config,
) -> Result<()> {
    info!("Searching for matching PRs...");

    // Determine org from config
    let org = &config.task.mo_org;

    let items = if let Some(ref repo) = repo_filter {
        // Specific repo - fetch PR directly
        let pr_info = get_pr_info(client, token, org, repo, pr_number).await?;
        info!(
            org,
            repo,
            pr = pr_number,
            title = %pr_info.title,
            state = %pr_info.state,
            "found PR"
        );
        vec![]
    } else {
        // Search across all repos in org
        search_prs(client, token, org, None, Some(pr_number)).await?
    };

    let matches = items_to_matches(client, token, items, config).await?;

    if matches.is_empty() && repo_filter.is_none() {
        warn!(org = %org, "No matching PRs found in organization");
        return Ok(());
    }

    println!("\nAffected repositories:");
    println!("{:-<80}", "");

    if let Some(ref repo) = repo_filter {
        // Single repo case
        let local_path = find_local_repo(config, repo);
        if let Some(path) = local_path {
            println!("{:<30} {} (local: {})", repo, pr_number, path.display());
        } else {
            println!("{repo:<30} {pr_number} (not found locally)");
        }
    } else {
        // Multiple repos from search
        for m in &matches {
            if let Some(path) = m.local_path() {
                println!(
                    "{:<30} #{} {} ({})",
                    m.repo(),
                    m.pr_number(),
                    m.title(),
                    path.display()
                );
            } else {
                println!(
                    "{:<30} #{} {} (not found locally)",
                    m.repo(),
                    m.pr_number(),
                    m.title()
                );
            }
        }
    }

    Ok(())
}

async fn run_pr_pull(
    client: &Client,
    token: &str,
    repo_filter: Option<String>,
    pr_number: u64,
    config: &Config,
) -> Result<()> {
    info!("Fetching and checking out PR...");

    let org = &config.task.mo_org;

    let matches = if let Some(ref repo) = repo_filter {
        // Specific repo
        let pr_info = get_pr_info(client, token, org, repo, pr_number).await?;
        let local_path = find_local_repo(config, repo);

        if local_path.is_none() {
            warn!(repo, "repository not found locally, skipping");
            return Ok(());
        }

        vec![PrMatch::new(
            repo.clone(),
            pr_number,
            pr_info.title,
            pr_info.head.ref_name,
            pr_info.head.sha,
            pr_info.head.repo.map(|r| r.clone_url).unwrap_or_default(),
            local_path,
        )]
    } else {
        // Search and convert
        let items = search_prs(client, token, org, None, Some(pr_number)).await?;
        items_to_matches(client, token, items, config).await?
    };

    for m in matches {
        let Some(local_path) = m.local_path() else {
            warn!(repo = %m.repo(), "repository not found locally, skipping");
            continue;
        };

        info!(
            repo = %m.repo(),
            pr = m.pr_number(),
            branch = %m.head_ref(),
            "fetching PR"
        );

        if config.global.dry {
            println!(
                "[DRY-RUN] Would fetch PR #{} from {} and checkout FETCH_HEAD",
                m.pr_number(),
                m.repo()
            );
            continue;
        }

        // Fetch the PR head using refspec
        let refspec = format!("refs/pull/{}/head", m.pr_number());
        fetch_refspec(local_path, m.clone_url(), &refspec)
            .with_context(|| format!("failed to fetch PR {} from {}", m.pr_number(), m.repo()))?;

        // Checkout FETCH_HEAD
        checkout(local_path, "FETCH_HEAD")
            .with_context(|| format!("failed to checkout FETCH_HEAD for {}", m.repo()))?;

        info!(
            repo = %m.repo(),
            pr = m.pr_number(),
            sha = %m.head_sha(),
            "checked out PR"
        );
    }

    Ok(())
}

async fn run_pr_revert(
    client: &Client,
    token: &str,
    repo_filter: Option<String>,
    pr_number: u64,
    config: &Config,
) -> Result<()> {
    info!("Reverting repositories to master...");

    let org = &config.task.mo_org;

    let matches = if let Some(ref repo) = repo_filter {
        // Specific repo
        let local_path = find_local_repo(config, repo);
        if local_path.is_none() {
            warn!(repo, "repository not found locally, skipping");
            return Ok(());
        }

        vec![PrMatch::new(
            repo.clone(),
            pr_number,
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            local_path,
        )]
    } else {
        // Search first
        let items = search_prs(client, token, org, None, Some(pr_number)).await?;
        items_to_matches(client, token, items, config).await?
    };

    for m in matches {
        let Some(local_path) = m.local_path() else {
            warn!(repo = %m.repo(), "repository not found locally, skipping");
            continue;
        };

        info!(repo = %m.repo(), "reverting to master");

        if config.global.dry {
            println!("[DRY-RUN] Would checkout master in {}", m.repo());
            continue;
        }

        // Checkout master
        checkout(local_path, "master")
            .with_context(|| format!("failed to checkout master for {}", m.repo()))?;

        info!(repo = %m.repo(), "reverted to master");
    }

    Ok(())
}

#[cfg(test)]
mod tests;
