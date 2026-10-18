use std::path::PathBuf;

use git2::Repository;

use crate::{
    cli::Args,
    gitinfo::{self, status::Status},
};

/// Holds information about a Git repository for status display.
#[expect(
    clippy::struct_excessive_bools,
    reason = "This structure holds repository state flags that are naturally represented as booleans"
)]
#[derive(serde::Serialize, Clone)]
pub struct RepoInfo {
    /// The directory name of the repository.
    pub name: String,
    /// The current branch name.
    pub branch: String,
    /// Number of commits ahead of upstream.
    pub ahead: usize,
    /// Number of commits behind upstream.
    pub behind: usize,
    /// Total number of commits in the current branch.
    pub commits: usize,
    /// Status of the repository.
    pub status: Status,
    /// True if there are unpushed commits.
    pub has_unpushed: bool,
    /// Remote URL (if available).
    pub remote_url: Option<String>,
    /// Path to the repository directory.
    pub path: PathBuf,
    /// Number of stashes in the repository.
    pub stash_count: usize,
    /// True if the current branch has no upstream (local-only).
    pub is_local_only: bool,
    /// True if the repository was fast-forwarded
    pub fast_forwarded: bool,
    /// relative path from the starting directory
    pub repo_path: String,
    /// True if this is a Git worktree
    pub is_worktree: bool,
    /// True if HEAD is detached, i.e. not on any branch
    pub is_detached: bool,
}

impl RepoInfo {
    /// Creates a new `RepoInfo` instance.
    /// # Arguments
    /// * `repo` - The Git repository to gather information from.
    /// * `name` - Fallback name, used when the repository has no remote to derive one from.
    /// * `args` - CLI arguments; `--remote`, `--fetch` and `--ff` are read from here.
    /// * `repo_path` - The path to show for the repository, relative to the scan root.
    ///
    /// # Returns
    /// A `RepoInfo` instance containing the repository's status information.
    ///
    /// # Errors
    /// Returns an error if the commit history of the repository cannot be walked.
    ///
    /// Fetching and fast-forwarding are best-effort: a repository without a remote
    /// or without an upstream branch is still reported, with a warning logged.
    pub fn new(
        repo: &mut Repository,
        name: &str,
        args: &Args,
        repo_path: &str,
    ) -> anyhow::Result<Self> {
        let name = gitinfo::get_repo_name(repo).unwrap_or_else(|| name.to_owned());

        // Fetching and merging must happen before any state is gathered, otherwise the
        // reported ahead/behind counts, commit count and status describe the pre-merge
        // repository and contradict the fast-forward marker shown next to them.
        if (args.fetch || args.fast_forward)
            && let Err(e) = gitinfo::fetch_remote(repo)
        {
            log::warn!("Failed to fetch for `{name}`: {e}");
        }
        let fast_forwarded = args.fast_forward
            && gitinfo::merge_ff(repo).unwrap_or_else(|e| {
                log::warn!("Failed to fast-forward `{name}`: {e}");
                false
            });

        let branch = gitinfo::get_branch_name(repo);
        let (ahead, behind, is_local_only) = gitinfo::get_ahead_behind_and_local_status(repo);
        let commits = gitinfo::get_total_commits(repo)?;
        let status = Status::new(repo);
        let has_unpushed = ahead > 0;
        let remote_url = if args.remote {
            gitinfo::get_remote_url(repo)
        } else {
            None
        };
        let path = gitinfo::get_repo_path(repo);
        let stash_count = gitinfo::get_stash_count(repo);
        let is_worktree = repo.is_worktree();
        let is_detached = repo.head_detached().unwrap_or(false);

        Ok(Self {
            name,
            branch,
            ahead,
            behind,
            commits,
            status,
            has_unpushed,
            remote_url,
            path,
            stash_count,
            is_local_only,
            fast_forwarded,
            repo_path: repo_path.to_owned(),
            is_worktree,
            is_detached,
        })
    }

    /// Formats the local status showing ahead/behind counts or local-only indication.
    /// # Returns
    /// A formatted string showing ahead/behind counts or local-only indication, or `-` for
    /// a detached HEAD, which has no branch to compare.
    pub fn format_local_status(&self) -> String {
        if self.is_detached {
            "-".to_owned()
        } else if self.is_local_only {
            "local-only".to_owned()
        } else {
            format!("↑{} ↓{}", self.ahead, self.behind)
        }
    }

    /// Formats the status with stash information if stashes are present.
    /// # Returns
    /// A formatted string showing status and stash count if present.
    pub fn format_status_with_stash_and_ff(&self) -> String {
        use std::fmt::Write as _;

        let mut status = self.status.to_string();
        if self.stash_count > 0 {
            // Writing into the buffer keeps this to one allocation; `format!` per suffix
            // would build and throw away a fresh `String` each time.
            let _ = write!(status, " ({}*)", self.stash_count);
        }
        if self.fast_forwarded {
            status.push_str(" ↑↑");
        }
        status
    }
}
