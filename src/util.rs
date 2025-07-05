use std::sync::Arc;

use anyhow::Context as _;
use log::LevelFilter;
use parking_lot::RwLock;
use rayon::iter::{IntoParallelRefIterator as _, ParallelIterator as _};
use simplelog::{ColorChoice, ConfigBuilder, TermLogger, TerminalMode};
use walkdir::WalkDir;

use crate::{cli::Args, gitinfo::RepoInfo};

/// Scans the given directory (recursively if requested) for Git repositories and collects their status information.
///
/// # Arguments
/// * `args` - CLI arguments controlling the scan behavior.
///
/// # Returns
/// A tuple containing:
/// - A vector of `RepoInfo` containing details about each found repository.
/// - A vector of strings of failed repositories (those that could not be opened or processed).
pub fn find_repositories(args: &Args) -> anyhow::Result<(Vec<RepoInfo>, Vec<String>)> {
    let min_depth = 1;
    let max_depth = if args.depth > 0 { args.depth } else { 1 };
    let walker = WalkDir::new(&args.dir)
        .min_depth(min_depth)
        .max_depth(max_depth)
        .follow_links(false)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .collect::<Vec<_>>();

    let repos: Arc<RwLock<Vec<RepoInfo>>> = Arc::new(RwLock::new(Vec::new()));
    let failed_repos: Arc<RwLock<Vec<String>>> = Arc::new(RwLock::new(Vec::new()));

    walker.par_iter().try_for_each(|entry| {
        let path = entry.path();
        let repo_name = get_repo_name(path);
        if !path.is_dir() {
            return Ok(());
        }
        let git_path = path.join(".git");
        if !git_path.exists() {
            return Ok(());
        }
        match git2::Repository::open(path) {
            Ok(repo) => {
                if let Ok(repo) = RepoInfo::new(&repo, args.remote, args.fetch, path) {
                    repos.write().push(repo);
                } else {
                    // println!("Failed to process repository: {}", path.display());
                    failed_repos.write().push(repo_name);
                }
                Ok(())
            }
            Err(e) => {
                anyhow::bail!("Could not open repository: {e}");
            }
        }
    })?;
    Ok((repos.read().to_vec(), failed_repos.read().to_vec()))
}

/// Extracts the repository name from the given path.
fn get_repo_name(path: &std::path::Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_owned()
}

/// Initializes the logger for the application.
///
/// Returns an error if logger initialization fails.
pub fn initialize_logger() -> anyhow::Result<()> {
    TermLogger::init(
        #[cfg(debug_assertions)]
        LevelFilter::max(),
        #[cfg(not(debug_assertions))]
        LevelFilter::Info,
        ConfigBuilder::new()
            .add_filter_allow_str("git_statuses")
            .build(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .context("Failed to initialize logger")
}
