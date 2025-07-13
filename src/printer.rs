use std::fs;
use std::path::Path;

use anyhow::Result;
use comfy_table::{Attribute, Cell, ContentArrangement, Table, presets};
use serde_json;
use strum::IntoEnumIterator;

use crate::{
    cli::Args,
    gitinfo::{repoinfo::RepoInfo, status::Status},
    output::OutputFormat,
};

/// Prints the repository status information in the specified format.
///
/// # Arguments
/// * `repos` - List of repositories to display.
/// * `args` - CLI arguments controlling the output format.
/// 
/// # Errors
/// Returns an error if file writing fails or if an unsupported format is specified.
pub fn repositories_table(repos: &mut [RepoInfo], args: &Args) -> Result<()> {
    if repos.is_empty() {
        log::info!("No repositories found.");
        return Ok(());
    }

    // Parse output format
    let output_format = args.output.parse::<OutputFormat>()?;
    
    // Validate file output options
    if let Some(ref file_path) = args.output_file {
        if !output_format.supports_file_output() {
            anyhow::bail!("File output is only supported for json and html formats, not {}", output_format);
        }
    }

    repos.sort_by_key(|r| r.name.to_ascii_lowercase());
    let repos_iter: Box<dyn Iterator<Item = &RepoInfo>> = if args.non_clean {
        Box::new(repos.iter().filter(|r| r.status != Status::Clean))
    } else {
        Box::new(repos.iter())
    };

    let repos_vec: Vec<&RepoInfo> = repos_iter.collect();

    match output_format {
        OutputFormat::Table => {
            let output = format_table(&repos_vec, args);
            if let Some(ref file_path) = args.output_file {
                anyhow::bail!("Table format cannot be written to file. Use json or html format instead.");
            }
            print!("{output}");
        }
        OutputFormat::Json => {
            let output = format_json(&repos_vec)?;
            if let Some(ref file_path) = args.output_file {
                write_to_file(file_path, &output)?;
            } else {
                print!("{output}");
            }
        }
        OutputFormat::Html => {
            let output = format_html(&repos_vec, args);
            if let Some(ref file_path) = args.output_file {
                write_to_file(file_path, &output)?;
            } else {
                print!("{output}");
            }
        }
    }

    Ok(())
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

/// Formats repository data as a table string.
fn format_table(repos: &[&RepoInfo], args: &Args) -> String {
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
    if args.path {
        header.push(Cell::new("Path").add_attribute(Attribute::Bold));
    }
    table.set_header(header);

    for repo in repos {
        let name_cell = Cell::new(&repo.name).fg(repo.status.color());

        let mut row = vec![
            name_cell,
            Cell::new(&repo.branch),
            Cell::new(repo.format_local_status()),
            Cell::new(repo.commits),
            Cell::new(repo.format_status_with_stash()).fg(repo.status.color()),
        ];
        if args.remote {
            row.push(Cell::new(repo.remote_url.as_deref().unwrap_or("-")));
        }
        if args.path {
            row.push(Cell::new(repo.path.display()));
        }
        table.add_row(row);
    }
    format!("{table}\n")
}

/// Formats repository data as JSON.
fn format_json(repos: &[&RepoInfo]) -> Result<String> {
    let json = serde_json::to_string_pretty(repos)?;
    Ok(format!("{json}\n"))
}

/// HTML template for the repository status report.
const HTML_TEMPLATE_START: &str = r#"<!DOCTYPE html>
<html>
<head>
    <title>Git Repository Status</title>
    <style>
        table { border-collapse: collapse; width: 100%; }
        th, td { border: 1px solid #ddd; padding: 8px; text-align: left; }
        th { background-color: #f2f2f2; font-weight: bold; }
        .clean { color: green; }
        .dirty { color: orange; }
        .unpushed { color: red; }
        .unpublished { color: blue; }
    </style>
</head>
<body>
    <h1>Git Repository Status</h1>
    <table>
"#;

const HTML_TEMPLATE_END: &str = r#"    </table>
</body>
</html>
"#;

/// Formats repository data as HTML table.
fn format_html(repos: &[&RepoInfo], args: &Args) -> String {
    let mut html = String::from(HTML_TEMPLATE_START);
    
    // Header
    html.push_str("        <tr>");
    html.push_str("<th>Directory</th><th>Branch</th><th>Local</th><th>Commits</th><th>Status</th>");
    if args.remote {
        html.push_str("<th>Remote</th>");
    }
    if args.path {
        html.push_str("<th>Path</th>");
    }
    html.push_str("</tr>\n");

    // Rows
    for repo in repos {
        let status_class = get_status_css_class(&repo.status);
        
        html.push_str("        <tr>");
        html.push_str(&format!("<td class=\"{status_class}\">{}</td>", html_escape(&repo.name)));
        html.push_str(&format!("<td>{}</td>", html_escape(&repo.branch)));
        html.push_str(&format!("<td>{}</td>", html_escape(&repo.format_local_status())));
        html.push_str(&format!("<td>{}</td>", repo.commits));
        html.push_str(&format!("<td class=\"{status_class}\">{}</td>", html_escape(&repo.format_status_with_stash())));
        if args.remote {
            html.push_str(&format!("<td>{}</td>", html_escape(repo.remote_url.as_deref().unwrap_or("-"))));
        }
        if args.path {
            html.push_str(&format!("<td>{}</td>", html_escape(&repo.path.display().to_string())));
        }
        html.push_str("</tr>\n");
    }

    html.push_str(HTML_TEMPLATE_END);
    html
}

/// Maps repository status to CSS class name.
fn get_status_css_class(status: &Status) -> &'static str {
    match status {
        Status::Clean => "clean",
        Status::Dirty(_) => "dirty",
        Status::Unpushed | Status::Unpublished => "unpushed",
        _ => "",
    }
}

/// Simple HTML escaping for table content.
fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// Writes content to a file.
fn write_to_file(file_path: &Path, content: &str) -> Result<()> {
    fs::write(file_path, content)?;
    log::info!("Output written to {}", file_path.display());
    Ok(())
}
