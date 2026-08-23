//! Discovery of Git repositories on disk.
//!
//! Walks the requested directory, decides which entries are repositories the user asked
//! about, and collects their state in parallel. Kept out of `cli`, which is only concerned
//! with what the user typed.

use std::{ffi::OsStr, path::Path};

use rayon::iter::{IntoParallelRefIterator as _, ParallelIterator as _};
use walkdir::WalkDir;

use crate::{cli::Args, gitinfo::repoinfo::RepoInfo, util::GitPathExt as _};

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
pub fn find_repositories(args: &Args) -> (Vec<RepoInfo>, Vec<String>) {
    let walker = {
        let mut walk = WalkDir::new(&args.dir).min_depth(0).follow_links(false);

        // Any negative depth means "no limit"; `-1` is just the documented spelling.
        // A depth of 0 would find nothing at all, so it is treated like 1.
        if args.depth >= 0
            && let Ok(depth) = usize::try_from(args.depth.max(1))
        {
            walk = walk.max_depth(depth);
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

    // The scan root never changes, so resolve it once here instead of once per
    // repository inside `RepoInfo::new`.
    let root = args.dir.canonicalize().unwrap_or_else(|_| args.dir.clone());

    let scanned = walker
        .par_iter()
        .filter_map(|entry| scan_entry(args, entry.path(), &root))
        .collect::<Vec<_>>();

    let mut repos = Vec::with_capacity(scanned.len());
    let mut failed_repos = Vec::new();
    for result in scanned {
        match result {
            Ok(repo) => repos.push(repo),
            Err(name) => failed_repos.push(name),
        }
    }

    repos.sort_by_key(|r| r.repo_path.to_lowercase());
    failed_repos.sort_by_key(|r| r.to_lowercase());
    (repos, failed_repos)
}

/// Resolves a single walked directory into a repository result.
///
/// # Returns
/// `None` if the directory is not a repository the user asked about, `Err` with the
/// directory name if it is one but could not be read, and `Ok` otherwise.
fn scan_entry(args: &Args, orig_path: &Path, root: &Path) -> Option<Result<RepoInfo, String>> {
    let path_buf = if orig_path.is_git_directory() {
        orig_path.to_path_buf()
    } else {
        // Without a `--subdir` there is nowhere else to look, and if the subdir is not
        // a repository either then this directory is simply not one.
        let subdir_path = orig_path.join(args.subdir.as_ref()?);
        subdir_path.is_git_directory().then_some(subdir_path)?
    };

    match git2::Repository::open(&path_buf) {
        Ok(mut git_repo) => Some(
            RepoInfo::new(&mut git_repo, &orig_path.dir_name(), args, root).map_err(|e| {
                let name = orig_path.dir_name();
                log::debug!("Failed to read repository `{name}`: {e}");
                name
            }),
        ),
        Err(e) => {
            log::debug!("Failed to open repository at {}: {e}", path_buf.display());
            Some(Err(path_buf.dir_name()))
        }
    }
}
