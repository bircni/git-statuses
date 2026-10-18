use std::fs;
use std::path::Path;

use git2::Repository;
use tempfile::TempDir;

use crate::cli::Args;
use crate::scan;

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

/// Commits `content` as `file` on top of the repository's current HEAD.
fn commit_file(repo: &Repository, file: &str, content: &str) -> git2::Oid {
    fs::write(repo.workdir().unwrap().join(file), content).unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new(file)).unwrap();
    index.write().unwrap();
    let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
    let sig = repo.signature().unwrap();
    let parent = repo.head().unwrap().peel_to_commit().unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "change", &tree, &[&parent])
        .unwrap()
}

/// Creates a repository standing in for a remote, and a clone of it at `<temp>/scan/clone`
/// whose branch tracks the remote's. Scanning `<temp>/scan` finds only the clone.
fn remote_and_clone(temp: &Path) -> (Repository, Repository) {
    let remote = create_git_repo_with_commit(temp, "remote");
    let clone = Repository::clone(
        remote.workdir().unwrap().to_str().unwrap(),
        temp.join("scan").join("clone"),
    )
    .unwrap();
    let mut config = clone.config().unwrap();
    config.set_str("user.name", "Test User").unwrap();
    config.set_str("user.email", "test@example.com").unwrap();
    drop(config);
    (remote, clone)
}

/// Scans `<temp>/scan` the way `remote_and_clone` laid it out.
fn scan_clone(temp: &Path, fetch: bool, fast_forward: bool) -> crate::gitinfo::repoinfo::RepoInfo {
    let args = Args {
        dir: temp.join("scan"),
        depth: 1,
        fetch,
        fast_forward,
        ..Default::default()
    };
    let (mut repos, failed) = scan::find_repositories(&args);
    assert_eq!(failed, Vec::<String>::new());
    assert_eq!(repos.len(), 1);
    repos.remove(0)
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

    let (repos, failed) = scan::find_repositories(&args);

    assert_eq!(repos.len(), 1);
    assert_eq!(failed.len(), 0);
    assert_eq!(repos[0].name, "test-repo");
    assert!(repos[0].commits > 0);
    assert_ne!(repos[0].branch, "");
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

    let (repos, failed) = scan::find_repositories(&args);

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

    let (repos, failed) = scan::find_repositories(&args);

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
        let (again, failed_again) = scan::find_repositories(&args);
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

    let (repos, failed) = scan::find_repositories(&args);

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

    let (repos, failed) = scan::find_repositories(&args);

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

    let (repos, failed) = scan::find_repositories(&args);

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

    let (repos, failed) = scan::find_repositories(&args);

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
    let (repos, failed) = scan::find_repositories(&args);

    assert_eq!(repos.len(), 1);
    assert_eq!(failed.len(), 0);
    assert!(repos[0].fast_forwarded);
    assert!(
        local_temp_dir.path().join("local-repo/dummy.md").exists(),
        "the fast-forward must check the new commit out, not only move the branch"
    );
    assert_ne!(
        repos[0].status,
        crate::gitinfo::status::Status::Dirty(1),
        "the index and working tree must match the new commit"
    );
    assert_eq!(
        repos[0].commits, 2,
        "commit count must be gathered after the fast-forward"
    );
    assert_eq!(
        repos[0].behind, 0,
        "a fast-forwarded repository is no longer behind its upstream"
    );

    // Test that the clone is now up to date and doesn't need fast-forward
    let (repos, failed) = scan::find_repositories(&args);

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

    let (repos, failed) = scan::find_repositories(&args);

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

    let (repos, failed) = scan::find_repositories(&args);

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
    // Whatever `init.defaultBranch` the machine running the tests has configured.
    let default_branch = head.shorthand().unwrap().to_owned();
    let target = head.target().unwrap();
    let commit = repo.find_commit(target).unwrap();
    repo.branch("feature-branch", &commit, false).unwrap();
    drop(commit);
    drop(head);

    // Check the feature branch out in a worktree. git2 does this itself, so the user's
    // global git config (hooks and the like) never runs during the test.
    let worktree_path = temp_dir.path().join("feature-worktree");
    let feature = repo
        .find_branch("feature-branch", git2::BranchType::Local)
        .unwrap();
    let mut opts = git2::WorktreeAddOptions::new();
    opts.reference(Some(feature.get()));
    repo.worktree("feature-worktree", &worktree_path, Some(&opts))
        .unwrap();

    // Now scan the temp directory for repositories
    let args = Args {
        dir: temp_dir.path().to_path_buf(),
        depth: 2,
        ..Default::default()
    };

    let (repos, failed) = scan::find_repositories(&args);

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
    assert_eq!(main_repo.branch, default_branch);

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
    // Without a reference, git2 checks out a new branch named after the worktree.
    let worktree_path = temp_dir.path().join("worktree");
    repo.worktree("new-branch", &worktree_path, None).unwrap();

    // Make changes in the worktree
    let worktree_file = worktree_path.join("new-file.txt");
    fs::write(&worktree_file, "worktree changes\n").unwrap();

    // Scan repositories
    let args = Args {
        dir: temp_dir.path().to_path_buf(),
        depth: 2,
        ..Default::default()
    };

    let (repos, _failed) = scan::find_repositories(&args);

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
    let (repos, failed) = scan::find_repositories(&scanned_directly);

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
    let (from_parent, _) = scan::find_repositories(&scanned_from_parent);
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
        let (repos, _) = scan::find_repositories(&args);
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

/// The walker must not descend into a repository's `.git` directory. Nothing in there is a
/// repository, and the guard that used to keep the `worktrees/<name>` metadata out of the
/// results matched the hardcoded substring `/.git/worktrees/`, which cannot match on
/// Windows, where the separator is a backslash.
#[test]
fn test_integration_unlimited_depth_ignores_git_internals() {
    let temp_dir = TempDir::new().unwrap();

    let repo = create_git_repo_with_commit(temp_dir.path(), "main-repo");

    // Adding a worktree populates .git/worktrees/<name> with a HEAD, commondir, gitdir...
    let worktree_path = temp_dir.path().join("feature-worktree");
    repo.worktree("feature", &worktree_path, None).unwrap();

    // An unlimited scan walks everything below the root, git directories included.
    let args = Args {
        dir: temp_dir.path().to_path_buf(),
        depth: -1,
        ..Default::default()
    };
    let (repos, failed) = scan::find_repositories(&args);

    assert_eq!(failed, Vec::<String>::new());
    assert_eq!(
        repos.len(),
        2,
        "expected only the main repo and its worktree, got: {:?}",
        repos.iter().map(|r| &r.repo_path).collect::<Vec<_>>()
    );
    for repo in &repos {
        assert!(
            !repo.path.components().any(|c| c.as_os_str() == ".git"),
            "no result may point inside a git directory, got: {}",
            repo.path.display()
        );
    }
}

/// The `--subdir` flag exists for layouts where the checkout lives one level below the
/// project directory (`repo-name/checkout`). The repository must be picked up through the
/// subdirectory, at a depth where it would not be found otherwise.
#[test]
fn test_integration_subdir_finds_repo_below_the_scanned_level() {
    let temp_dir = TempDir::new().unwrap();

    // temp/project/checkout/.git - `project` itself is not a repository.
    let project_dir = temp_dir.path().join("project");
    fs::create_dir_all(&project_dir).unwrap();
    create_git_repo_with_commit(&project_dir, "checkout");

    // Depth 1 only reaches `project`, so without --subdir there is nothing to find.
    let without_subdir = Args {
        dir: temp_dir.path().to_path_buf(),
        depth: 1,
        ..Default::default()
    };
    let (repos, failed) = scan::find_repositories(&without_subdir);
    assert_eq!(
        repos.len(),
        0,
        "the checkout is a level below the scan depth"
    );
    assert_eq!(failed.len(), 0);

    // With --subdir, `project/checkout` is inspected and found.
    let with_subdir = Args {
        dir: temp_dir.path().to_path_buf(),
        depth: 1,
        subdir: Some("checkout".to_owned()),
        ..Default::default()
    };
    let (repos, failed) = scan::find_repositories(&with_subdir);
    assert_eq!(failed.len(), 0);
    assert_eq!(repos.len(), 1, "--subdir must find the nested checkout");
    assert_eq!(
        repos[0].repo_path,
        Path::new("project").join("checkout").display().to_string()
    );

    // A subdir that does not exist anywhere must simply yield nothing.
    let missing_subdir = Args {
        dir: temp_dir.path().to_path_buf(),
        depth: 1,
        subdir: Some("does-not-exist".to_owned()),
        ..Default::default()
    };
    let (repos, failed) = scan::find_repositories(&missing_subdir);
    assert_eq!(repos.len(), 0);
    assert_eq!(failed.len(), 0);
}

/// A clone that is in sync with its remote is Clean; committing locally without pushing
/// turns it into Unpushed and marks it as having unpushed work.
#[test]
fn test_integration_clean_and_unpushed_against_a_remote() {
    let remote_dir = TempDir::new().unwrap();
    let local_dir = TempDir::new().unwrap();

    let remote_path = remote_dir.path().join("origin-repo");
    create_git_repo_with_commit(remote_dir.path(), "origin-repo");

    let clone_path = local_dir.path().join("clone");
    let clone = Repository::clone(&remote_path.to_string_lossy(), &clone_path).unwrap();
    let mut config = clone.config().unwrap();
    config.set_str("user.name", "Test User").unwrap();
    config.set_str("user.email", "test@example.com").unwrap();
    drop(config);

    let args = Args {
        dir: local_dir.path().to_path_buf(),
        depth: 1,
        ..Default::default()
    };

    // A fresh clone tracks its remote and matches it exactly.
    let (repos, _) = scan::find_repositories(&args);
    assert_eq!(repos.len(), 1);
    assert_eq!(repos[0].status, crate::gitinfo::status::Status::Clean);
    assert!(!repos[0].has_unpushed);
    assert!(!repos[0].is_local_only, "a clone has an upstream");
    assert_eq!(repos[0].ahead, 0);

    // Commit locally without pushing.
    fs::write(clone_path.join("local.txt"), "local work").unwrap();
    let mut index = clone.index().unwrap();
    index.add_path(Path::new("local.txt")).unwrap();
    index.write().unwrap();
    let tree = clone.find_tree(index.write_tree().unwrap()).unwrap();
    let sig = clone.signature().unwrap();
    let parent = clone.head().unwrap().peel_to_commit().unwrap();
    clone
        .commit(Some("HEAD"), &sig, &sig, "local commit", &tree, &[&parent])
        .unwrap();

    let (repos, _) = scan::find_repositories(&args);
    assert_eq!(repos.len(), 1);
    assert_eq!(
        repos[0].status,
        crate::gitinfo::status::Status::Unpushed,
        "a committed but unpushed change must be reported as Unpushed"
    );
    assert!(repos[0].has_unpushed);
    assert_eq!(repos[0].ahead, 1);
    assert_eq!(repos[0].behind, 0);
}

/// A repository that opens but cannot be read must be reported as failed rather than
/// silently dropped from the scan. Removing the object database leaves the refs in place,
/// so the repository opens and HEAD resolves, but the commit history cannot be walked.
#[test]
fn test_repository_with_unreadable_history_is_reported_as_failed() {
    let temp = TempDir::new().unwrap();
    create_git_repo_with_commit(temp.path(), "healthy");
    let broken = temp.path().join("no-objects");
    create_git_repo_with_commit(temp.path(), "no-objects");

    // Drop the object database but keep the refs pointing into it.
    fs::remove_dir_all(broken.join(".git/objects")).unwrap();

    let args = Args {
        dir: temp.path().to_path_buf(),
        depth: 2,
        ..Default::default()
    };
    let (repos, failed) = scan::find_repositories(&args);

    assert!(
        repos.iter().any(|r| r.repo_path == "healthy"),
        "the readable repository must still be reported, got: {:?}",
        repos.iter().map(|r| &r.repo_path).collect::<Vec<_>>()
    );
    assert!(
        failed.iter().any(|f| f == "no-objects"),
        "the unreadable repository must be listed as failed, got: {failed:?}"
    );
}

/// A `--subdir` checkout that cannot be opened is reported under the scanned directory's
/// name. Reporting the opened path's name listed every broken project as `checkout`.
#[test]
fn test_integration_unreadable_subdir_is_named_after_the_project() {
    let temp_dir = TempDir::new().unwrap();
    let checkout = temp_dir.path().join("project").join("checkout");
    fs::create_dir_all(&checkout).unwrap();
    fs::write(checkout.join(".git"), "gitdir: /nowhere").unwrap();

    let args = Args {
        dir: temp_dir.path().to_path_buf(),
        depth: 1,
        subdir: Some("checkout".to_owned()),
        ..Default::default()
    };
    let (repos, failed) = scan::find_repositories(&args);

    assert!(repos.is_empty());
    assert_eq!(failed, ["project"]);
}

/// `--fetch` alone updates the remote-tracking branch but not the local one, so the
/// repository is reported as behind and nothing is fast-forwarded.
#[test]
fn test_integration_fetch_reports_commits_behind() {
    let temp = TempDir::new().unwrap();
    let (remote, _clone) = remote_and_clone(temp.path());
    commit_file(&remote, "upstream.txt", "new upstream work");

    let info = scan_clone(temp.path(), true, false);

    assert_eq!((info.ahead, info.behind), (0, 1));
    assert!(!info.fast_forwarded);
    assert_eq!(info.status, crate::gitinfo::status::Status::Clean);
}

/// A branch that diverged from its upstream cannot be fast-forwarded. `--ff` must leave it
/// where it is and report both sides.
#[test]
fn test_integration_fast_forward_leaves_a_diverged_branch_alone() {
    let temp = TempDir::new().unwrap();
    let (remote, clone) = remote_and_clone(temp.path());
    commit_file(&remote, "upstream.txt", "upstream");
    let local_head = commit_file(&clone, "local.txt", "local");

    let info = scan_clone(temp.path(), false, true);

    assert!(!info.fast_forwarded);
    assert_eq!((info.ahead, info.behind), (1, 1));
    let clone = Repository::open(clone.workdir().unwrap()).unwrap();
    assert_eq!(clone.head().unwrap().target(), Some(local_head));
}

/// An uncommitted change to a file the upstream changed too would be overwritten by the
/// fast-forward. The checkout must refuse, keeping both the branch and the change.
#[test]
fn test_integration_fast_forward_keeps_conflicting_local_changes() {
    let temp = TempDir::new().unwrap();
    let (remote, clone) = remote_and_clone(temp.path());
    let before = clone.head().unwrap().target();
    commit_file(&remote, "README.md", "# Changed upstream\n");
    let readme = clone.workdir().unwrap().join("README.md");
    fs::write(&readme, "# Changed locally\n").unwrap();

    let info = scan_clone(temp.path(), false, true);

    assert!(!info.fast_forwarded);
    assert_eq!(info.behind, 1);
    let clone = Repository::open(clone.workdir().unwrap()).unwrap();
    assert_eq!(clone.head().unwrap().target(), before, "HEAD must not move");
    assert_eq!(fs::read_to_string(&readme).unwrap(), "# Changed locally\n");
}

/// A detached HEAD is not on a branch, so there is nothing to fast-forward.
#[test]
fn test_integration_fast_forward_skips_a_detached_head() {
    let temp = TempDir::new().unwrap();
    let (remote, clone) = remote_and_clone(temp.path());
    let before = clone.head().unwrap().target().unwrap();
    clone.set_head_detached(before).unwrap();
    commit_file(&remote, "upstream.txt", "upstream");

    let info = scan_clone(temp.path(), false, true);

    assert!(!info.fast_forwarded);
    assert!(info.is_detached);
    let clone = Repository::open(clone.workdir().unwrap()).unwrap();
    assert_eq!(clone.head().unwrap().target(), Some(before));
}

/// A worktree's git directory lives inside the main repository's, so the fetch has to run
/// in the worktree's working directory to reach the remote at all.
#[test]
fn test_integration_fetch_from_a_worktree() {
    let temp = TempDir::new().unwrap();
    let (remote, clone) = remote_and_clone(temp.path());
    let branch = clone.head().unwrap().shorthand().unwrap().to_owned();
    let worktree_path = temp.path().join("worktree");
    clone.worktree("wt", &worktree_path, None).unwrap();
    let upstream = commit_file(&remote, "upstream.txt", "upstream");

    let worktree = Repository::open(&worktree_path).unwrap();
    crate::gitinfo::fetch_remote(&worktree).unwrap();

    let tracking = worktree
        .find_reference(&format!("refs/remotes/origin/{branch}"))
        .unwrap();
    assert_eq!(tracking.target(), Some(upstream));
}

/// An unlimited scan keeps descending into a repository's working directory, so a
/// repository nested inside another one is reported as well.
#[test]
fn test_integration_unlimited_depth_finds_nested_repositories() {
    let temp_dir = TempDir::new().unwrap();
    let outer = create_git_repo_with_commit(temp_dir.path(), "outer");
    create_git_repo_with_commit(outer.workdir().unwrap(), "inner");

    let args = Args {
        dir: temp_dir.path().to_path_buf(),
        depth: -1,
        ..Default::default()
    };
    let (repos, failed) = scan::find_repositories(&args);

    assert_eq!(failed, Vec::<String>::new());
    let paths: Vec<&str> = repos.iter().map(|r| r.repo_path.as_str()).collect();
    let inner = Path::new("outer").join("inner").display().to_string();
    assert_eq!(paths, ["outer", inner.as_str()]);
}

/// The walk does not follow symlinks, but a link to a repository is still a directory
/// with a `.git` in it. It is shown under the link's name: git resolves the link when it
/// opens the repository, which used to put the absolute target path in the table.
#[cfg(unix)]
#[test]
fn test_integration_symlinked_repository_is_shown_under_the_link_name() {
    let target_dir = TempDir::new().unwrap();
    create_git_repo_with_commit(target_dir.path(), "real");
    let scan_dir = TempDir::new().unwrap();
    std::os::unix::fs::symlink(target_dir.path().join("real"), scan_dir.path().join("link"))
        .unwrap();

    let args = Args {
        dir: scan_dir.path().to_path_buf(),
        depth: 1,
        ..Default::default()
    };
    let (repos, failed) = scan::find_repositories(&args);

    assert_eq!(failed, Vec::<String>::new());
    assert_eq!(repos.len(), 1);
    assert_eq!(repos[0].repo_path, "link");
}
