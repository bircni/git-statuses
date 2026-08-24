use std::{path, process::Command};

use git2::{Branch, Repository, StatusOptions};

use crate::gitinfo::status::Status;

pub mod repoinfo;
pub mod status;

/// The status bits that make a working directory count as changed.
///
/// Shared by the clean/dirty decision and by the change counter shown next to it, so the
/// two can never disagree about what a change is. `TYPECHANGE` matters for files that were
/// swapped for a symlink (or vice versa), `RENAMED` for renames once git detects them.
pub const CHANGED: git2::Status = git2::Status::WT_NEW
    .union(git2::Status::WT_MODIFIED)
    .union(git2::Status::WT_DELETED)
    .union(git2::Status::WT_TYPECHANGE)
    .union(git2::Status::WT_RENAMED)
    .union(git2::Status::INDEX_NEW)
    .union(git2::Status::INDEX_MODIFIED)
    .union(git2::Status::INDEX_DELETED)
    .union(git2::Status::INDEX_TYPECHANGE)
    .union(git2::Status::INDEX_RENAMED)
    .union(git2::Status::CONFLICTED);

/// Gets the first available remote name, preferring "origin".
/// If "origin" doesn't exist, it returns the first available remote.
/// # Arguments
/// * `repo` - The Git repository to check for remotes.
/// # Returns
/// An `Option<String>` containing the remote name if found, or `None` if no remotes exist.
fn get_remote_name(repo: &Repository) -> Option<String> {
    // Try "origin" first
    if repo.find_remote("origin").is_ok() {
        return Some("origin".to_owned());
    }

    // Otherwise, get the first available remote
    repo.remotes()
        .ok()
        .and_then(|remotes| remotes.get(0).ok().flatten().map(ToOwned::to_owned))
}

/// Gets the path of the repository.
/// If the path ends with `.git`, it returns the parent directory.
/// For worktrees, returns the worktree's working directory path.
/// # Arguments
/// * `repo` - The Git repository to check for the path.
/// # Returns
/// A `PathBuf` containing the repository path.
fn get_repo_path(repo: &Repository) -> path::PathBuf {
    // For worktrees, workdir() returns the actual working directory
    if let Some(workdir) = repo.workdir() {
        return workdir.to_path_buf();
    }

    // Fallback for bare repos or edge cases
    let path = repo.path();
    if path.ends_with(".git") {
        path.parent().unwrap_or(path).to_path_buf()
    } else {
        path.to_path_buf()
    }
}

/// Extracts the repository name from a remote URL.
///
/// Handles the shapes git accepts: `https://host/user/repo.git`, the SCP-like
/// `git@host:user/repo.git` (and its slash-less `git@host:repo.git` form), local paths,
/// and any of those with a trailing slash.
///
/// # Arguments
/// * `url` - The remote URL to parse.
/// # Returns
/// The repository name, or `None` if the URL carries no usable name.
pub fn repo_name_from_url(url: &str) -> Option<String> {
    // A trailing slash would otherwise make the last segment empty.
    let url = url.trim_end_matches('/');
    // Both separators matter: in `git@host:repo.git` the name is not preceded by a slash.
    let name = url.rsplit(['/', ':', '\\']).next()?;
    // `strip_suffix` rather than `trim_end_matches`, which would strip a repeated suffix
    // and turn `repo.git.git` into `repo`.
    let name = name.strip_suffix(".git").unwrap_or(name);

    (!name.is_empty()).then(|| name.to_owned())
}

/// Gets the name of the repository from the remote URL.
/// If the remote URL is not available, it returns `None`.
/// # Arguments
/// * `repo` - The Git repository to check for the name.
/// # Returns
/// An `Option<String>` containing the repository name if found, or `None` if not.
fn get_repo_name(repo: &Repository) -> Option<String> {
    repo_name_from_url(&get_remote_url(repo)?)
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
            if let Ok(name) = head.shorthand() {
                return name.to_owned();
            }
        } else {
            // Detached HEAD
            return "N/A".to_owned();
        }
        if let Ok(Some(target)) = head.symbolic_target()
            && let Some(branch) = target.rsplit('/').next()
        {
            return format!("{branch} (no commits)");
        }
    } else if let Ok(headref) = repo.find_reference("HEAD")
        && let Ok(Some(sym)) = headref.symbolic_target()
        && let Some(branch) = sym.rsplit('/').next()
    {
        return format!("{branch} (no commits)");
    }
    "(no branch)".to_owned()
}

/// Resolves the commit the current branch is measured against.
///
/// Prefers the configured upstream (`branch.<name>.merge`) - the same thing `git status`
/// and `@{u}` mean by "upstream". When a branch has no upstream configured, falls back to
/// a remote-tracking ref of the same name, so a branch that was fetched but never set to
/// track is not treated as if it had no remote at all.
///
/// Both the ahead/behind counts and the push status go through this, so the two can never
/// disagree about whether a branch has an upstream.
///
/// # Arguments
/// * `repo` - The Git repository to resolve in.
/// * `branch_name` - The short name of the local branch.
/// # Returns
/// The upstream commit, or `None` if the branch has no upstream under either rule.
fn upstream_oid(repo: &Repository, branch_name: &str) -> Option<git2::Oid> {
    if let Ok(branch) = repo.find_branch(branch_name, git2::BranchType::Local)
        && let Ok(upstream) = branch.upstream()
        && let Some(oid) = upstream.get().target()
    {
        return Some(oid);
    }

    let remote_name = get_remote_name(repo)?;
    repo.find_reference(&format!("refs/remotes/{remote_name}/{branch_name}"))
        .ok()?
        .target()
}

/// Get the number of commits ahead and behind the upstream branch, and whether the branch is local-only.
/// If the current branch has no upstream, it returns (0, 0, true).
/// # Arguments
/// * `repo` - The Git repository to check for ahead/behind status.
/// # Returns
/// A tuple containing the number of commits ahead, behind, and whether the branch is local-only.
pub fn get_ahead_behind_and_local_status(repo: &Repository) -> (usize, usize, bool) {
    const LOCAL_ONLY: (usize, usize, bool) = (0, 0, true);

    let Ok(head) = repo.head() else {
        return LOCAL_ONLY;
    };
    let (Ok(branch_name), Some(local_oid)) = (head.shorthand(), head.target()) else {
        return LOCAL_ONLY;
    };
    let Some(upstream) = upstream_oid(repo, branch_name) else {
        return LOCAL_ONLY;
    };

    let (ahead, behind) = repo
        .graph_ahead_behind(local_oid, upstream)
        .unwrap_or((0, 0));
    (ahead, behind, false)
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

/// Counts the changed (staged, unstaged or untracked) entries in a status list.
///
/// Takes an already-collected `Statuses` so the caller that decides clean-vs-dirty can
/// reuse its own walk instead of asking the repository a second time.
pub fn count_changed(statuses: &git2::Statuses<'_>) -> usize {
    statuses
        .iter()
        .filter(|e| !e.status().is_ignored() && e.status().intersects(CHANGED))
        .count()
}

/// The `StatusOptions` used everywhere a working directory is inspected.
///
/// Shared so the clean/dirty decision, the change counter and any future caller can never
/// disagree about which entries git is asked to report.
pub fn status_options() -> StatusOptions {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true).include_ignored(false);
    opts
}

/// Returns the remote URL for the first available remote (preferring "origin"), if available.
pub fn get_remote_url(repo: &Repository) -> Option<String> {
    let remote_name = get_remote_name(repo)?;
    repo.find_remote(&remote_name)
        .ok()
        .and_then(|r| r.url().map(ToOwned::to_owned).ok())
}

/// Fetches from the first available remote (preferring "origin") to update upstream information.
///
/// Shells out to `git` rather than using `git2` so that the user's credential helpers,
/// SSH agent and proxy configuration apply exactly as they do on the command line;
/// reimplementing that against `git2` would mean reimplementing authentication.
///
/// # Errors
/// Returns an error if the repository has no remote, no directory to run in, or if `git
/// fetch` itself fails.
pub fn fetch_remote(repo: &Repository) -> anyhow::Result<()> {
    let remote_name = get_remote_name(repo).ok_or_else(|| anyhow::anyhow!("No remotes found"))?;
    // `repo.path()` is the git directory. For a worktree that is
    // `<main>/.git/worktrees/<name>`, whose parent is not a working directory at all, so
    // prefer the working directory and only fall back for bare repositories.
    let path = repo
        .workdir()
        .or_else(|| repo.path().parent())
        .ok_or_else(|| anyhow::anyhow!("No working directory found"))?;
    let output = Command::new("git")
        .arg("fetch")
        .arg(&remote_name)
        .current_dir(path)
        .output()?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to fetch from {}: {}",
            remote_name,
            String::from_utf8_lossy(&output.stderr)
        )
    }

    Ok(())
}

/// Executes a fast-forward merge to update local checkout
pub fn merge_ff(repo: &Repository) -> anyhow::Result<bool> {
    let head = repo.head()?;

    if head.is_branch() {
        let branch = Branch::wrap(head);
        let upstream = branch.upstream()?;
        let upstream_head_commit = repo.reference_to_annotated_commit(upstream.get())?;

        // If fast-forward merge is possible and the user doesn't explicitly forbids it, let's proceed
        if let Ok((merge_analysis, merge_pref)) = repo.merge_analysis(&[&upstream_head_commit])
            && merge_analysis.is_fast_forward()
            && !merge_pref.is_no_fast_forward()
        {
            let upstream_head_commit_id = upstream_head_commit.id();
            repo.checkout_tree(&repo.find_object(upstream_head_commit_id, None)?, None)?;
            repo.head()?
                .set_target(upstream_head_commit_id, "updated by git-statuses")?;
            return Ok(true);
        }
    }

    Ok(false)
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

    let (Ok(branch_name), Some(local_oid)) = (head.shorthand(), head.target()) else {
        return Status::Unknown;
    };

    let Some(remote_oid) = upstream_oid(repo, branch_name) else {
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
