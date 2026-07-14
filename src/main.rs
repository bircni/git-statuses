use std::io;

use anyhow::Result;
use clap::{CommandFactory as _, Parser as _};

use crate::cli::Args;

mod cli;
mod gitinfo;
mod printer;
#[cfg(test)]
mod tests;
mod util;

/// Entry point for the git-statuses CLI tool.
/// Parses arguments, scans for repositories, prints their status and a summary.
fn main() -> Result<()> {
    util::initialize_logger()?;

    let args = Args::parse();

    if let Some(shell) = args.completions {
        let mut cmd = Args::command();
        clap_complete::generate(shell, &mut cmd, env!("CARGO_PKG_NAME"), &mut io::stdout());
        return Ok(());
    }

    if args.legend {
        printer::legend(args.condensed);
        return Ok(());
    }

    let (repos, failed_repos) = args.find_repositories();
    let displayed = args.filter_repos(&repos);

    if args.json {
        printer::json_output(&displayed, &failed_repos);
        return Ok(());
    }

    printer::repositories_table(&displayed, &args);
    printer::failed_summary(&failed_repos);
    if args.summary {
        // The summary describes the whole scan, not just the filtered selection.
        printer::summary(&repos, failed_repos.len());
    }

    Ok(())
}
