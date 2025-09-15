use comfy_table::{Attribute, Cell, ContentArrangement, Table, presets};
use strum::IntoEnumIterator;

use crate::{
    cli::Args,
    gitinfo::{repoinfo::RepoInfo, status::Status},
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
    repos.sort_by_key(|r| r.name.to_ascii_lowercase());
    let repos_iter: Box<dyn Iterator<Item = &RepoInfo>> = if args.non_clean {
        Box::new(repos.iter().filter(|r| r.status != Status::Clean))
    } else {
        Box::new(repos.iter())
    };

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
        Cell::new("Repository").add_attribute(Attribute::Bold),
        Cell::new("Branch").add_attribute(Attribute::Bold),
        Cell::new("Local").add_attribute(Attribute::Bold),
        Cell::new("Commits").add_attribute(Attribute::Bold),
        Cell::new("Status").add_attribute(Attribute::Bold),
    ];
    if args.remote {
        header.push(Cell::new("Remote").add_attribute(Attribute::Bold));
    }
    if args.path {
        header.push(Cell::new("Path").add_attribute(Attribute::Bold));
    }
    table.set_header(header);

    for repo in repos_iter {
        let repo_path = repo
            .path
            .canonicalize()
            .unwrap_or_else(|_| repo.path.clone());
        let root_path = args.dir.canonicalize().unwrap_or_else(|_| args.dir.clone());
        let repo_path_relative = repo_path
            .strip_prefix(&root_path)
            .unwrap_or(&repo_path)
            .display()
            .to_string();
        let display_str = if repo_path_relative == repo.name {
            repo.name.clone()
        } else {
            format!("{} ({})", repo.name, repo_path_relative)
        };
        let name_cell = Cell::new(display_str).fg(repo.status.comfy_color());

        let mut row = vec![
            name_cell,
            Cell::new(&repo.branch),
            Cell::new(repo.format_local_status()),
            Cell::new(repo.commits),
            Cell::new(repo.format_status_with_stash()).fg(repo.status.comfy_color()),
        ];
        if args.remote {
            row.push(Cell::new(repo.remote_url.as_deref().unwrap_or("-")));
        }
        if args.path {
            row.push(Cell::new(repo.path.display()));
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
    println!("The counts in brackets indicate the number of changed files.");
    println!("The counts in brackets with an asterisk (*) indicate the number of stashes.");
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
    let with_stashes = repos.iter().filter(|r| r.stash_count > 0).count();
    let local_only = repos.iter().filter(|r| r.is_local_only).count();
    println!("\nSummary:");
    println!("  Total repositories:   {total}");
    println!("  Clean:                {clean}");
    println!("  With changes:         {dirty}");
    println!("  With unpushed:        {unpushed}");
    println!("  With stashes:         {with_stashes}");
    println!("  Local-only branches:  {local_only}");
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

/// Prints the repository information in JSON format.
/// # Arguments
/// * `repos` - List of repositories to output.
/// * `failed_repos` - List of repository names that failed to process.
pub fn json_output(repos: &[RepoInfo], failed_repos: &[String]) {
    let output = serde_json::json!({
        "repositories": repos,
        "failed": failed_repos
    });
    println!("{output}");
}
