use std::path::PathBuf;

use clap::Parser;
use clap_complete::Shell;

/// Scan the given directory for Git repositories and display their status.
/// A Repository turns red if it has unpushed changes.
#[expect(
    clippy::struct_excessive_bools,
    reason = "This is a CLI tool with many options, and excessive bools are common in such cases."
)]
#[derive(Parser, Debug, Default)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Directory to scan
    #[arg(default_value = ".")]
    pub dir: PathBuf,
    /// Recursively scan all subdirectories to the given depth.
    /// If set to 1, only the current directory is scanned.
    /// If set to -1, all subdirectories are scanned. (this may take a while)
    #[arg(short, long, default_value = "1")]
    pub depth: i32,
    /// Show remote URL
    #[arg(short = 'r', long)]
    pub remote: bool,
    /// Use a condensed layout
    #[arg(short, long)]
    pub condensed: bool,
    /// Show a summary of the scan
    #[arg(short = 's', long)]
    pub summary: bool,
    /// Run a fetch before scanning to update the repository state
    /// Note: This may take a while for large repositories.
    #[arg(short, long)]
    pub fetch: bool,
    /// Print a legend explaining the color codes and statuses used in the output
    #[arg(short, long)]
    pub legend: bool,
    /// Look in a specific subdir if it exists for each folder
    /// This can be useful, if you don't checkout in a folder directly
    /// but in a subfolder like `repo-name/checkout`
    #[arg(long)]
    pub subdir: Option<String>,
    /// Generate shell completions
    #[arg(long, value_name = "SHELL")]
    pub completions: Option<Shell>,
    /// Show the path to the repository
    #[arg(short, long)]
    pub path: bool,
    /// Only show non clean repositories
    #[arg(short = 'n', long)]
    pub non_clean: bool,
    /// Enable interactive mode to select and interact with repositories
    #[arg(short, long)]
    pub interactive: bool,
}
