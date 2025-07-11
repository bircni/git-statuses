use std::{path::Path, sync::Arc};

use anyhow::Context as _;
use log::LevelFilter;
use parking_lot::RwLock;
use rayon::iter::{IntoParallelRefIterator as _, ParallelIterator as _};
use simplelog::{ColorChoice, ConfigBuilder, TermLogger, TerminalMode};
use walkdir::WalkDir;

use crate::{cli::Args, gitinfo::repoinfo::RepoInfo};

/// Scans the given directory (recursively if requested) for Git repositories and collects their status information.
///
/// # Arguments
/// * `args` - CLI arguments controlling the scan behavior.
///
/// # Returns
/// A tuple containing:
/// - A vector of `RepoInfo` containing details about each found repository.
/// - A vector of strings of failed repositories (those that could not be opened or processed).
#[expect(
    clippy::cast_sign_loss,
    reason = "We check i32 to be non-negative, so casting to usize is safe"
)]
pub fn find_repositories(args: &Args) -> (Vec<RepoInfo>, Vec<String>) {
    let min_depth = 0;
    let walker = {
        let mut walk = WalkDir::new(&args.dir)
            .min_depth(min_depth)
            .follow_links(false);

        if args.depth != -1 && args.depth >= 0 {
            let max_depth = if args.depth > 0 { args.depth } else { 1 };
            walk = walk.max_depth(max_depth as usize);
        }

        walk.into_iter().filter_map(Result::ok).collect::<Vec<_>>()
    };

    let repos: Arc<RwLock<Vec<RepoInfo>>> = Arc::new(RwLock::new(Vec::new()));
    let failed_repos: Arc<RwLock<Vec<String>>> = Arc::new(RwLock::new(Vec::new()));

    walker.par_iter().for_each(|entry| {
        let orig_path = entry.path();
        let repo_name = orig_path.dir_name();
        let path_buf = {
            if orig_path.is_git_directory() {
                orig_path.to_path_buf()
            } else if let Some(subdir) = &args.subdir {
                let subdir_path = orig_path.join(subdir);
                if subdir_path.is_git_directory() {
                    subdir_path
                } else {
                    // If the subdir does not exist, skip this directory
                    return;
                }
            } else {
                // If no subdir is specified and the path is not a git directory, skip it
                return;
            }
        };
        match git2::Repository::open(path_buf.as_path()) {
            Ok(mut git_repo) => {
                if let Ok(repo) = RepoInfo::new(&mut git_repo, &repo_name, args.remote, args.fetch)
                {
                    repos.write().push(repo);
                } else {
                    failed_repos.write().push(repo_name);
                }
            }
            Err(e) => {
                log::debug!("Failed to open repository at {}: {}", path_buf.display(), e);
                failed_repos.write().push(path_buf.dir_name());
            }
        }
    });
    (repos.read().to_vec(), failed_repos.read().to_vec())
}

/// Initializes the logger for the application.
///
/// # Errors
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

/// Extension trait for working with Git repository paths.
pub trait GitPathExt {
    /// Checks if the path is a Git repository directory.
    ///
    /// This checks if the directory exists and contains a `.git` subdirectory.
    ///
    /// # Returns
    ///
    /// `true` if the path is a Git repository, `false` otherwise.
    fn is_git_directory(&self) -> bool;

    /// Extracts the repository name from the path.
    ///
    /// # Returns
    ///
    /// The final component of the path (i.e., the directory name) as a `String`,
    /// which typically corresponds to the repository name. Returns `"unknown"` if
    /// the path has no final component or cannot be converted to a valid UTF-8 string.
    fn dir_name(&self) -> String;
}

impl GitPathExt for Path {
    fn is_git_directory(&self) -> bool {
        self.is_dir() && self.join(".git").exists()
    }

    fn dir_name(&self) -> String {
        self.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_owned()
    }
}
