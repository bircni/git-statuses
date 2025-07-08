use comfy_table::{Attribute, Cell, ContentArrangement, Table, presets};
use strum::IntoEnumIterator;

use crate::{
    cli::Args,
    gitinfo::{RepoInfo, status::Status},
};

/// Prints the repository status information as a table or list, depending on CLI options.
///
/// # Arguments
/// * `repos` - List of repositories to display.
/// * `args` - CLI arguments controlling the output format.
pub fn repositories_table(repos: &mut [RepoInfo], args: &Args) {
    if repos.is_empty() {
        log::info!("No repositories found.");
        return;
    }
    let mut table = Table::new();
    let preset = if args.condensed {
        presets::UTF8_FULL_CONDENSED
    } else {
        presets::UTF8_FULL
    };
    table
        .load_preset(preset)
        .set_content_arrangement(ContentArrangement::Dynamic);

    let mut header = vec![
        Cell::new("Directory").add_attribute(Attribute::Bold),
        Cell::new("Branch").add_attribute(Attribute::Bold),
        Cell::new("Local").add_attribute(Attribute::Bold),
        Cell::new("Commits").add_attribute(Attribute::Bold),
        Cell::new("Status").add_attribute(Attribute::Bold),
    ];
    if args.remote {
        header.push(Cell::new("Remote").add_attribute(Attribute::Bold));
    }
    table.set_header(header);
    repos.sort_by_key(|r| r.name.to_ascii_lowercase());
    for repo in repos {
        let status_cell = repo.status.as_cell();
        let name_cell = Cell::new(&repo.name).fg(repo.status.color());

        let mut row = vec![
            name_cell,
            Cell::new(&repo.branch),
            Cell::new(format!("↑{} ↓{}", repo.ahead, repo.behind)),
            Cell::new(repo.commits),
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
/// # Arguments
/// * `condensed` - If true, uses a condensed format for the legend.
pub fn legend(condensed: bool) {
    let mut table = Table::new();
    let preset = if condensed {
        presets::UTF8_FULL_CONDENSED
    } else {
        presets::UTF8_FULL
    };
    table
        .load_preset(preset)
        .set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec![
        Cell::new("Status").add_attribute(Attribute::Bold),
        Cell::new("Description").add_attribute(Attribute::Bold),
    ]);
    Status::iter().for_each(|status| {
        table.add_row(vec![status.as_cell(), Cell::new(status.description())]);
    });
    println!("{table}");
}

/// Prints a summary of the repository scan (total, clean, dirty, unpushed).
///
/// # Arguments
/// * `repos` - List of repositories to summarize.
/// * `failed` - Number of repositories that failed to process.
pub fn summary(repos: &[RepoInfo], failed: usize) {
    let total = repos.len();
    let clean = repos.iter().filter(|r| r.status == Status::Clean).count();
    let dirty = repos
        .iter()
        .filter(|r| matches!(r.status, Status::Dirty(_)))
        .count();
    let unpushed = repos.iter().filter(|r| r.has_unpushed).count();
    println!("\nSummary:");
    println!("  Total repositories:   {total}");
    println!("  Clean:                {clean}");
    println!("  With changes:         {dirty}");
    println!("  With unpushed:        {unpushed}");
    if failed > 0 {
        println!("  Failed to process:    {failed}");
    }
}

/// Prints a summary of failed repositories that could not be processed.
/// # Arguments
/// * `failed_repos` - List of repository names that failed to process.
pub fn failed_summary(failed_repos: &[String]) {
    if !failed_repos.is_empty() {
        log::warn!("Failed to process the following repositories:");
        for repo in failed_repos {
            log::warn!(" - {repo}");
        }
    }
}
