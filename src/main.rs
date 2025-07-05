use anyhow::Result;
use clap::Parser as _;
use cli::Args;

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
