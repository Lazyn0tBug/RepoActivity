use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use futures::stream::{self, StreamExt};
use git2::{Diff, DiffOptions, Repository};
use std::path::Path;
use std::str;
use tokio::task;

use crate::models::{CommitInfo, RepositoryStats};

/// Analyzes a git repository and returns statistics
pub fn analyze_repository(
    repo_path: &Path,
    start_date_str: Option<&str>,
    end_date_str: Option<&str>,
) -> Result<RepositoryStats> {
    // Parse date filters if provided
    let start_date = match start_date_str {
        Some(date_str) => Some(parse_date(date_str)?),
        None => None,
    };
    
    let end_date = match end_date_str {
        Some(date_str) => Some(parse_date(date_str)?),
        None => None,
    };
    
    // Open repository
    let repo_path_str = repo_path.to_string_lossy().to_string();
    let repo = Repository::open(&repo_path_str).context("Failed to open repository")?;
    
    // Create stats object
    let mut stats = RepositoryStats::new(&repo_path.to_string_lossy());
    
    // Get all commits
    let commits = get_commits(&repo, start_date, end_date)?;
    
    // Process commits
    for commit_id in commits {
        match process_commit(&repo, commit_id) {
            Ok(commit_info) => stats.add_commit(commit_info),
            Err(e) => eprintln!("Error processing commit: {}", e),
        }
    }
    
    Ok(stats)
}

/// Gets all commits in the repository that match the date filters
fn get_commits(
    repo: &Repository,
    start_date: Option<DateTime<Utc>>,
    end_date: Option<DateTime<Utc>>,
) -> Result<Vec<git2::Oid>> {
    // 直接在当前线程上下文中执行
    let mut revwalk = repo.revwalk().context("Failed to create revision walker")?;
    revwalk.push_head().context("Failed to push HEAD to revision walker")?;
    
    // Collect all commit OIDs that match our date filters
    let mut commits = Vec::new();
    for oid_result in revwalk {
        let oid = oid_result.context("Failed to get commit OID")?;
        let commit = repo.find_commit(oid).context("Failed to find commit")?;
        
        let commit_time = commit.time();
        let dt = git_time_to_datetime(commit_time.seconds());
        
        // Apply date filters if provided
        if let Some(start) = start_date {
            if dt < start {
                continue;
            }
        }
        
        if let Some(end) = end_date {
            if dt > end {
                continue;
            }
        }
        
        commits.push(oid);
    }
    
    Ok(commits)
}

/// Processes a single commit to extract its information
fn process_commit(repo: &Repository, commit_id: git2::Oid) -> Result<CommitInfo> {
    let commit = repo.find_commit(commit_id).context("Failed to find commit")?;
    
    // Get commit metadata
    let author = commit.author();
    let name = author.name().unwrap_or("Unknown").to_string();
    let email = author.email().unwrap_or("Unknown").to_string();
    let message = commit.message().unwrap_or("").to_string();
    let hash = commit.id().to_string();
    let date = git_time_to_datetime(commit.time().seconds());
    
    // Get diff stats for this commit
    let (lines_added, lines_removed, files_changed) = get_commit_diff_stats(repo, &commit)?;
    
    Ok(CommitInfo {
        hash,
        author: name,
        email,
        date,
        message,
        lines_added,
        lines_removed,
        files_changed,
    })
}

/// Gets diff statistics for a commit
fn get_commit_diff_stats(repo: &Repository, commit: &git2::Commit) -> Result<(usize, usize, usize)> {
    let mut lines_added = 0;
    let mut lines_removed = 0;
    let mut files_changed = 0;
    
    // Get parent commit (if any)
    let parent = if commit.parent_count() > 0 {
        Some(commit.parent(0).context("Failed to get parent commit")?)
    } else {
        None
    };
    
    // Get commit tree
    let commit_tree = commit.tree().context("Failed to get commit tree")?;
    
    // Create diff
    let diff = match parent {
        Some(parent) => {
            let parent_tree = parent.tree().context("Failed to get parent tree")?;
            let mut diff_opts = DiffOptions::new();
            diff_opts.context_lines(0).patience(true);
            repo.diff_tree_to_tree(
                Some(&parent_tree),
                Some(&commit_tree),
                Some(&mut diff_opts),
            )
        }
        None => {
            let mut diff_opts = DiffOptions::new();
            diff_opts.context_lines(0).patience(true);
            repo.diff_tree_to_tree(None, Some(&commit_tree), Some(&mut diff_opts))
        }
    }
    .context("Failed to create diff")?;
    
    // Get diff stats
    files_changed = diff.deltas().len();
    
    // Process each hunk in the diff to count lines added/removed
    diff.foreach(
        &mut |_, _| true,                                // file_cb
        None,                                            // binary_cb
        None,                                            // hunk_cb
        Some(&mut |_delta, _delta_idx, line| {
            match line.origin() {
                '+' => lines_added += 1,
                '-' => lines_removed += 1,
                _ => {}
            }
            true
        }),
    )
    .context("Failed to process diff")?;
    
    Ok((lines_added, lines_removed, files_changed))
}

/// Converts a git timestamp to a chrono DateTime
fn git_time_to_datetime(time: i64) -> DateTime<Utc> {
    match DateTime::from_timestamp(time, 0) {
        Some(ndt) => ndt,
        None => Utc::now(), // Fallback to current time if conversion fails
    }
}

/// Parses a date string in YYYY-MM-DD format
fn parse_date(date_str: &str) -> Result<DateTime<Utc>> {
    let naive_date = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .context("Failed to parse date, expected format YYYY-MM-DD")?;
    
    let naive_datetime = naive_date.and_hms_opt(0, 0, 0).unwrap();
    Ok(Utc.from_utc_datetime(&naive_datetime))
}