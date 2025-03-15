use anyhow::{Context, Result};
use rusqlite::{params, Connection, Transaction};
use std::path::Path;

use crate::models::{CommitInfo, RepositoryStats};

/// Initialize the SQLite database
pub fn init_db() -> Result<Connection> {
    let db_path = Path::new("repo_activity.db");
    
    // 连接到数据库
    let conn = Connection::open(db_path)
        .context("Failed to connect to SQLite database")?;
    
    // 启用外键约束
    conn.execute("PRAGMA foreign_keys = ON", [])?;
    
    // 创建表（如果不存在）
    conn.execute("
        CREATE TABLE IF NOT EXISTS repositories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            path TEXT NOT NULL,
            total_commits INTEGER NOT NULL,
            total_lines_added INTEGER NOT NULL,
            total_lines_removed INTEGER NOT NULL,
            first_commit_date TEXT NOT NULL,
            last_commit_date TEXT NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )
    ", [])
    .context("Failed to create repositories table")?;
    
    conn.execute("
        CREATE TABLE IF NOT EXISTS contributors (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            repository_id INTEGER NOT NULL,
            name TEXT NOT NULL,
            email TEXT,
            commits INTEGER NOT NULL,
            lines_added INTEGER NOT NULL,
            lines_removed INTEGER NOT NULL,
            first_commit_date TEXT NOT NULL,
            last_commit_date TEXT NOT NULL,
            FOREIGN KEY (repository_id) REFERENCES repositories(id)
        )
    ", [])
    .context("Failed to create contributors table")?;
    
    conn.execute("
        CREATE TABLE IF NOT EXISTS commits (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            repository_id INTEGER NOT NULL,
            hash TEXT NOT NULL,
            author TEXT NOT NULL,
            email TEXT,
            date TEXT NOT NULL,
            message TEXT,
            lines_added INTEGER NOT NULL,
            lines_removed INTEGER NOT NULL,
            files_changed INTEGER NOT NULL,
            FOREIGN KEY (repository_id) REFERENCES repositories(id)
        )
    ", [])
    .context("Failed to create commits table")?;
    
    Ok(conn)
}

/// Save repository statistics to the database
pub fn save_stats(mut conn: Connection, stats: &RepositoryStats) -> Result<()> {
    // 开始事务
    let tx = conn.transaction()
        .context("Failed to start database transaction")?;
    
    // 插入仓库信息
    tx.execute(
        "INSERT INTO repositories 
            (path, total_commits, total_lines_added, total_lines_removed, first_commit_date, last_commit_date) 
         VALUES (?, ?, ?, ?, ?, ?)",
        params![
            &stats.repo_path,
            stats.total_commits as i64,
            stats.total_lines_added as i64,
            stats.total_lines_removed as i64,
            stats.first_commit_date.to_rfc3339(),
            stats.last_commit_date.to_rfc3339()
        ],
    )
    .context("Failed to insert repository info")?;
    
    // 获取插入的仓库 ID
    let repo_id = tx.last_insert_rowid();
    
    // 插入贡献者信息
    for (name, contributor) in &stats.contributors {
        // 查找该贡献者的邮箱
        let email = stats.commits.iter()
            .find(|commit| commit.author == *name)
            .map(|commit| commit.email.clone())
            .unwrap_or_default();
            
        tx.execute(
            r#"INSERT INTO contributors 
                (repository_id, name, email, commits, lines_added, lines_removed, first_commit_date, last_commit_date) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#,
            params![
                repo_id,
                name,
                email,
                contributor.commits as i64,
                contributor.lines_added as i64,
                contributor.lines_removed as i64,
                contributor.first_commit.to_rfc3339(),
                contributor.last_commit.to_rfc3339()
            ],
        )
        .context("Failed to insert contributor info")?;
    }
    
    // 插入提交信息
    for commit in &stats.commits {
        tx.execute(
            "INSERT INTO commits 
                (repository_id, hash, author, email, date, message, lines_added, lines_removed, files_changed) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                repo_id,
                &commit.hash,
                &commit.author,
                &commit.email,
                commit.date.to_rfc3339(),
                &commit.message,
                commit.lines_added as i64,
                commit.lines_removed as i64,
                commit.files_changed as i64
            ],
        )
        .context("Failed to insert commit info")?;
    }
    
    // 提交事务
    tx.commit()
        .context("Failed to commit database transaction")?;
    
    Ok(())
}

/// Query repository statistics from the database
pub fn get_repository_stats(conn: &Connection, repo_id: i64) -> Result<RepositoryStats> {
    // 获取仓库基本信息
    let mut stmt = conn.prepare(
        "SELECT id, path, total_commits, total_lines_added, total_lines_removed, 
                first_commit_date, last_commit_date 
         FROM repositories 
         WHERE id = ?"
    )?;
    
    let repo_row = stmt.query_row(params![repo_id], |row| {
        Ok((
            row.get::<_, String>(1)?, // path
            row.get::<_, i64>(2)?,    // total_commits
            row.get::<_, i64>(3)?,    // total_lines_added
            row.get::<_, i64>(4)?,    // total_lines_removed
            row.get::<_, String>(5)?, // first_commit_date
            row.get::<_, String>(6)?, // last_commit_date
        ))
    }).context("Failed to find repository")?;
    
    let (path, total_commits, total_lines_added, total_lines_removed, 
         first_commit_date_str, last_commit_date_str) = repo_row;
    
    // 解析日期
    let first_commit_date = chrono::DateTime::parse_from_rfc3339(&first_commit_date_str)
        .context("Failed to parse first commit date")?
        .with_timezone(&chrono::Utc);
    
    let last_commit_date = chrono::DateTime::parse_from_rfc3339(&last_commit_date_str)
        .context("Failed to parse last commit date")?
        .with_timezone(&chrono::Utc);
    
    // 创建 RepositoryStats 对象
    let mut stats = crate::models::RepositoryStats {
        repo_path: path,
        total_commits: total_commits as usize,
        total_lines_added: total_lines_added as usize,
        total_lines_removed: total_lines_removed as usize,
        first_commit_date,
        last_commit_date,
        contributors: std::collections::HashMap::new(),
        commits: Vec::new(),
    };
    
    // 获取贡献者信息
    let mut stmt = conn.prepare(
        "SELECT name, email, commits, lines_added, lines_removed, 
                first_commit_date, last_commit_date 
         FROM contributors 
         WHERE repository_id = ?"
    )?;
    
    let contributors = stmt.query_map(params![repo_id], |row| {
        Ok((
            row.get::<_, String>(0)?, // name
            row.get::<_, Option<String>>(1)?, // email
            row.get::<_, i64>(2)?,    // commits
            row.get::<_, i64>(3)?,    // lines_added
            row.get::<_, i64>(4)?,    // lines_removed
            row.get::<_, String>(5)?, // first_commit_date
            row.get::<_, String>(6)?, // last_commit_date
        ))
    })?;
    
    for contributor_result in contributors {
        let (name, email, commits, lines_added, lines_removed, 
             first_commit_str, last_commit_str) = contributor_result?;
        
        // 解析日期
        let first_commit = chrono::DateTime::parse_from_rfc3339(&first_commit_str)
            .context("Failed to parse contributor first commit date")?
            .with_timezone(&chrono::Utc);
        
        let last_commit = chrono::DateTime::parse_from_rfc3339(&last_commit_str)
            .context("Failed to parse contributor last commit date")?
            .with_timezone(&chrono::Utc);
        
        // 添加贡献者
        stats.contributors.insert(name, crate::models::ContributorStats {
            commits: commits as usize,
            lines_added: lines_added as usize,
            lines_removed: lines_removed as usize,
            first_commit,
            last_commit,
        });
    }
    
    // 获取提交信息
    let mut stmt = conn.prepare(
        "SELECT hash, author, email, date, message, lines_added, lines_removed, files_changed 
         FROM commits 
         WHERE repository_id = ? 
         ORDER BY date DESC"
    )?;
    
    let commits = stmt.query_map(params![repo_id], |row| {
        Ok(crate::models::CommitInfo {
            hash: row.get(0)?,
            author: row.get(1)?,
            email: row.get(2)?,
            date: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            message: row.get(4)?,
            lines_added: row.get::<_, i64>(5)? as usize,
            lines_removed: row.get::<_, i64>(6)? as usize,
            files_changed: row.get::<_, i64>(7)? as usize,
        })
    })?;
    
    for commit_result in commits {
        stats.commits.push(commit_result?);
    }
    
    Ok(stats)
}