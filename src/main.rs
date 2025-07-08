use anyhow::Result;
use clap::{Parser, crate_name, crate_version};
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
    } else if args.version {
        println!("{} {}", crate_name!(), crate_version!());
        #[cfg(feature = "update-check")]
        update_available::print_check_force(
            crate_name!(),
            crate_version!(),
            update_available::Source::CratesIo,
        );
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
