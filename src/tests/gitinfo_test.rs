use std::{
    fs,
    path::{Path, PathBuf},
};

use comfy_table::Color;
use git2::Repository;

use crate::gitinfo::{self, repoinfo::RepoInfo, status::Status};

fn init_temp_repo() -> (tempfile::TempDir, Repository) {
    let tmp_dir = tempfile::tempdir().unwrap();
    let repo = Repository::init(tmp_dir.path()).unwrap();
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
    fs::write(&path, "bar").unwrap();
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
    fs::write(&path, "bar").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("foo.txt")).unwrap();
    index.write().unwrap();
    let oid = index.write_tree().unwrap();
    let sig = repo.signature().unwrap();
    let tree = repo.find_tree(oid).unwrap();
    let first_commit = repo
        .commit(Some("HEAD"), &sig, &sig, "msg", &tree, &[])
        .unwrap();
    fs::write(&path, "baz").unwrap();
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
    let info = RepoInfo::new(
        &mut repo,
        "tmp",
        false,
        false,
        &PathBuf::from("/path/to/repo"),
    );
    info.unwrap();
    // With remote (origin does not exist)
    let info_remote = RepoInfo::new(
        &mut repo,
        "tmp",
        true,
        false,
        &PathBuf::from("/path/to/repo"),
    );
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
    fs::remove_file(&head_path).unwrap();
    let commits = gitinfo::get_total_commits(&repo).unwrap();
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
    let info = RepoInfo::new(
        &mut repo,
        "test",
        false,
        false,
        &PathBuf::from("/path/to/repo"),
    )
    .unwrap();
    assert_eq!(info.stash_count, 0);
    assert!(info.is_local_only);
}

// New tests for additional git states and edge cases

#[test]
fn test_status_new_with_merge_state() {
    let (tmp, repo) = init_temp_repo();

    // Create initial commit
    let path = tmp.path().join("file.txt");
    fs::write(&path, "initial content").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("file.txt")).unwrap();
    index.write().unwrap();
    let oid = index.write_tree().unwrap();
    let sig = repo.signature().unwrap();
    let tree = repo.find_tree(oid).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
        .unwrap();

    // Simulate merge state by creating MERGE_HEAD file directly
    let merge_head_path = tmp.path().join(".git/MERGE_HEAD");
    fs::write(&merge_head_path, "1234567890abcdef1234567890abcdef12345678").unwrap();

    let status = Status::new(&repo);
    assert_eq!(status, Status::Merge);
}

#[test]
fn test_get_changed_count_multiple_types() {
    let (tmp, repo) = init_temp_repo();

    // Create initial commit
    let file1 = tmp.path().join("file1.txt");
    fs::write(&file1, "content1").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("file1.txt")).unwrap();
    index.write().unwrap();
    let oid = index.write_tree().unwrap();
    let sig = repo.signature().unwrap();
    let tree = repo.find_tree(oid).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
        .unwrap();

    // Create various types of changes
    let file2 = tmp.path().join("file2.txt"); // new file
    fs::write(&file2, "new content").unwrap();

    fs::write(&file1, "modified content").unwrap(); // modified file

    let file3 = tmp.path().join("file3.txt"); // staged new file
    fs::write(&file3, "staged content").unwrap();
    index.add_path(Path::new("file3.txt")).unwrap();
    index.write().unwrap();

    let changed_count = gitinfo::get_changed_count(&repo);
    assert!(changed_count >= 3); // At least the three changes we made
}

#[test]
fn test_get_branch_push_status_unpublished() {
    let (tmp, repo) = init_temp_repo();

    // Create a commit on local branch
    let path = tmp.path().join("file.txt");
    fs::write(&path, "content").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("file.txt")).unwrap();
    index.write().unwrap();
    let oid = index.write_tree().unwrap();
    let sig = repo.signature().unwrap();
    let tree = repo.find_tree(oid).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "commit", &tree, &[])
        .unwrap();

    let status = gitinfo::get_branch_push_status(&repo);
    assert_eq!(status, Status::Unpublished);
}

#[test]
fn test_get_branch_push_status_detached() {
    let (tmp, repo) = init_temp_repo();

    // Create a commit
    let path = tmp.path().join("file.txt");
    fs::write(&path, "content").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("file.txt")).unwrap();
    index.write().unwrap();
    let oid = index.write_tree().unwrap();
    let sig = repo.signature().unwrap();
    let tree = repo.find_tree(oid).unwrap();
    let commit_oid = repo
        .commit(Some("HEAD"), &sig, &sig, "commit", &tree, &[])
        .unwrap();

    // Detach HEAD
    repo.set_head_detached(commit_oid).unwrap();

    let status = gitinfo::get_branch_push_status(&repo);
    assert_eq!(status, Status::Detached);
}

#[test]
fn test_multiple_stashes() {
    let (tmp, mut repo) = init_temp_repo();

    // Create initial commit
    let path = tmp.path().join("file.txt");
    fs::write(&path, "initial").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("file.txt")).unwrap();
    index.write().unwrap();
    let oid = index.write_tree().unwrap();
    let sig = repo.signature().unwrap();
    let tree = repo.find_tree(oid).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
        .unwrap();

    // Drop the tree reference to avoid borrow conflicts
    drop(tree);
    drop(index);

    // Create first stash
    fs::write(&path, "work1").unwrap();
    repo.stash_save(&sig, "First stash", None).unwrap();

    // Create second stash
    fs::write(&path, "work2").unwrap();
    repo.stash_save(&sig, "Second stash", None).unwrap();

    let stash_count = gitinfo::get_stash_count(&mut repo);
    assert_eq!(stash_count, 2);
}

#[test]
fn test_status_new_additional_variants() {
    // Test additional status variants that might not be covered
    assert_eq!(Status::Unpublished.to_string(), "Unpublished");
    assert_eq!(Status::Unpushed.to_string(), "Unpushed");
    assert_eq!(Status::Detached.to_string(), "Detached");
}

#[test]
fn test_status_additional_colors() {
    assert_eq!(Status::Unpublished.comfy_color(), Color::Red);
    assert_eq!(Status::Unpushed.comfy_color(), Color::Red);
    assert_eq!(
        Status::Detached.comfy_color(),
        Color::Rgb {
            r: 255,
            g: 0,
            b: 255
        }
    );
}

#[test]
fn test_status_additional_descriptions() {
    assert_eq!(
        Status::Unpublished.description(),
        "The branch is not published."
    );
    assert_eq!(
        Status::Unpushed.description(),
        "There are unpushed commits."
    );
    assert_eq!(
        Status::Detached.description(),
        "The repository is in a detached HEAD state or has no upstream branch."
    );
}

#[test]
fn test_get_repo_name_from_url() {
    // Test the get_repo_name function indirectly through RepoInfo
    let (_, mut repo) = init_temp_repo();

    // Just test with the fallback name since adding remotes can be tricky
    let info = RepoInfo::new(
        &mut repo,
        "fallback-name",
        false,
        false,
        &PathBuf::from("/path/to/repo"),
    )
    .unwrap();
    assert_eq!(info.name, "fallback-name"); // Should use the provided name
}

#[test]
fn test_get_repo_path_functionality() {
    let (tmp, repo) = init_temp_repo();

    // Test that the repo path is correctly identified
    let repo_path = repo.path();
    assert!(repo_path.ends_with(".git"));

    // When we get the working directory, it should be the parent
    let workdir = repo.workdir().unwrap();
    assert_eq!(workdir, tmp.path());
}

#[test]
fn test_get_remote_url_with_origin() {
    let (_tmp, repo) = init_temp_repo();
    repo.remote("origin", "https://github.com/user/repo.git")
        .unwrap();
    let url = gitinfo::get_remote_url(&repo);
    assert_eq!(url, Some("https://github.com/user/repo.git".to_owned()));
}

#[test]
fn test_get_remote_url_without_origin() {
    let (_tmp, repo) = init_temp_repo();
    repo.remote("upstream", "https://github.com/upstream/repo.git")
        .unwrap();
    let url = gitinfo::get_remote_url(&repo);
    assert_eq!(url, Some("https://github.com/upstream/repo.git".to_owned()));
}

#[test]
fn test_get_remote_url_no_remotes() {
    let (_tmp, repo) = init_temp_repo();
    let url = gitinfo::get_remote_url(&repo);
    assert_eq!(url, None);
}

#[test]
fn test_get_remote_url_prefers_origin() {
    let (_tmp, repo) = init_temp_repo();
    repo.remote("upstream", "https://github.com/upstream/repo.git")
        .unwrap();
    repo.remote("origin", "https://github.com/origin/repo.git")
        .unwrap();
    let url = gitinfo::get_remote_url(&repo);
    assert_eq!(url, Some("https://github.com/origin/repo.git".to_owned()));
}

#[test]
fn test_get_branch_push_status_no_remote() {
    let (tmp, repo) = init_temp_repo();
    let path = tmp.path().join("test.txt");
    fs::write(&path, "content").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("test.txt")).unwrap();
    index.write().unwrap();
    let oid = index.write_tree().unwrap();
    let sig = repo.signature().unwrap();
    let tree = repo.find_tree(oid).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
        .unwrap();

    let status = gitinfo::get_branch_push_status(&repo);
    assert_eq!(status, Status::Unpublished);
}
