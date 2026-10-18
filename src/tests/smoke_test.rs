#![allow(
    clippy::too_many_lines,
    reason = "Comprehensive test covering multiple scenarios"
)]

use std::{fs, path::Path, time::Instant};

use git2::{Repository, RepositoryInitOptions};

use crate::scan;
use crate::{cli::Args, gitinfo::status::Status};

/// Helper function to create a repository with a commit
fn create_repo_with_commit(path: &Path, file_content: &str) -> Repository {
    // Setting `init.defaultBranch` after `init` has no effect, so name the branch up front:
    // the remote-tracking refs the tests create assume it is `main`.
    let repo =
        Repository::init_opts(path, RepositoryInitOptions::new().initial_head("main")).unwrap();
    {
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
    }

    let file_path = path.join("test.txt");
    fs::write(&file_path, file_content).unwrap();

    {
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("test.txt")).unwrap();
        index.write().unwrap();
        let oid = index.write_tree().unwrap();
        let sig = repo.signature().unwrap();
        let tree = repo.find_tree(oid).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();
    }

    repo
}

/// Helper function to add a remote to a repository
fn add_remote(repo: &Repository, remote_name: &str, url: &str) {
    repo.remote(remote_name, url).unwrap();
}

/// Helper function to create a remote tracking branch
fn create_remote_branch(
    repo: &Repository,
    remote_name: &str,
    branch_name: &str,
    commit_oid: git2::Oid,
) {
    repo.reference(
        &format!("refs/remotes/{remote_name}/{branch_name}"),
        commit_oid,
        false,
        "create remote tracking branch",
    )
    .unwrap();
}

/// Helper function to make an additional commit
fn make_commit(repo: &Repository, file_path: &Path, content: &str, message: &str) -> git2::Oid {
    fs::write(file_path, content).unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("test.txt")).unwrap();
    index.write().unwrap();
    let oid = index.write_tree().unwrap();
    let sig = repo.signature().unwrap();
    let tree = repo.find_tree(oid).unwrap();
    let parent = repo.head().unwrap().peel_to_commit().unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&parent])
        .unwrap()
}

#[test]
fn test_multiple_repos_with_different_remotes_and_statuses() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let base_path = tmp_dir.path();

    let test_cases: Vec<String> = (0..40).map(|i| format!("repo_{i}")).collect();

    // Create all repositories
    for test in &test_cases {
        let repo_path = base_path.join(test);
        fs::create_dir_all(&repo_path).unwrap();
        let repo = create_repo_with_commit(&repo_path, "initial content");
        let initial_commit = repo.head().unwrap().target().unwrap();
        let remote_name = "origin";
        let remote_url = format!("https://github.com/user/{test}.git");
        add_remote(&repo, remote_name, &remote_url);
        create_remote_branch(&repo, remote_name, "main", initial_commit);
    }

    // Measure time to check all repositories
    let start = Instant::now();

    let args = Args {
        dir: base_path.to_path_buf(),
        ..Default::default()
    };
    let (repos, failed) = scan::find_repositories(&args);

    let duration = start.elapsed();

    assert_eq!(failed, Vec::<String>::new());
    assert_eq!(repos.len(), test_cases.len());
    assert!(
        repos.iter().all(|r| r.status == Status::Clean),
        "every repository matches its remote-tracking branch"
    );

    // A generous ceiling on purpose: this guards against a pathological regression (a
    // per-repository network call, an accidental O(n^2) walk), not against a slow or
    // loaded CI runner. A tight bound here only buys flaky failures.
    assert!(
        duration.as_secs() < 60,
        "Status check took too long: {duration:?}"
    );

    println!("✓ All 40 repositories checked correctly in {duration:?}");
}

#[test]
fn test_concurrent_status_checks_performance() {
    use rayon::prelude::*;

    let tmp_dir = tempfile::tempdir().unwrap();
    let base_path = tmp_dir.path();

    // Create 20 repositories, each with the status its configuration must produce
    let repo_count = 20;
    let mut repo_paths = Vec::new();
    let mut expected = Vec::new();

    for i in 0..repo_count {
        let repo_path = base_path.join(format!("repo_{i:02}"));
        fs::create_dir_all(&repo_path).unwrap();
        let repo = create_repo_with_commit(&repo_path, &format!("content {i}"));
        let initial_commit = repo.head().unwrap().target().unwrap();

        // Mix of different configurations
        if i % 3 == 0 {
            add_remote(
                &repo,
                "origin",
                &format!("https://github.com/user/repo_{i}.git"),
            );
            create_remote_branch(&repo, "origin", "main", initial_commit);
            expected.push(Status::Clean);
        } else if i % 3 == 1 {
            add_remote(
                &repo,
                "upstream",
                &format!("https://github.com/upstream/repo_{i}.git"),
            );
            create_remote_branch(&repo, "upstream", "main", initial_commit);
            make_commit(&repo, &repo_path.join("test.txt"), "new", "Second");
            expected.push(Status::Unpushed);
        } else {
            expected.push(Status::Unpublished);
        }

        repo_paths.push(repo_path);
    }

    // Sequential check
    let start_seq = Instant::now();
    let sequential: Vec<Status> = repo_paths
        .iter()
        .map(|path| Status::new(&Repository::open(path).unwrap()))
        .collect();
    let duration_seq = start_seq.elapsed();

    // Parallel check (simulating real-world usage with rayon)
    let start_par = Instant::now();
    let parallel: Vec<Status> = repo_paths
        .par_iter()
        .map(|path| Status::new(&Repository::open(path).unwrap()))
        .collect();
    let duration_par = start_par.elapsed();

    assert_eq!(sequential, expected);
    assert_eq!(
        parallel, expected,
        "checking in parallel must not change a result"
    );

    println!("Sequential: {duration_seq:?}, Parallel: {duration_par:?}");

    // Deliberately generous - see the note in the 40-repository test above. No speedup
    // ratio is asserted: on a busy or single-core runner parallel is legitimately not
    // faster, so that would be a flake rather than a regression signal.
    assert!(
        duration_seq.as_secs() < 60,
        "Sequential status checks took too long: {duration_seq:?}"
    );
    assert!(
        duration_par.as_secs() < 60,
        "Parallel status checks took too long: {duration_par:?}"
    );
}
