use std::io;

use anyhow::Result;
use clap::{CommandFactory as _, Parser as _};

use crate::{cli::Args, interactive::mode::InteractiveMode};

mod cli;
mod gitinfo;
mod interactive;
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

    let (mut repos, failed_repos) = args.find_repositories();

    if args.json {
        printer::json_output(&repos, &failed_repos);
        return Ok(());
    }

    // Enter interactive mode if requested
    if args.interactive {
        let mut interactive_mode = InteractiveMode::new(&repos, args)?;
        interactive_mode.run()?;
        return Ok(());
    }

    printer::repositories_table(&mut repos, &args);
    printer::failed_summary(&failed_repos);
    if args.summary {
        printer::summary(&repos, failed_repos.len());
    }

    Ok(())
}
