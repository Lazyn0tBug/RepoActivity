use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;

mod git;
mod db;
mod models;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the git repository
    #[arg(short, long)]
    repo_path: PathBuf,

    /// Start date for analysis (YYYY-MM-DD)
    #[arg(short, long)]
    start_date: Option<String>,

    /// End date for analysis (YYYY-MM-DD)
    #[arg(short, long)]
    end_date: Option<String>,
}

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    // Parse command line arguments
    let args = Args::parse();
    
    // Initialize database
    let db_pool = db::init_db()
        .context("Failed to initialize database")?;
    
    // Process repository
    let repo_stats = git::analyze_repository(
        &args.repo_path,
        args.start_date.as_deref(),
        args.end_date.as_deref(),
    ).context("Failed to analyze repository")?;
    
    // Save results to database
    db::save_stats(db_pool, &repo_stats)
        .context("Failed to save stats to database")?;
    
    // Print summary
    print_summary(&repo_stats);
    
    Ok(())
}

fn print_summary(stats: &models::RepositoryStats) {
    println!("\nRepository Analysis Summary:");
    println!("---------------------------");
    println!("Total commits: {}", stats.total_commits);
    println!("Total contributors: {}", stats.contributors.len());
    println!("Total lines added: {}", stats.total_lines_added);
    println!("Total lines removed: {}", stats.total_lines_removed);
    
    println!("\nTop contributors:");
    let mut contributors: Vec<_> = stats.contributors.iter().collect();
    contributors.sort_by(|a, b| b.1.commits.cmp(&a.1.commits));
    
    for (i, (name, stats)) in contributors.iter().take(5).enumerate() {
        println!("{:>2}. {}: {} commits, +{} -{}  lines", 
            i + 1, name, stats.commits, stats.lines_added, stats.lines_removed);
    }
}