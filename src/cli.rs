use std::path::PathBuf;

use clap::{ArgAction, Parser};

/// Scan the given directory for Git repositories and display their status.
/// A Repository turns red if it has unpushed changes.
#[expect(
    clippy::struct_excessive_bools,
    reason = "This is a CLI tool with many options, and excessive bools are common in such cases."
)]
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Directory to scan
    #[arg(default_value = ".")]
    pub dir: PathBuf,
    /// Recursively scan all subdirectories to the given depth.
    /// If set to 1, only the current directory is scanned.
    #[arg(short, long, default_value = "1")]
    pub depth: usize,
    /// Show remote URL
    #[arg(short = 'r', long, action = ArgAction::SetTrue)]
    pub remote: bool,
    /// Show a summary of the scan
    #[arg(short, long, action = ArgAction::SetTrue)]
    pub summary: bool,
    /// Run a fetch before scanning to update the repository state
    /// Note: This may take a while for large repositories.
    #[arg(short, long, action = ArgAction::SetTrue)]
    pub fetch: bool,
    /// Print a legend explaining the color codes and statuses used in the output
    #[arg(short, long, action = ArgAction::SetTrue)]
    pub legend: bool,
}
