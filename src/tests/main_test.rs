use std::path::PathBuf;

use clap::Parser;

use crate::{
    cli::Args,
    gitinfo::{repoinfo::RepoInfo, status::Status},
};

fn repo_info_with_status(status: Status, stash_count: usize, fast_forwarded: bool) -> RepoInfo {
    RepoInfo {
        name: "repo".to_owned(),
        branch: "main".to_owned(),
        ahead: 3,
        behind: 1,
        commits: 42,
        status,
        has_unpushed: true,
        remote_url: Some("https://example.com/repo.git".to_owned()),
        path: PathBuf::from("/tmp/repo"),
        stash_count,
        is_local_only: false,
        fast_forwarded,
        repo_path: "repo".to_owned(),
        is_worktree: false,
    }
}

#[test]
fn test_repo_info_format_local_status_local_only() {
    let mut repo = repo_info_with_status(Status::Clean, 0, false);
    repo.is_local_only = true;
    assert_eq!(repo.format_local_status(), "local-only");
}

#[test]
fn test_repo_info_format_local_status_with_upstream_counts() {
    let repo = repo_info_with_status(Status::Clean, 0, false);
    assert_eq!(repo.format_local_status(), "↑3 ↓1");
}

#[test]
fn test_repo_info_format_status_with_stash_only() {
    let repo = repo_info_with_status(Status::Dirty(2), 4, false);
    assert_eq!(repo.format_status_with_stash_and_ff(), "Dirty (2) (4*)");
}

#[test]
fn test_repo_info_format_status_with_fast_forward_only() {
    let repo = repo_info_with_status(Status::Clean, 0, true);
    assert_eq!(repo.format_status_with_stash_and_ff(), "Clean ↑↑");
}

#[test]
fn test_repo_info_format_status_with_stash_and_fast_forward() {
    let repo = repo_info_with_status(Status::Unpushed, 2, true);
    assert_eq!(repo.format_status_with_stash_and_ff(), "Unpushed (2*) ↑↑");
}

#[test]
fn test_args_parse_json_fast_forward_and_subdir() {
    let args = Args::parse_from([
        "git-statuses",
        "--json",
        "--ff",
        "--subdir",
        "checkout",
        "--depth=-1",
        ".",
    ]);
    assert!(args.json);
    assert!(args.fast_forward);
    assert_eq!(args.subdir.as_deref(), Some("checkout"));
    assert_eq!(args.depth, -1);
}
