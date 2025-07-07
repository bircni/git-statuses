use std::process::Command;

use git2::{Repository, StatusOptions};

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
    /// Number of untracked files.
    pub untracked: usize,
    /// Number of changed (unstaged or staged) files.
    pub changed: usize,
    /// Status string: "Clean", "Dirty", or "?".
    pub status: String,
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
        let untracked = get_untracked_count(repo);
        let changed = get_changed_count(repo).saturating_add(untracked);
        let status = get_repo_status(repo);
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
            untracked,
            changed,
            status,
            has_unpushed,
            remote_url,
        })
    }
}

/// Returns the current branch name or a fallback if not available.
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

/// Returns (ahead, behind) tuple for the current branch vs. its upstream.
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

/// Returns the total number of commits in the current branch.
pub fn get_total_commits(repo: &Repository) -> anyhow::Result<usize> {
    let Ok(head) = repo.head() else { return Ok(0) };
    let Some(oid) = head.target() else {
        return Ok(0);
    };
    let mut revwalk = repo.revwalk()?;
    revwalk.push(oid)?;
    Ok(revwalk.count())
}

/// Returns the number of untracked files in the working directory.
pub fn get_untracked_count(repo: &Repository) -> usize {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true);
    repo.statuses(Some(&mut opts))
        .map(|statuses| statuses.iter().filter(|e| e.status().is_wt_new()).count())
        .unwrap_or(0)
}

/// Returns the number of changed (unstaged or staged) files.
pub fn get_changed_count(repo: &Repository) -> usize {
    let mut opts = StatusOptions::new();
    opts.include_untracked(false);
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
                })
                .count()
        })
        .unwrap_or(0)
}

/// Returns the status string for the repository: "Clean", "Dirty", or "?".
pub fn get_repo_status(repo: &Repository) -> String {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true).include_ignored(false);
    repo.statuses(Some(&mut opts)).map_or_else(
        |_| "?".to_owned(),
        |statuses| {
            if statuses.iter().all(|e| {
                e.status().is_ignored()
                    || !e.status().is_wt_new()
                        && !e.status().is_index_new()
                        && !e.status().is_wt_modified()
                        && !e.status().is_index_modified()
                        && !e.status().is_wt_deleted()
                        && !e.status().is_index_deleted()
                        && !e.status().is_conflicted()
            }) {
                "Clean".to_owned()
            } else {
                "Dirty".to_owned()
            }
        },
    )
}

/// Returns the remote URL for "origin", if available.
pub fn get_remote_url(repo: &Repository) -> Option<String> {
    repo.find_remote("origin")
        .ok()
        .and_then(|r| r.url().map(std::borrow::ToOwned::to_owned))
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
