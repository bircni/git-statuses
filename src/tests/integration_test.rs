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

/// Repositories are collected in parallel with rayon, so the raw collection order depends
/// on thread scheduling. `find_repositories` must hand back a sorted, reproducible order,
/// otherwise consumers that do not sort themselves (`--json`, the failure warnings) emit a
/// different order on every run.
#[test]
fn test_integration_find_repositories_returns_sorted_results() {
    let temp_dir = TempDir::new().unwrap();

    for name in ["delta", "alpha", "Charlie", "bravo"] {
        create_git_repo_with_commit(temp_dir.path(), name);
    }
    // Directories with a `.git` that git cannot open end up in the failed list.
    for name in ["zeta-broken", "echo-broken"] {
        let broken = temp_dir.path().join(name);
        fs::create_dir_all(&broken).unwrap();
        fs::write(broken.join(".git"), "not a git directory").unwrap();
    }

    let args = Args {
        dir: temp_dir.path().to_path_buf(),
        depth: 1,
        ..Default::default()
    };

    let (repos, failed) = args.find_repositories();

    let paths: Vec<&str> = repos.iter().map(|r| r.repo_path.as_str()).collect();
    assert_eq!(
        paths,
        ["alpha", "bravo", "Charlie", "delta"],
        "repositories must be sorted case-insensitively"
    );
    assert_eq!(
        failed,
        ["echo-broken", "zeta-broken"],
        "failed repositories must be sorted"
    );

    // The order must not depend on the parallel scheduling of a particular run.
    for _ in 0..5 {
        let (again, failed_again) = args.find_repositories();
        assert_eq!(
            again.iter().map(|r| &r.repo_path).collect::<Vec<_>>(),
            repos.iter().map(|r| &r.repo_path).collect::<Vec<_>>(),
            "repository order must be stable across scans"
        );
        assert_eq!(
            failed_again, failed,
            "failed repository order must be stable across scans"
        );
    }
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
    let remote_path = remote_repo_path.to_string_lossy().to_string();

    // Create repository faking remote
    let remote_repo = create_git_repo_with_commit(remote_temp_dir.path(), remote_repo_name);

    // Create git repository, clone from remote
    let _local_repo =
        Repository::clone(&remote_path, local_temp_dir.path().join("local-repo")).unwrap();

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

    // Test that the clone was fast-forwarded, and that the reported state describes the
    // repository *after* the merge rather than before it.
    let (repos, failed) = args.find_repositories();

    assert_eq!(repos.len(), 1);
    assert_eq!(failed.len(), 0);
    assert!(repos[0].fast_forwarded);
    assert_eq!(
        repos[0].commits, 2,
        "commit count must be gathered after the fast-forward"
    );
    assert_eq!(
        repos[0].behind, 0,
        "a fast-forwarded repository is no longer behind its upstream"
    );

    // Test that the clone is now up to date and doesn't need fast-forward
    let (repos, failed) = args.find_repositories();

    assert_eq!(repos.len(), 1);
    assert_eq!(failed.len(), 0);
    assert_eq!(repos[0].commits, 2);
    assert_eq!(repos[0].behind, 0);
    assert!(!repos[0].fast_forwarded);
}

/// A repository without any remote cannot be fetched from. It must still be reported
/// instead of being swallowed into the list of failed repositories.
#[test]
fn test_integration_fetch_on_repo_without_remote_is_not_a_failure() {
    let temp_dir = TempDir::new().unwrap();
    let _repo = create_git_repo_with_commit(temp_dir.path(), "local-only");

    let args = Args {
        dir: temp_dir.path().to_path_buf(),
        depth: 1,
        fetch: true,
        ..Default::default()
    };

    let (repos, failed) = args.find_repositories();

    assert_eq!(
        failed,
        Vec::<String>::new(),
        "repo must not be reported as failed"
    );
    assert_eq!(repos.len(), 1, "repo must still be listed");
    assert_eq!(repos[0].name, "local-only");
    assert!(!repos[0].fast_forwarded);
}

/// A branch without an upstream cannot be fast-forwarded. `--ff` must degrade to "no
/// fast-forward happened" rather than dropping the repository from the output.
#[test]
fn test_integration_fast_forward_without_upstream_is_not_a_failure() {
    let temp_dir = TempDir::new().unwrap();
    let _repo = create_git_repo_with_commit(temp_dir.path(), "no-upstream");

    let args = Args {
        dir: temp_dir.path().to_path_buf(),
        depth: 1,
        fetch: true,
        fast_forward: true,
        ..Default::default()
    };

    let (repos, failed) = args.find_repositories();

    assert_eq!(
        failed,
        Vec::<String>::new(),
        "repo must not be reported as failed"
    );
    assert_eq!(repos.len(), 1, "repo must still be listed");
    assert!(!repos[0].fast_forwarded);
    assert!(repos[0].is_local_only);
}

#[test]
fn test_integration_worktree_detection() {
    let temp_dir = TempDir::new().unwrap();

    // Create a main repository with an initial commit
    let main_repo_path = temp_dir.path().join("main-repo");
    fs::create_dir_all(&main_repo_path).unwrap();
    let repo = Repository::init(&main_repo_path).unwrap();

    // Configure user for commits
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test User").unwrap();
    config.set_str("user.email", "test@example.com").unwrap();
    drop(config);

    // Create initial commit on main branch
    let file_path = main_repo_path.join("README.md");
    fs::write(&file_path, "# Main Repository\n").unwrap();

    let mut index = repo.index().unwrap();
    index.add_path(Path::new("README.md")).unwrap();
    index.write().unwrap();

    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = repo.signature().unwrap();

    repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
        .unwrap();

    drop(tree);
    drop(index);

    // Create a feature branch
    let head = repo.head().unwrap();
    let target = head.target().unwrap();
    let commit = repo.find_commit(target).unwrap();
    repo.branch("feature-branch", &commit, false).unwrap();
    drop(commit);
    drop(head);

    // Create a worktree using git command (git2-rs doesn't have worktree creation API)
    let worktree_path = temp_dir.path().join("feature-worktree");
    let output = std::process::Command::new("git")
        .arg("worktree")
        .arg("add")
        .arg(&worktree_path)
        .arg("feature-branch")
        .current_dir(&main_repo_path)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "Failed to create worktree: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Now scan the temp directory for repositories
    let args = Args {
        dir: temp_dir.path().to_path_buf(),
        depth: 2,
        ..Default::default()
    };

    let (repos, failed) = args.find_repositories();

    // We should find exactly 2 repositories: main repo and worktree
    assert_eq!(failed.len(), 0, "Failed repos: {failed:?}");
    assert_eq!(
        repos.len(),
        2,
        "Expected 2 repos (main + worktree), found {}: {:?}",
        repos.len(),
        repos.iter().map(|r| &r.repo_path).collect::<Vec<_>>()
    );

    // Find the main repo and worktree
    let main_repo = repos
        .iter()
        .find(|r| r.repo_path.contains("main-repo"))
        .unwrap();
    let worktree = repos
        .iter()
        .find(|r| r.repo_path.contains("feature-worktree"))
        .unwrap();

    // Verify main repo is NOT a worktree
    assert!(
        !main_repo.is_worktree,
        "Main repo should not be marked as worktree"
    );
    // Git might use "main" or "master" depending on configuration
    assert!(
        main_repo.branch == "main" || main_repo.branch == "master",
        "Main repo branch should be 'main' or 'master', got: {}",
        main_repo.branch
    );

    // Verify worktree IS detected as a worktree
    assert!(
        worktree.is_worktree,
        "Worktree should be marked as worktree"
    );
    assert_eq!(worktree.branch, "feature-branch");

    // Verify that the worktree path doesn't contain .git/worktrees
    assert!(
        !worktree.repo_path.contains(".git/worktrees"),
        "Worktree path should not contain .git/worktrees, got: {}",
        worktree.repo_path
    );

    // Verify both have the same commit count (since they're from the same repo)
    assert_eq!(main_repo.commits, worktree.commits);
}

#[test]
fn test_integration_worktree_with_changes() {
    let temp_dir = TempDir::new().unwrap();

    // Create a main repository
    let main_repo_path = temp_dir.path().join("main-repo");
    fs::create_dir_all(&main_repo_path).unwrap();
    let repo = Repository::init(&main_repo_path).unwrap();

    // Configure user
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test User").unwrap();
    config.set_str("user.email", "test@example.com").unwrap();
    drop(config);

    // Create initial commit
    let file_path = main_repo_path.join("file.txt");
    fs::write(&file_path, "content\n").unwrap();

    let mut index = repo.index().unwrap();
    index.add_path(Path::new("file.txt")).unwrap();
    index.write().unwrap();

    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = repo.signature().unwrap();

    repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
        .unwrap();

    drop(tree);
    drop(index);

    // Create worktree
    let worktree_path = temp_dir.path().join("worktree");
    let output = std::process::Command::new("git")
        .arg("worktree")
        .arg("add")
        .arg(&worktree_path)
        .arg("-b")
        .arg("new-branch")
        .current_dir(&main_repo_path)
        .output()
        .unwrap();

    assert!(output.status.success());

    // Make changes in the worktree
    let worktree_file = worktree_path.join("new-file.txt");
    fs::write(&worktree_file, "worktree changes\n").unwrap();

    // Scan repositories
    let args = Args {
        dir: temp_dir.path().to_path_buf(),
        depth: 2,
        ..Default::default()
    };

    let (repos, _failed) = args.find_repositories();

    // Find worktree
    let worktree = repos.iter().find(|r| r.is_worktree).unwrap();

    // Verify worktree has dirty status due to uncommitted changes
    assert!(
        worktree.status.to_string().contains("Dirty"),
        "Worktree should be dirty, got status: {}",
        worktree.status
    );
}

/// Scanning a repository directly (the common `git-statuses` with no argument, run from
/// inside a checkout) leaves the path relative to the scan root empty. It used to fall back
/// to the absolute path, so the Directory column showed `/home/user/code/my-repo` here but
/// a bare `my-repo` for the very same repository when scanning its parent.
#[test]
fn test_integration_scanning_a_repository_directly_shows_its_name() {
    let temp_dir = TempDir::new().unwrap();
    let _repo = create_git_repo_with_commit(temp_dir.path(), "my-repo");
    let repo_dir = temp_dir.path().join("my-repo");

    let scanned_directly = Args {
        dir: repo_dir.clone(),
        depth: 1,
        ..Default::default()
    };
    let (repos, failed) = scanned_directly.find_repositories();

    assert_eq!(failed.len(), 0);
    assert_eq!(repos.len(), 1);
    assert_eq!(
        repos[0].repo_path, "my-repo",
        "a directly scanned repository must be shown by name, not by absolute path"
    );

    // Scanning the parent must produce the same label for the same repository.
    let scanned_from_parent = Args {
        dir: temp_dir.path().to_path_buf(),
        depth: 1,
        ..Default::default()
    };
    let (from_parent, _) = scanned_from_parent.find_repositories();
    assert_eq!(from_parent[0].repo_path, repos[0].repo_path);

    // The absolute location is still available in the dedicated path field (`--path`).
    assert_eq!(
        repos[0].path.canonicalize().unwrap(),
        repo_dir.canonicalize().unwrap()
    );
}

/// `-1` is the documented spelling of "no limit", but the guard let *any* negative value
/// through to the same unlimited branch. Rather than have `--depth -5` mean something
/// different from what the help text says, every negative depth is unlimited, and a depth
/// of 0 (which would otherwise find nothing) behaves like 1.
#[test]
fn test_integration_depth_edge_values() {
    let temp_dir = TempDir::new().unwrap();

    let level1 = temp_dir.path().join("level1");
    let level2 = level1.join("level2");
    fs::create_dir_all(&level2).unwrap();

    create_git_repo_with_commit(temp_dir.path(), "root-repo");
    create_git_repo_with_commit(&level1, "level1-repo");
    create_git_repo_with_commit(&level2, "level2-repo");

    let scan = |depth: i32| {
        let args = Args {
            dir: temp_dir.path().to_path_buf(),
            depth,
            ..Default::default()
        };
        let (repos, _) = args.find_repositories();
        repos.len()
    };

    // Depth 0 finds nothing without the clamp; it must behave like depth 1.
    assert_eq!(scan(0), 1, "depth 0 must behave like depth 1");
    assert_eq!(scan(1), 1);
    assert_eq!(scan(3), 3);

    // Every negative depth is unlimited, not just -1.
    assert_eq!(scan(-1), 3, "-1 must scan all subdirectories");
    assert_eq!(
        scan(-5),
        3,
        "any negative depth must scan all subdirectories"
    );
    assert_eq!(
        scan(i32::MIN),
        3,
        "i32::MIN must not overflow the depth cast"
    );
}
