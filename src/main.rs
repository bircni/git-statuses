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
        printer::print_legend();
        return Ok(());
    }

    let (mut repos, failed_repos) = util::find_repositories(&args)?;

    printer::repositories_table(&mut repos, &args);
    printer::failed_summary(&failed_repos);
    if args.summary {
        printer::summary(&repos, failed_repos.len());
    }

    Ok(())
}
