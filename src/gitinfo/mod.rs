use std::process::Command;

use git2::{Repository, StatusOptions};

use crate::gitinfo::status::Status;

pub mod status;

/// Holds information about a Git repository for status display.
#[derive(Clone)]
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
        repo: &Repository,
        name: &str,
        show_remote: bool,
        fetch: bool,
    ) -> anyhow::Result<Self> {
        if fetch {
            // Attempt to fetch from origin, ignoring errors
            fetch_origin(repo)?;
        }
        let branch = get_branch_name(repo);
        let (ahead, behind) = get_ahead_behind(repo);
        let commits = get_total_commits(repo)?;
        let status = Status::new(repo);
        let has_unpushed = ahead > 0;
        let remote_url = if show_remote {
            get_remote_url(repo)
        } else {
            None
        };

        Ok(Self {
            name: name.to_owned(),
            branch,
            ahead,
            behind,
            commits,
            status,
            has_unpushed,
            remote_url,
        })
    }
}

/// Returns the current branch name or a fallback if not available.
/// If the HEAD is detached or not pointing to a branch,
/// it returns the symbolic target of HEAD or "(no branch)" if no commits exist.
/// # Arguments
/// * `repo` - The Git repository to check for the branch name.
/// # Returns
/// A `String` containing the branch name or a fallback message.
pub fn get_branch_name(repo: &Repository) -> String {
    if let Ok(head) = repo.head() {
        if let Some(name) = head.shorthand() {
            return name.to_owned();
        }
        if let Some(target) = head.symbolic_target()
            && let Some(branch) = target.rsplit('/').next()
        {
            return format!("{branch} (no commits)");
        }
    } else if let Ok(headref) = repo.find_reference("HEAD")
        && let Some(sym) = headref.symbolic_target()
        && let Some(branch) = sym.rsplit('/').next()
    {
        return format!("{branch} (no commits)");
    }
    "(no branch)".to_owned()
}

/// Get the number of commits ahead and behind the upstream branch.
/// If the current branch has no upstream, it returns (0, 0).
/// # Arguments
/// * `repo` - The Git repository to check for ahead/behind status.
/// # Returns
/// A tuple containing the number of commits ahead and behind the upstream branch.
pub fn get_ahead_behind(repo: &Repository) -> (usize, usize) {
    let Ok(head) = repo.head() else { return (0, 0) };
    let branch = head.shorthand().map_or_else(
        || None,
        |name| repo.find_branch(name, git2::BranchType::Local).ok(),
    );
    if let Some(branch) = branch
        && let Ok(upstream) = branch.upstream()
    {
        let local_oid = branch.get().target();
        let upstream_oid = upstream.get().target();
        if let (Some(local), Some(up)) = (local_oid, upstream_oid) {
            return repo.graph_ahead_behind(local, up).unwrap_or((0, 0));
        }
    }
    (0, 0)
}

/// Gets the total number of commits in the current branch.
/// If the HEAD is detached or not pointing to a branch,
/// # Arguments
/// * `repo` - The Git repository to check for total commits.
/// # Returns
/// The total number of commits in the current branch.
/// # Errors
/// Returns an error if the repository cannot be accessed or if the revwalk fails.
pub fn get_total_commits(repo: &Repository) -> anyhow::Result<usize> {
    let Ok(head) = repo.head() else { return Ok(0) };
    let Some(oid) = head.target() else {
        return Ok(0);
    };
    let mut revwalk = repo.revwalk()?;
    revwalk.push(oid)?;
    Ok(revwalk.count())
}

/// Returns the number of changed (unstaged, staged or untracked) files.
pub fn get_changed_count(repo: &Repository) -> usize {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true);
    repo.statuses(Some(&mut opts))
        .map(|statuses| {
            statuses
                .iter()
                .filter(|e| {
                    let s = e.status();
                    s.is_wt_modified()
                        || s.is_index_modified()
                        || s.is_wt_deleted()
                        || s.is_index_deleted()
                        || s.is_conflicted()
                        || s.is_wt_new()
                        || s.is_index_new()
                })
                .count()
        })
        .unwrap_or(0)
}

/// Returns the remote URL for "origin", if available.
pub fn get_remote_url(repo: &Repository) -> Option<String> {
    repo.find_remote("origin")
        .ok()
        .and_then(|r| r.url().map(ToOwned::to_owned))
}

/// Executes a fetch operation for the "origin" remote to update upstream information.
pub fn fetch_origin(repo: &Repository) -> anyhow::Result<()> {
    let path = repo
        .path()
        .parent()
        .ok_or_else(|| anyhow::anyhow!("No parent directory found"))?;
    let output = Command::new("git")
        .arg("fetch")
        .arg("origin")
        .current_dir(path)
        .output()?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to fetch from origin: {}",
            String::from_utf8_lossy(&output.stderr)
        )
    }

    Ok(())
}
