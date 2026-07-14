use std::{borrow::Cow, ffi::OsStr, path::PathBuf, sync::Arc};

use clap::Parser;
use clap_complete::Shell;
use parking_lot::RwLock;
use rayon::iter::{IntoParallelRefIterator as _, ParallelIterator as _};
use walkdir::WalkDir;

use crate::{
    gitinfo::{repoinfo::RepoInfo, status::Status},
    util::GitPathExt as _,
};

/// Scan the given directory for Git repositories and display their status.
/// A Repository turns red if it has unpushed changes.
#[expect(
    clippy::struct_excessive_bools,
    reason = "This is a CLI tool with many options, and excessive bools are common in such cases."
)]
#[derive(Parser, Debug, Default)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Directory to scan
    #[arg(default_value = ".")]
    pub dir: PathBuf,
    /// Recursively scan all subdirectories to the given depth.
    /// If set to 1, only the current directory is scanned.
    /// If set to a negative value, all subdirectories are scanned. (this may take a while)
    #[arg(short, long, default_value = "1", allow_negative_numbers = true)]
    pub depth: i32,
    /// Show remote URL
    #[arg(short = 'r', long)]
    pub remote: bool,
    /// Use a condensed layout
    #[arg(short, long)]
    pub condensed: bool,
    /// Show a summary of the scan
    #[arg(short = 's', long)]
    pub summary: bool,
    /// Run a fetch before scanning to update the repository state
    /// Note: This may take a while for large repositories.
    #[arg(short, long)]
    pub fetch: bool,
    /// Run a fast-forward merge after fetching
    #[arg(short = 'F', long = "ff")]
    pub fast_forward: bool,
    /// Print a legend explaining the color codes and statuses used in the output
    #[arg(short, long)]
    pub legend: bool,
    /// Look in a specific subdir if it exists for each folder
    /// This can be useful, if you don't checkout in a folder directly
    /// but in a subfolder like `repo-name/checkout`
    #[arg(long)]
    pub subdir: Option<String>,
    /// Generate shell completions
    #[arg(long, value_name = "SHELL")]
    pub completions: Option<Shell>,
    /// Show the path to the repository
    #[arg(short, long)]
    pub path: bool,
    /// Only show non clean repositories
    #[arg(short = 'n', long)]
    pub non_clean: bool,
    /// Output in JSON format
    #[arg(long)]
    pub json: bool,
}

impl Args {
    /// Scans the given directory (recursively if requested) for Git repositories and collects their status information.
    ///
    /// The repositories are collected in parallel, so both returned vectors are sorted
    /// before they are handed back. Every consumer (table, JSON, warnings) therefore sees
    /// the same, reproducible order.
    ///
    /// # Returns
    /// A tuple containing:
    /// - A vector of `RepoInfo` containing details about each found repository.
    /// - A vector of strings of failed repositories (those that could not be opened or processed).
    #[expect(
        clippy::cast_sign_loss,
        reason = "We check i32 to be non-negative, so casting to usize is safe"
    )]
    pub fn find_repositories(&self) -> (Vec<RepoInfo>, Vec<String>) {
        let walker = {
            let mut walk = WalkDir::new(&self.dir).min_depth(0).follow_links(false);

            // Any negative depth means "no limit"; `-1` is just the documented spelling.
            // A depth of 0 would find nothing at all, so it is treated like 1.
            if self.depth >= 0 {
                walk = walk.max_depth(self.depth.max(1) as usize);
            }

            // Never descend into a repository's own git directory. Nothing inside it is a
            // repository the user asked about - it holds git's bookkeeping, including the
            // `worktrees/<name>` metadata directories - and on a deep scan it is a lot of
            // entries to walk and stat for nothing.
            walk.into_iter()
                .filter_entry(|e| e.depth() == 0 || e.file_name() != OsStr::new(".git"))
                .filter_map(Result::ok)
                .collect::<Vec<_>>()
        };

        let repos: Arc<RwLock<Vec<RepoInfo>>> = Arc::new(RwLock::new(Vec::new()));
        let failed_repos: Arc<RwLock<Vec<String>>> = Arc::new(RwLock::new(Vec::new()));

        walker.par_iter().for_each(|entry| {
            let orig_path = entry.path();
            let repo_name = orig_path.dir_name();
            let path_buf = {
                if orig_path.is_git_directory() || orig_path.is_git_worktree() {
                    orig_path.to_path_buf()
                } else if let Some(subdir) = &self.subdir {
                    let subdir_path = orig_path.join(subdir);
                    if subdir_path.is_git_directory() || subdir_path.is_git_worktree() {
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
                    if let Ok(repo) = RepoInfo::new(
                        &mut git_repo,
                        &repo_name,
                        self.remote,
                        self.fetch,
                        self.fast_forward,
                        &self.dir,
                    ) {
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

        let mut repos = repos.read().to_vec();
        let mut failed_repos = failed_repos.read().to_vec();
        repos.sort_by_key(|r| r.repo_path.to_lowercase());
        failed_repos.sort_by_key(|r| r.to_lowercase());
        (repos, failed_repos)
    }

    /// Applies the output filters (currently only `--non-clean`) to a scan result.
    ///
    /// Every output format has to go through this, otherwise the formats disagree about
    /// which repositories the user asked to see.
    ///
    /// # Returns
    /// The repositories to display. Borrows the input when no filter is active.
    pub fn filter_repos<'a>(&self, repos: &'a [RepoInfo]) -> Cow<'a, [RepoInfo]> {
        if self.non_clean {
            Cow::Owned(
                repos
                    .iter()
                    .filter(|r| r.status != Status::Clean)
                    .cloned()
                    .collect(),
            )
        } else {
            Cow::Borrowed(repos)
        }
    }
}
