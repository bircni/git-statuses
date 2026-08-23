use std::io::{self, Write};

use anyhow::Result;
use clap::{CommandFactory as _, Parser as _};
use clap_complete::Shell;

use crate::cli::Args;

mod cli;
mod gitinfo;
mod printer;
mod scan;
#[cfg(test)]
mod tests;
mod util;

/// Entry point for the git-statuses CLI tool.
/// Parses arguments, scans for repositories, prints their status and a summary.
fn main() -> Result<()> {
    util::initialize_logger()?;

    run(&Args::parse(), &mut io::stdout());

    Ok(())
}

/// Runs the tool for the given arguments.
///
/// Split out of `main` so that it can be driven from tests without spawning a process.
/// Repositories that cannot be read are collected into the failed list rather than
/// aborting the scan, so this cannot fail.
///
/// # Arguments
/// * `args` - The parsed CLI arguments.
/// * `out` - Where all generated output is written to.
fn run(args: &Args, out: &mut impl Write) {
    if let Some(shell) = args.completions {
        completions(shell, out);
        return;
    }

    if args.legend {
        printer::legend(args.condensed, out);
        return;
    }

    let (repos, failed_repos) = scan::find_repositories(args);
    let displayed = args.filter_repos(&repos);

    if args.json {
        printer::json_output(&displayed, &failed_repos, out);
        return;
    }

    printer::repositories_table(&displayed, args, out);
    printer::failed_summary(&failed_repos);
    if args.summary {
        // The summary describes the whole scan, not just the filtered selection.
        printer::summary(&repos, failed_repos.len(), out);
    }
}

/// Writes the shell completion script for `shell`.
///
/// # Arguments
/// * `shell` - The shell to generate completions for.
/// * `out` - Where to write the completion script to.
fn completions(shell: Shell, out: &mut impl Write) {
    clap_complete::generate(
        shell,
        &mut Args::command(),
        env!("CARGO_PKG_NAME"),
        &mut *out,
    );
}
