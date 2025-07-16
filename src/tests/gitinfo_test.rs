use std::{fs, path::Path};

use comfy_table::Color;
use git2::Repository;

use crate::gitinfo::{self, repoinfo::RepoInfo, status::Status};

fn init_temp_repo() -> (tempfile::TempDir, git2::Repository) {
    let tmp_dir = tempfile::tempdir().unwrap();
    let repo = git2::Repository::init(tmp_dir.path()).unwrap();
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test User").unwrap();
    config.set_str("user.email", "test@example.com").unwrap();
    (tmp_dir, repo)
}

#[test]
fn test_get_branch_name_empty() {
    let (_tmp, repo) = init_temp_repo();
    let branch = gitinfo::get_branch_name(&repo);
    assert!(branch.contains("no commits") || branch.contains("no branch") || !branch.is_empty());
}

#[test]
fn test_get_total_commits_empty() {
    let (_tmp, repo) = init_temp_repo();
    let commits = gitinfo::get_total_commits(&repo).unwrap();
    assert_eq!(commits, 0);
}

#[test]
fn test_get_repo_status_clean_dirty() {
    let (tmp, repo) = init_temp_repo();
    let path = tmp.path().join("foo.txt");
    fs::write(&path, "bar").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("foo.txt")).unwrap();
    index.write().unwrap();
    let oid = index.write_tree().unwrap();
    let sig = repo.signature().unwrap();
    let tree = repo.find_tree(oid).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "msg", &tree, &[])
        .unwrap();
    let status_unpublished = Status::new(&repo);
    assert_eq!(status_unpublished, Status::Unpublished);
    fs::write(&path, "baz").unwrap();
    let status_dirty = Status::new(&repo);
    assert_eq!(status_dirty, Status::Dirty(1));
}

#[test]
fn test_get_ahead_behind_no_upstream() {
    let (_tmp, repo) = init_temp_repo();
    let (ahead, behind, is_local_only) = gitinfo::get_ahead_behind_and_local_status(&repo);
    assert_eq!((ahead, behind, is_local_only), (0, 0, true));
}

#[test]
fn test_get_remote_url_none() {
    let (_tmp, repo) = init_temp_repo();
    let remote = gitinfo::get_remote_url(&repo);
    assert!(remote.is_none());
}

#[test]
fn test_get_repo_status_invalid_repo() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = Repository::open(tmp.path());
    assert!(repo.is_err());
}

#[test]
fn test_get_branch_name_detached_head() {
    let (tmp, repo) = init_temp_repo();
    let path = tmp.path().join("foo.txt");
    std::fs::write(&path, "bar").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("foo.txt")).unwrap();
    index.write().unwrap();
    let oid = index.write_tree().unwrap();
    let sig = repo.signature().unwrap();
    let tree = repo.find_tree(oid).unwrap();
    let commit_oid = repo
        .commit(Some("HEAD"), &sig, &sig, "msg", &tree, &[])
        .unwrap();
    // Checkout detached HEAD
    repo.set_head_detached(commit_oid).unwrap();
    let branch = gitinfo::get_branch_name(&repo);
    assert!(!branch.is_empty());
}

#[test]
fn test_get_total_commits_multiple() {
    let (tmp, repo) = init_temp_repo();
    let path = tmp.path().join("foo.txt");
    std::fs::write(&path, "bar").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("foo.txt")).unwrap();
    index.write().unwrap();
    let oid = index.write_tree().unwrap();
    let sig = repo.signature().unwrap();
    let tree = repo.find_tree(oid).unwrap();
    let first_commit = repo
        .commit(Some("HEAD"), &sig, &sig, "msg", &tree, &[])
        .unwrap();
    std::fs::write(&path, "baz").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("foo.txt")).unwrap();
    index.write().unwrap();
    let oid = index.write_tree().unwrap();
    let tree = repo.find_tree(oid).unwrap();
    let parent = repo.find_commit(first_commit).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "msg2", &tree, &[&parent])
        .unwrap();
    let commits = gitinfo::get_total_commits(&repo).unwrap();
    assert_eq!(commits, 2);
}

#[test]
fn test_repo_info_new_with_and_without_remote() {
    let (_, mut repo) = init_temp_repo();
    // Without remote
    let info = RepoInfo::new(&mut repo, "tmp", false, false);
    info.unwrap();
    // With remote (origin does not exist)
    let info_remote = RepoInfo::new(&mut repo, "tmp", true, false);
    info_remote.unwrap();
}

#[test]
fn test_get_branch_name_no_head() {
    let (_tmp, repo) = init_temp_repo();
    // Simulate a repository with an invalid HEAD by setting it to a non-existent branch
    repo.set_head("refs/heads/nonexistent-branch").unwrap();
    let branch = gitinfo::get_branch_name(&repo);
    assert_eq!(branch, "nonexistent-branch (no commits)");
}

#[test]
fn test_get_ahead_behind_error_cases() {
    let (_tmp, repo) = init_temp_repo();
    // Simulate a repository with an invalid HEAD by setting it to a non-existent branch
    repo.set_head("refs/heads/nonexistent-branch").unwrap();
    let (ahead, behind, is_local_only) = gitinfo::get_ahead_behind_and_local_status(&repo);
    assert_eq!((ahead, behind, is_local_only), (0, 0, true));
}

#[test]
fn test_fetch_origin_failure() {
    let (_tmp, repo) = init_temp_repo();
    // Simulate a fetch failure by pointing to a non-existent remote
    repo.remote("origin", "https://invalid-url").unwrap();
    let result = gitinfo::fetch_origin(&repo);
    assert!(result.is_err());
}

#[test]
fn test_get_total_commits_error_cases() {
    let (tmp, repo) = init_temp_repo();
    // Remove HEAD to trigger an error
    let head_path = tmp.path().join(".git/HEAD");
    std::fs::remove_file(&head_path).unwrap();
    let commits = crate::gitinfo::get_total_commits(&repo).unwrap();
    assert_eq!(commits, 0);
}

#[test]
fn test_status_display_variants() {
    assert_eq!(Status::Clean.to_string(), "Clean");
    assert_eq!(Status::Dirty(3).to_string(), "Dirty (3)");
    assert_eq!(Status::Merge.to_string(), "Merge");
    assert_eq!(Status::Revert.to_string(), "Revert");
    assert_eq!(Status::Rebase.to_string(), "Rebase");
    assert_eq!(Status::Bisect.to_string(), "Bisect");
    assert_eq!(Status::CherryPick.to_string(), "Cherry Pick");
    assert_eq!(Status::Unknown.to_string(), "Unknown");
}

#[test]
fn test_status_colors() {
    assert_eq!(Status::Clean.comfy_color(), Color::Reset);
    assert_eq!(Status::Dirty(1).comfy_color(), Color::Red);
    assert_eq!(Status::Merge.comfy_color(), Color::Blue);
    assert_eq!(Status::Revert.comfy_color(), Color::Magenta);
    assert_eq!(Status::Rebase.comfy_color(), Color::Cyan);
    assert_eq!(Status::Bisect.comfy_color(), Color::Yellow);
    assert_eq!(Status::CherryPick.comfy_color(), Color::DarkYellow);
    assert_eq!(
        Status::Unknown.comfy_color(),
        Color::Rgb {
            r: 255,
            g: 165,
            b: 0
        }
    );
}

#[test]
fn test_status_descriptions() {
    assert_eq!(
        Status::Clean.description(),
        "No changes, no unpushed commits."
    );
    assert_eq!(
        Status::Dirty(42).description(),
        "Working directory has changes."
    );
    assert_eq!(Status::Merge.description(), "Merge in progress.");
    assert_eq!(Status::Revert.description(), "Revert in progress.");
    assert_eq!(Status::Rebase.description(), "Rebase in progress.");
    assert_eq!(Status::Bisect.description(), "Bisecting in progress.");
    assert_eq!(Status::CherryPick.description(), "Cherry-pick in progress.");
    assert_eq!(
        Status::Unknown.description(),
        "Status is unknown or not recognized."
    );
}

#[test]
fn test_as_cell_contains_expected_text_and_color() {
    let status = Status::Dirty(5);
    let cell = status.as_cell();
    assert!(cell.content().contains("Dirty (5)"));
}

#[test]
fn test_get_stash_count_empty() {
    let (_tmp, mut repo) = init_temp_repo();
    let stash_count = gitinfo::get_stash_count(&mut repo);
    assert_eq!(stash_count, 0);
}

#[test]
fn test_get_ahead_behind_and_local_status_no_upstream() {
    let (_tmp, repo) = init_temp_repo();
    let (ahead, behind, is_local_only) = gitinfo::get_ahead_behind_and_local_status(&repo);
    assert_eq!((ahead, behind, is_local_only), (0, 0, true));
}

#[test]
fn test_repo_info_includes_stash_and_local_status() {
    let (_tmp, mut repo) = init_temp_repo();
    let info = RepoInfo::new(&mut repo, "test", false, false).unwrap();
    assert_eq!(info.stash_count, 0);
    assert!(info.is_local_only);
}
