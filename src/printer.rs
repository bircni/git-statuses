use std::io::Write;

use comfy_table::{Attribute, Cell, ContentArrangement, Table, presets};
use strum::IntoEnumIterator;

use crate::{
    cli::Args,
    gitinfo::{repoinfo::RepoInfo, status::Status},
};

/// Builds an empty table with the shared style applied.
///
/// Both the repository table and the legend use the same preset and arrangement, so the
/// two can never drift apart in how they render.
fn styled_table(condensed: bool) -> Table {
    let mut table = Table::new();
    table
        .load_style(if condensed {
            presets::UTF8_FULL_CONDENSED
        } else {
            presets::UTF8_FULL
        })
        .set_content_arrangement(ContentArrangement::Dynamic);
    table
}

/// Builds a bold header cell.
fn header_cell(title: &str) -> Cell {
    Cell::new(title).add_attribute(Attribute::Bold)
}

/// Prints the repository status information as a table or list, depending on CLI options.
///
/// Expects the repositories to already be sorted and filtered (see
/// `Args::find_repositories` and `Args::filter_repos`).
///
/// # Arguments
/// * `repos` - List of repositories to display.
/// * `args` - CLI arguments controlling the output format.
/// * `out` - Where to write the table to.
pub fn repositories_table(repos: &[RepoInfo], args: &Args, out: &mut impl Write) {
    if repos.is_empty() {
        log::info!("No repositories found.");
        return;
    }

    let mut table = styled_table(args.condensed);

    let mut header: Vec<Cell> = ["Directory", "Branch", "Local", "Commits", "Status"]
        .into_iter()
        .map(header_cell)
        .collect();
    if args.remote {
        header.push(header_cell("Remote"));
    }
    if args.path {
        header.push(header_cell("Path"));
    }
    table.set_header(header);

    for repo in repos {
        // `⎇` marks a linked worktree; the plain path is passed through untouched.
        let name_cell = if repo.is_worktree {
            Cell::new(format!("⎇ {}", repo.repo_path))
        } else {
            Cell::new(&repo.repo_path)
        }
        .fg(repo.status.comfy_color());

        let mut row = vec![
            name_cell,
            Cell::new(&repo.branch),
            Cell::new(repo.format_local_status()),
            Cell::new(repo.commits),
            Cell::new(repo.format_status_with_stash_and_ff()).fg(repo.status.comfy_color()),
        ];
        if args.remote {
            row.push(Cell::new(repo.remote_url.as_deref().unwrap_or("-")));
        }
        if args.path {
            row.push(Cell::new(repo.path.display()));
        }
        table.add_row(row);
    }
    // Writing to the caller's sink cannot be recovered from here, and a broken pipe is
    // the normal way this ends when the output is piped into `head`.
    let _ = writeln!(out, "{table}");
}

/// Prints a legend explaining the color codes and statuses used in the output.
/// # Arguments
/// * `condensed` - If true, uses a condensed format for the legend.
/// * `out` - Where to write the legend to.
pub fn legend(condensed: bool, out: &mut impl Write) {
    let mut table = styled_table(condensed);
    table.set_header(vec![header_cell("Status"), header_cell("Description")]);
    Status::iter().for_each(|status| {
        table.add_row(vec![status.as_cell(), Cell::new(status.description())]);
    });
    let _ = writeln!(out, "{table}");
    let _ = writeln!(
        out,
        "The counts in brackets indicate the number of changed files."
    );
    let _ = writeln!(
        out,
        "The counts in brackets with an asterisk (*) indicate the number of stashes."
    );
    let _ = writeln!(out, "↑↑ indicates that the repository was fast-forwarded");
    let _ = writeln!(out, "⎇ indicates a Git worktree");
}

/// Prints a summary of the repository scan (total, clean, dirty, unpushed).
///
/// # Arguments
/// * `repos` - List of repositories to summarize.
/// * `failed` - Number of repositories that failed to process.
/// * `out` - Where to write the summary to.
pub fn summary(repos: &[RepoInfo], failed: usize, out: &mut impl Write) {
    let total = repos.len();
    let clean = repos.iter().filter(|r| r.status == Status::Clean).count();
    let dirty = repos
        .iter()
        .filter(|r| matches!(r.status, Status::Dirty(_)))
        .count();
    let unpushed = repos.iter().filter(|r| r.has_unpushed).count();
    let with_stashes = repos.iter().filter(|r| r.stash_count > 0).count();
    let local_only = repos.iter().filter(|r| r.is_local_only).count();
    let fast_forwarded = repos.iter().filter(|r| r.fast_forwarded).count();
    let _ = writeln!(out, "\nSummary:");
    let _ = writeln!(out, "  Total repositories:   {total}");
    let _ = writeln!(out, "  Clean:                {clean}");
    let _ = writeln!(out, "  With changes:         {dirty}");
    let _ = writeln!(out, "  With unpushed:        {unpushed}");
    let _ = writeln!(out, "  With stashes:         {with_stashes}");
    let _ = writeln!(out, "  Local-only branches:  {local_only}");
    let _ = writeln!(out, "  Fast-forwarded:       {fast_forwarded}");
    if failed > 0 {
        let _ = writeln!(out, "  Failed to process:    {failed}");
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

/// Builds the JSON representation of a scan result.
/// # Arguments
/// * `repos` - List of repositories to output.
/// * `failed_repos` - List of repository names that failed to process.
/// # Returns
/// The JSON value that `json_output` prints.
pub fn json_value(repos: &[RepoInfo], failed_repos: &[String]) -> serde_json::Value {
    serde_json::json!({
        "repositories": repos,
        "failed": failed_repos
    })
}

/// Prints the repository information in JSON format.
/// # Arguments
/// * `repos` - List of repositories to output.
/// * `failed_repos` - List of repository names that failed to process.
/// * `out` - Where to write the JSON to.
pub fn json_output(repos: &[RepoInfo], failed_repos: &[String], out: &mut impl Write) {
    let _ = writeln!(out, "{}", json_value(repos, failed_repos));
}
