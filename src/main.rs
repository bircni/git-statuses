use std::{
    io::{self, Write},
    process::ExitCode,
};

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

/// Exit code used when at least one repository could not be processed.
const EXIT_FAILED_REPOS: u8 = 1;

/// Entry point for the git-statuses CLI tool.
/// Parses arguments, scans for repositories, prints their status and a summary.
fn main() -> ExitCode {
    if let Err(e) = util::initialize_logger() {
        eprintln!("{e:?}");
        return ExitCode::FAILURE;
    }

    let outcome = run(&Args::parse(), &mut io::stdout());

    // A scan that could not read some of the repositories it found is not a success: a
    // caller piping this into a script has no other way to notice.
    if outcome.failed > 0 {
        ExitCode::from(EXIT_FAILED_REPOS)
    } else {
        ExitCode::SUCCESS
    }
}

/// What a run ended up reporting.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    /// Number of repositories that were found but could not be processed.
    pub failed: usize,
}

/// Runs the tool for the given arguments.
///
/// Split out of `main` so that it can be driven from tests without spawning a process.
/// Repositories that cannot be read are collected into the failed list rather than
/// aborting the scan, so this cannot fail - it reports what happened instead.
///
/// # Arguments
/// * `args` - The parsed CLI arguments.
/// * `out` - Where all generated output is written to.
///
/// # Returns
/// What the run reported, which decides the process exit code.
fn run(args: &Args, out: &mut impl Write) -> Outcome {
    if let Some(shell) = args.completions {
        completions(shell, out);
        return Outcome::default();
    }

    if args.legend {
        printer::legend(args.condensed, out);
        return Outcome::default();
    }

    let (repos, failed_repos) = scan::find_repositories(args);
    let displayed = args.filter_repos(&repos);

    let outcome = Outcome {
        failed: failed_repos.len(),
    };

    if args.json {
        printer::json_output(&displayed, &failed_repos, out);
        return outcome;
    }

    printer::repositories_table(&displayed, args, out);
    printer::failed_summary(&failed_repos);
    if args.summary {
        // The summary describes the whole scan, not just the filtered selection.
        printer::summary(&repos, failed_repos.len(), out);
    }

    outcome
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
