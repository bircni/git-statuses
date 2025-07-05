use comfy_table::{Attribute, Cell, Color, ContentArrangement, Table, presets};

use crate::{cli::Args, gitinfo::RepoInfo};

/// Prints the repository status information as a table or list, depending on CLI options.
///
/// # Arguments
/// * `repos` - List of repositories to display.
/// * `args` - CLI arguments controlling the output format.
///
/// # Errors
/// Returns an error if output fails.
pub fn repositories_table(repos: &mut [RepoInfo], args: &Args) {
    if repos.is_empty() {
        log::info!("No repositories found.");
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
        let name_cell = Cell::new(&repo.name).fg(if repo.has_unpushed {
            Color::Red
        } else if repo.commits == 0 {
            Color::Blue
        } else if repo.ahead > 0 {
            Color::Yellow
        } else if repo.behind > 0 {
            Color::Cyan
        } else {
            Color::Reset
        });

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

/// Prints a legend explaining the color codes and statuses used in the output.
pub fn print_legend() {
    println!("\nLegend:");
    println!("  Clean: No changes, no unpushed commits.");
    println!("  Dirty: Changes present, may or may not have unpushed commits.");
    println!("  Unpushed: Commits that are not pushed to the remote repository.");
    println!("  Red: Repository has unpushed commits.");
    println!("  Blue: Repository has no commits in the current branch.");
    println!("  Yellow: Repository is ahead of upstream.");
    println!("  Cyan: Repository is behind upstream.");
}

/// Prints a summary of the repository scan (total, clean, dirty, unpushed).
///
/// # Arguments
/// * `repos` - List of repositories to summarize.
///
/// # Errors
/// Returns an error if output fails.
pub fn summary(repos: &[RepoInfo]) {
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

/// Prints a summary of failed repositories that could not be processed.
/// # Arguments
/// * `failed_repos` - List of repository names that failed to process.
///
/// # Errors
/// Returns an error if output fails.
pub fn failed_summary(failed_repos: &[String]) {
    if !failed_repos.is_empty() {
        log::warn!("Failed to process the following repositories:");
        for repo in failed_repos {
            log::warn!(" - {repo}");
        }
    }
}
