use std::{fs, path::Path};

use git2::Repository;

use crate::gitinfo;

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
fn test_get_untracked_count() {
    let (tmp, repo) = init_temp_repo();
    let path = tmp.path().join("foo.txt");
    fs::write(&path, "bar").unwrap();
    let count = gitinfo::get_untracked_count(&repo);
    assert_eq!(count, 1);
}

#[test]
fn test_get_changed_count() {
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
    fs::write(&path, "baz").unwrap();
    let changed = gitinfo::get_changed_count(&repo);
    assert_eq!(changed, 1);
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
    let status_clean = gitinfo::get_repo_status(&repo);
    assert_eq!(status_clean, "Clean");
    fs::write(&path, "baz").unwrap();
    let status_dirty = gitinfo::get_repo_status(&repo);
    assert_eq!(status_dirty, "Dirty");
}

#[test]
fn test_get_ahead_behind_no_upstream() {
    let (_tmp, repo) = init_temp_repo();
    let (ahead, behind) = gitinfo::get_ahead_behind(&repo);
    assert_eq!((ahead, behind), (0, 0));
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
    let (tmp, repo) = init_temp_repo();
    // Without remote
    let info = crate::gitinfo::RepoInfo::new(&repo, false, false, tmp.path());
    info.unwrap();
    // With remote (origin does not exist)
    let info_remote = crate::gitinfo::RepoInfo::new(&repo, true, false, tmp.path());
    info_remote.unwrap();
}

#[test]
fn test_get_branch_name_no_head() {
    let (tmp, repo) = init_temp_repo();
    // Remove HEAD
    let head_path = tmp.path().join(".git/HEAD");
    std::fs::remove_file(&head_path).unwrap();
    let branch = crate::gitinfo::get_branch_name(&repo);
    assert_eq!(branch, "(no branch)");
}

#[test]
fn test_get_ahead_behind_error_cases() {
    let (tmp, repo) = init_temp_repo();
    // Remove HEAD to trigger an error
    let head_path = tmp.path().join(".git/HEAD");
    std::fs::remove_file(&head_path).unwrap();
    let (ahead, behind) = crate::gitinfo::get_ahead_behind(&repo);
    assert_eq!((ahead, behind), (0, 0));
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
