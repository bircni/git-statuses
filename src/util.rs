use std::sync::Arc;

use comfy_table::{Attribute, Cell, Color, ContentArrangement, Table, presets};
use parking_lot::RwLock;
use rayon::iter::{IntoParallelRefIterator as _, ParallelIterator as _};
use walkdir::WalkDir;

use crate::{cli::Args, gitinfo::RepoInfo};

/// Scans the given directory (recursively if requested) for Git repositories and collects their status information.
///
/// # Arguments
/// * `args` - CLI arguments controlling the scan behavior.
///
/// # Returns
/// A vector of `RepoInfo` for each found repository, or an error.
pub fn find_repositories(args: &Args) -> anyhow::Result<Vec<RepoInfo>> {
    let min_depth = 1;
    let max_depth = if args.depth > 0 { args.depth } else { 1 };
    let walker = WalkDir::new(&args.dir)
        .min_depth(min_depth)
        .max_depth(max_depth)
        .follow_links(false)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .collect::<Vec<_>>();

    let repos: Arc<RwLock<Vec<RepoInfo>>> = Arc::new(RwLock::new(Vec::new()));

    walker.par_iter().try_for_each(|entry| {
        let path = entry.path();
        if !path.is_dir() {
            return Ok(());
        }
        let git_path = path.join(".git");
        if !git_path.exists() {
            return Ok(());
        }
        match git2::Repository::open(path) {
            Ok(repo) => {
                match RepoInfo::new(&repo, args.remote, args.fetch, path) {
                    Ok(repo) => {
                        repos.write().push(repo);
                    }
                    Err(e) => {
                        eprintln!("Failed to read repository at {}: {:?}", path.display(), e);
                    }
                }
                Ok(())
            }
            Err(e) => {
                anyhow::bail!("Could not open repository: {e}");
            }
        }
    })?;
    Ok(repos.read().to_vec())
}

/// Prints the repository status information as a table or list, depending on CLI options.
///
/// # Arguments
/// * `repos` - List of repositories to display.
/// * `args` - CLI arguments controlling the output format.
///
/// # Errors
/// Returns an error if output fails.
pub fn print_repositories(repos: &mut [RepoInfo], args: &Args) {
    if repos.is_empty() {
        println!("No repositories found.");
        return;
    }
    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic);

    let mut header = vec![
        Cell::new("Directory").add_attribute(Attribute::Bold),
        Cell::new("Branch").add_attribute(Attribute::Bold),
        Cell::new("Ahead").add_attribute(Attribute::Bold),
        Cell::new("Behind").add_attribute(Attribute::Bold),
        Cell::new("Commits").add_attribute(Attribute::Bold),
        Cell::new("Untracked").add_attribute(Attribute::Bold),
        Cell::new("Status").add_attribute(Attribute::Bold),
    ];
    if args.remote {
        header.push(Cell::new("Remote").add_attribute(Attribute::Bold));
    }
    table.set_header(header);
    repos.sort_by_key(|r| r.name.to_ascii_lowercase());
    for repo in repos {
        let status_str = if repo.status == "Dirty" {
            format!("Dirty ({} changed)", repo.changed)
        } else {
            repo.status.clone()
        };
        let status_cell = match repo.status.as_str() {
            "Clean" => Cell::new("Clean").fg(Color::Green),
            "Dirty" => Cell::new(&status_str).fg(Color::Red),
            _ => Cell::new(&repo.status),
        };
        let name_cell = if repo.has_unpushed {
            Cell::new(&repo.name).fg(Color::Red)
        } else {
            Cell::new(&repo.name)
        };
        let mut row = vec![
            name_cell,
            Cell::new(&repo.branch),
            Cell::new(repo.ahead),
            Cell::new(repo.behind),
            Cell::new(repo.commits),
            Cell::new(repo.untracked),
            status_cell,
        ];
        if args.remote {
            row.push(Cell::new(repo.remote_url.as_deref().unwrap_or("-")));
        }
        table.add_row(row);
    }
    println!("{table}");
}

/// Prints a summary of the repository scan (total, clean, dirty, unpushed).
///
/// # Arguments
/// * `repos` - List of repositories to summarize.
///
/// # Errors
/// Returns an error if output fails.
pub fn print_summary(repos: &[RepoInfo]) {
    let total = repos.len();
    let clean = repos.iter().filter(|r| r.status == "Clean").count();
    let dirty = repos.iter().filter(|r| r.status == "Dirty").count();
    let unpushed = repos.iter().filter(|r| r.has_unpushed).count();
    println!("\nSummary:");
    println!("  Total repositories:   {total}");
    println!("  Clean:                {clean}");
    println!("  With changes:         {dirty}");
    println!("  With unpushed:        {unpushed}");
}
