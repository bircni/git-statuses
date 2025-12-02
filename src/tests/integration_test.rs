use std::fs;
use std::path::Path;

use git2::Repository;
use tempfile::TempDir;

use crate::cli::Args;

/// Helper to create a git repository with initial commit
fn create_git_repo_with_commit(path: &Path, repo_name: &str) -> Repository {
    let repo_path = path.join(repo_name);
    fs::create_dir_all(&repo_path).unwrap();
    let repo = Repository::init(&repo_path).unwrap();

    // Configure user for commits
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test User").unwrap();
    config.set_str("user.email", "test@example.com").unwrap();

    // Create initial commit
    let file_path = repo_path.join("README.md");
    fs::write(&file_path, "# Test Repository\n").unwrap();

    let mut index = repo.index().unwrap();
    index.add_path(Path::new("README.md")).unwrap();
    index.write().unwrap();

    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = repo.signature().unwrap();

    repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
        .unwrap();

    // Drop borrowed references before returning repo
    drop(tree);
    drop(index);
    drop(config);

    repo
}

/// Helper to create a git repository with dirty working directory
fn create_dirty_repo(path: &Path, repo_name: &str) -> Repository {
    let repo = create_git_repo_with_commit(path, repo_name);
    let repo_path = path.join(repo_name);

    // Create uncommitted changes
    let file_path = repo_path.join("dirty_file.txt");
    fs::write(&file_path, "This file has uncommitted changes").unwrap();

    // Modify existing file
    let readme_path = repo_path.join("README.md");
    fs::write(&readme_path, "# Modified Test Repository\nWith changes\n").unwrap();

    repo
}

#[test]
fn test_integration_single_clean_repository() {
    let temp_dir = TempDir::new().unwrap();
    let _repo = create_git_repo_with_commit(temp_dir.path(), "test-repo");

    let args = Args {
        dir: temp_dir.path().to_path_buf(),
        depth: 1,
        ..Default::default()
    };

    let (repos, failed) = args.find_repositories();

    assert_eq!(repos.len(), 1);
    assert_eq!(failed.len(), 0);
    assert_eq!(repos[0].name, "test-repo");
    assert!(repos[0].commits > 0);
    assert!(!repos[0].branch.is_empty());
}

#[test]
fn test_integration_multiple_repositories() {
    let temp_dir = TempDir::new().unwrap();
    let _repo1 = create_git_repo_with_commit(temp_dir.path(), "repo1");
    let _repo2 = create_git_repo_with_commit(temp_dir.path(), "repo2");
    let _repo3 = create_dirty_repo(temp_dir.path(), "repo3");

    let args = Args {
        dir: temp_dir.path().to_path_buf(),
        depth: 1,
        ..Default::default()
    };

    let (repos, failed) = args.find_repositories();

    assert_eq!(repos.len(), 3);
    assert_eq!(failed.len(), 0);

    let repo_names: Vec<&str> = repos.iter().map(|r| r.name.as_str()).collect();
    assert!(repo_names.contains(&"repo1"));
    assert!(repo_names.contains(&"repo2"));
    assert!(repo_names.contains(&"repo3"));

    // Find the dirty repo and verify it's marked as dirty
    let dirty_repo = repos.iter().find(|r| r.name == "repo3").unwrap();
    assert!(matches!(
        dirty_repo.status,
        crate::gitinfo::status::Status::Dirty(_)
    ));
}

#[test]
fn test_integration_nested_repositories_with_depth() {
    let temp_dir = TempDir::new().unwrap();

    // Create nested structure: root/level1/level2/repo
    let level1_dir = temp_dir.path().join("level1");
    let level2_dir = level1_dir.join("level2");
    fs::create_dir_all(&level2_dir).unwrap();

    let _repo_root = create_git_repo_with_commit(temp_dir.path(), "root-repo");
    let _repo_level1 = create_git_repo_with_commit(&level1_dir, "level1-repo");
    let _repo_level2 = create_git_repo_with_commit(&level2_dir, "level2-repo");

    // Test depth 1 - should only find root repo
    let args_depth1 = Args {
        dir: temp_dir.path().to_path_buf(),
        depth: 1,
        ..Default::default()
    };
    let (repos_depth1, _) = args_depth1.find_repositories();
    assert_eq!(repos_depth1.len(), 1);
    assert_eq!(repos_depth1[0].name, "root-repo");

    // Test depth 3 - should find all repos
    let args_depth3 = Args {
        dir: temp_dir.path().to_path_buf(),
        depth: 3,
        ..Default::default()
    };
    let (repos_depth3, _) = args_depth3.find_repositories();
    assert_eq!(repos_depth3.len(), 3);

    let repo_names: Vec<&str> = repos_depth3.iter().map(|r| r.name.as_str()).collect();
    assert!(repo_names.contains(&"root-repo"));
    assert!(repo_names.contains(&"level1-repo"));
    assert!(repo_names.contains(&"level2-repo"));
}

#[test]
fn test_integration_subdir_functionality() {
    let temp_dir = TempDir::new().unwrap();

    // Create structure: project/checkout/.git
    let project_dir = temp_dir.path().join("project");
    let checkout_dir = project_dir.join("checkout");
    fs::create_dir_all(&checkout_dir).unwrap();

    let _repo = create_git_repo_with_commit(&checkout_dir, "test-repo");

    // Test with subdir option
    let args = Args {
        dir: temp_dir.path().to_path_buf(),
        depth: 3,
        subdir: Some("checkout".to_owned()),
        ..Default::default()
    };

    let (repos, failed) = args.find_repositories();

    assert_eq!(repos.len(), 1);
    assert_eq!(failed.len(), 0);
    assert_eq!(repos[0].name, "test-repo");
}

#[test]
fn test_integration_mixed_git_and_non_git_directories() {
    let temp_dir = TempDir::new().unwrap();

    // Create mix of git repos and regular directories
    let _repo = create_git_repo_with_commit(temp_dir.path(), "git-repo");

    let regular_dir = temp_dir.path().join("regular-dir");
    fs::create_dir_all(&regular_dir).unwrap();
    fs::write(regular_dir.join("file.txt"), "not a git repo").unwrap();

    let empty_dir = temp_dir.path().join("empty-dir");
    fs::create_dir_all(&empty_dir).unwrap();

    let args = Args {
        dir: temp_dir.path().to_path_buf(),
        depth: 1,
        ..Default::default()
    };

    let (repos, failed) = args.find_repositories();

    assert_eq!(repos.len(), 1);
    assert_eq!(failed.len(), 0);
    assert_eq!(repos[0].name, "git-repo");
}

#[test]
fn test_integration_repository_with_stashes() {
    let temp_dir = TempDir::new().unwrap();
    let mut repo = create_git_repo_with_commit(temp_dir.path(), "stash-repo");

    // Create changes and stash them
    let repo_path = temp_dir.path().join("stash-repo");
    let new_file = repo_path.join("stashed_work.txt");
    fs::write(&new_file, "Work in progress").unwrap();

    // Add to index
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("stashed_work.txt")).unwrap();
    index.write().unwrap();

    // Create stash
    let sig = repo.signature().unwrap();
    repo.stash_save(
        &sig,
        "Work in progress",
        Some(git2::StashFlags::INCLUDE_UNTRACKED),
    )
    .unwrap();

    let args = Args {
        dir: temp_dir.path().to_path_buf(),
        depth: 1,
        ..Default::default()
    };

    let (repos, failed) = args.find_repositories();

    assert_eq!(repos.len(), 1);
    assert_eq!(failed.len(), 0);
    assert_eq!(repos[0].stash_count, 1);
}

#[test]
fn test_integration_repository_with_remote() {
    let temp_dir = TempDir::new().unwrap();
    let repo = create_git_repo_with_commit(temp_dir.path(), "remote-repo");

    // Add a remote
    repo.remote("origin", "https://github.com/example/test-repo.git")
        .unwrap();

    let args = Args {
        dir: temp_dir.path().to_path_buf(),
        depth: 1,
        remote: true,
        ..Default::default()
    };

    let (repos, failed) = args.find_repositories();

    assert_eq!(repos.len(), 1);
    assert_eq!(failed.len(), 0);
    assert_eq!(
        repos[0].remote_url,
        Some("https://github.com/example/test-repo.git".to_owned())
    );
}

#[test]
fn test_integration_repository_fast_forward() {
    let remote_temp_dir = TempDir::new().unwrap();
    let local_temp_dir = TempDir::new().unwrap();

    let remote_repo_name = "remote-repo";
    let remote_repo_path = remote_temp_dir.path().join(remote_repo_name);
    let remote_url = format!("file://{}", remote_repo_path.display());

    // Create repository faking remote
    let remote_repo = create_git_repo_with_commit(remote_temp_dir.path(), remote_repo_name);

    // Create git repository, clone from remote
    let _local_repo =
        Repository::clone(&remote_url, local_temp_dir.path().join("local-repo")).unwrap();

    // Test that the clone was NOT fast-forwarded
    let args = Args {
        dir: local_temp_dir.path().to_path_buf(),
        fast_forward: true,
        ..Default::default()
    };

    let (repos, failed) = args.find_repositories();

    assert_eq!(repos.len(), 1);
    assert_eq!(failed.len(), 0);
    assert!(!repos[0].fast_forwarded);

    // Add a commit to remote
    let file_path = remote_repo_path.join("dummy.md");
    fs::write(&file_path, "# Second commit\n").unwrap();

    let mut index = remote_repo.index().unwrap();
    index.add_path(Path::new("dummy.md")).unwrap();
    index.write().unwrap();

    let tree_id = index.write_tree().unwrap();
    let tree = remote_repo.find_tree(tree_id).unwrap();
    let sig = remote_repo.signature().unwrap();
    let head = remote_repo.head().unwrap();
    let head_annotated_commit = remote_repo.reference_to_annotated_commit(&head).unwrap();
    let head_commit_id = head_annotated_commit.id();
    let head_object = remote_repo.find_object(head_commit_id, None).unwrap();
    let head_commit = head_object.into_commit().unwrap();
    remote_repo
        .commit(
            Some("HEAD"),
            &sig,
            &sig,
            "Second commit",
            &tree,
            &[&head_commit],
        )
        .unwrap();

    // Test that the clone was fast-forwarded
    let (repos, failed) = args.find_repositories();

    assert_eq!(repos.len(), 1);
    assert_eq!(failed.len(), 0);
    assert_eq!(repos[0].commits, 1);
    assert_eq!(repos[0].behind, 1);
    assert!(repos[0].fast_forwarded);

    // Test that the clone is now up to date and doesn't need fast-forward
    let (repos, failed) = args.find_repositories();

    assert_eq!(repos.len(), 1);
    assert_eq!(failed.len(), 0);
    assert_eq!(repos[0].commits, 2);
    assert_eq!(repos[0].behind, 0);
    assert!(!repos[0].fast_forwarded);
}
