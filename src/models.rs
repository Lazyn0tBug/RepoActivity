use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    pub hash: String,
    pub author: String,
    pub email: String,
    pub date: DateTime<Utc>,
    pub message: String,
    pub lines_added: usize,
    pub lines_removed: usize,
    pub files_changed: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributorStats {
    pub commits: usize,
    pub lines_added: usize,
    pub lines_removed: usize,
    pub first_commit: DateTime<Utc>,
    pub last_commit: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryStats {
    pub repo_path: String,
    pub total_commits: usize,
    pub total_lines_added: usize,
    pub total_lines_removed: usize,
    pub first_commit_date: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub last_commit_date: DateTime<Utc>,
    pub contributors: HashMap<String, ContributorStats>,
    pub commits: Vec<CommitInfo>,
}

impl RepositoryStats {
    pub fn new(repo_path: &str) -> Self {
        Self {
            repo_path: repo_path.to_string(),
            total_commits: 0,
            total_lines_added: 0,
            total_lines_removed: 0,
            first_commit_date: Utc::now(),
            last_commit_date: Utc::now(),
            contributors: HashMap::new(),
            commits: Vec::new(),
        }
    }

    pub fn add_commit(&mut self, commit: CommitInfo) {
        // Update repository totals
        self.total_commits += 1;
        self.total_lines_added += commit.lines_added;
        self.total_lines_removed += commit.lines_removed;
        
        // Update first/last commit dates
        if self.total_commits == 1 || commit.date < self.first_commit_date {
            self.first_commit_date = commit.date;
        }
        if commit.date > self.last_commit_date {
            self.last_commit_date = commit.date;
        }
        
        // Update contributor stats
        let contributor = self.contributors
            .entry(commit.author.clone())
            .or_insert_with(|| ContributorStats {
                commits: 0,
                lines_added: 0,
                lines_removed: 0,
                first_commit: commit.date,
                last_commit: commit.date,
            });
        
        contributor.commits += 1;
        contributor.lines_added += commit.lines_added;
        contributor.lines_removed += commit.lines_removed;
        
        if commit.date < contributor.first_commit {
            contributor.first_commit = commit.date;
        }
        if commit.date > contributor.last_commit {
            contributor.last_commit = commit.date;
        }
        
        // Store commit info
        self.commits.push(commit);
    }
}