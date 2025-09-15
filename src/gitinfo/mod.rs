use std::{
    path::{self},
    process::Command,
};

use git2::{Repository, StatusOptions};

use crate::gitinfo::status::Status;

pub mod repoinfo;
pub mod status;

/// Gets the path of the repository.
/// If the path ends with `.git`, it returns the parent directory.
/// # Arguments
/// * `repo` - The Git repository to check for the path.
/// # Returns
/// A `PathBuf` containing the repository path.
fn get_repo_path(repo: &Repository) -> path::PathBuf {
    let path = repo.path();
    if path.ends_with(".git") {
        path.parent().unwrap_or(path).to_path_buf()
    } else {
        path.to_path_buf()
    }
}

/// Gets the name of the repository from the remote URL.
/// If the remote URL is not available, it returns `None`.
/// # Arguments
/// * `repo` - The Git repository to check for the name.
/// # Returns
/// An `Option<String>` containing the repository name if found, or `None` if not.
fn get_repo_name(repo: &Repository) -> Option<String> {
    if let Ok(remote) = repo.find_remote("origin")
        && let Some(url) = remote.url()
    {
        // Remove trailing "/" and ".git" if present
        let url = url.trim_end_matches('/').trim_end_matches(".git");
        return Some(url.split('/').next_back().unwrap_or("unknown").to_owned());
    }
    None
}

/// Returns the current branch name or a fallback if not available.
/// If the HEAD is detached, it returns "N/A".
/// If not pointing to a branch, it returns the symbolic target of HEAD or "(no branch)" if no commits exist.
/// # Arguments
/// * `repo` - The Git repository to check for the branch name.
/// # Returns
/// A `String` containing the branch name or a fallback message.
pub fn get_branch_name(repo: &Repository) -> String {
    if let Ok(head) = repo.head() {
        if head.is_branch() {
            if let Some(name) = head.shorthand() {
                return name.to_owned();
            }
        } else {
            // Detached HEAD
            return "N/A".to_owned();
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

/// Get the number of commits ahead and behind the upstream branch, and whether the branch is local-only.
/// If the current branch has no upstream, it returns (0, 0, true).
/// # Arguments
/// * `repo` - The Git repository to check for ahead/behind status.
/// # Returns
/// A tuple containing the number of commits ahead, behind, and whether the branch is local-only.
pub fn get_ahead_behind_and_local_status(repo: &Repository) -> (usize, usize, bool) {
    let Ok(head) = repo.head() else {
        return (0, 0, true);
    };
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
            let (ahead, behind) = repo.graph_ahead_behind(local, up).unwrap_or((0, 0));
            return (ahead, behind, false);
        }
    }
    (0, 0, true)
}

/// Gets the total number of commits in the current branch.
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

/// Checks if the current branch is unpushed or has unpushed commits.
/// Returns `true` if the branch is not published or ahead of its remote.
pub fn get_branch_push_status(repo: &Repository) -> Status {
    let Ok(head) = repo.head() else {
        return Status::Unknown;
    };

    if !head.is_branch() {
        return Status::Detached;
    }

    let Some(local_branch) = head.shorthand() else {
        return Status::Unknown;
    };

    let Some(local_oid) = head.target() else {
        return Status::Unknown;
    };

    let Ok(remote_ref) = repo.find_reference(&format!("refs/remotes/origin/{local_branch}")) else {
        return Status::Unpublished;
    };

    let Some(remote_oid) = remote_ref.target() else {
        return Status::Unpublished;
    };

    match repo.graph_ahead_behind(local_oid, remote_oid) {
        Ok((ahead, _)) if ahead > 0 => Status::Unpushed,
        Ok(_) => Status::Clean,
        Err(_) => Status::Unknown,
    }
}

/// Returns the number of stashes in the repository.
/// # Arguments
/// * `repo` - The Git repository to check for stashes.
/// # Returns
/// The number of stashes in the repository.
/// Returns the number of stashes in the repository using `git2`.
pub fn get_stash_count(repo: &mut Repository) -> usize {
    let mut count = 0;
    let _ = repo.stash_foreach(|_, _, _| {
        count += 1;
        true // continue iterating
    });
    count
}
