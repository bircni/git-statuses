use std::path::{Path, PathBuf};

use git2::Repository;

use crate::{
    gitinfo::{self, status::Status},
    util::GitPathExt as _,
};

/// Holds information about a Git repository for status display.
#[expect(
    clippy::struct_excessive_bools,
    reason = "This structure holds repository state flags that are naturally represented as booleans"
)]
#[derive(serde::Serialize, serde::Deserialize, Clone)]
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
}

impl RepoInfo {
    /// Creates a new `RepoInfo` instance.
    /// # Arguments
    /// * `repo` - The Git repository to gather information from.
    /// * `show_remote` - Whether to include the remote URL in the info.
    /// * `fetch` - Whether to run a fetch operation before gathering info.
    /// * `path` - The path to the repository directory.
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
        show_remote: bool,
        fetch: bool,
        merge: bool,
        dir: &Path,
    ) -> anyhow::Result<Self> {
        let name = gitinfo::get_repo_name(repo).unwrap_or_else(|| name.to_owned());

        // Fetching and merging must happen before any state is gathered, otherwise the
        // reported ahead/behind counts, commit count and status describe the pre-merge
        // repository and contradict the fast-forward marker shown next to them.
        if (fetch || merge)
            && let Err(e) = gitinfo::fetch_origin(repo)
        {
            log::warn!("Failed to fetch for `{name}`: {e}");
        }
        let fast_forwarded = merge
            && gitinfo::merge_ff(repo).unwrap_or_else(|e| {
                log::warn!("Failed to fast-forward `{name}`: {e}");
                false
            });

        let branch = gitinfo::get_branch_name(repo);
        let (ahead, behind, is_local_only) = gitinfo::get_ahead_behind_and_local_status(repo);
        let commits = gitinfo::get_total_commits(repo)?;
        let status = Status::new(repo);
        let has_unpushed = ahead > 0;
        let remote_url = if show_remote {
            gitinfo::get_remote_url(repo)
        } else {
            None
        };
        let path = gitinfo::get_repo_path(repo);
        let stash_count = gitinfo::get_stash_count(repo);
        let repo_path = path.canonicalize().unwrap_or_else(|_| path.clone());
        let root_path = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
        let repo_path_relative = repo_path.strip_prefix(&root_path).unwrap_or(&repo_path);
        // The scanned directory is the repository itself when git-statuses is run from
        // inside one, which leaves the relative path empty. Fall back to the directory
        // name, so the column reads like it would for a repository one level down instead
        // of suddenly showing an absolute path.
        let repo_path = if repo_path_relative.as_os_str().is_empty() {
            repo_path.dir_name()
        } else {
            repo_path_relative.display().to_string()
        };
        let is_worktree = repo.is_worktree();

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
            repo_path,
            is_worktree,
        })
    }

    /// Formats the local status showing ahead/behind counts or local-only indication.
    /// # Returns
    /// A formatted string showing ahead/behind counts or local-only indication.
    pub fn format_local_status(&self) -> String {
        if self.is_local_only {
            "local-only".to_owned()
        } else {
            format!("↑{} ↓{}", self.ahead, self.behind)
        }
    }

    /// Formats the status with stash information if stashes are present.
    /// # Returns
    /// A formatted string showing status and stash count if present.
    pub fn format_status_with_stash_and_ff(&self) -> String {
        let mut status_str = self.status.to_string();
        if self.stash_count > 0 {
            status_str = format!("{status_str} ({}*)", self.stash_count);
        }
        if self.fast_forwarded {
            status_str = format!("{status_str} ↑↑");
        }
        status_str
    }
}
