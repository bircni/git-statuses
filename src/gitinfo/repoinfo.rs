use std::path::PathBuf;

use git2::Repository;
use serde::Serialize;

use crate::gitinfo::{self, status::Status};

/// Holds information about a Git repository for status display.
#[derive(Clone, Serialize)]
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
    /// Returns an error if the repository cannot be opened, or if fetching fails.
    /// If `fetch` is true, it will attempt to fetch from the "origin"
    /// remote to update upstream information.
    /// If fetching fails, it will use that error to return an error.
    pub fn new(
        repo: &mut Repository,
        name: &str,
        show_remote: bool,
        fetch: bool,
    ) -> anyhow::Result<Self> {
        if fetch {
            // Attempt to fetch from origin, ignoring errors
            gitinfo::fetch_origin(repo)?;
        }
        let name = gitinfo::get_repo_name(repo).unwrap_or_else(|| name.to_owned());
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
    pub fn format_status_with_stash(&self) -> String {
        let status_str = self.status.to_string();
        if self.stash_count > 0 {
            format!("{status_str} ({}*)", self.stash_count)
        } else {
            status_str
        }
    }
}
