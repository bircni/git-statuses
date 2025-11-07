#![allow(
    clippy::too_many_lines,
    reason = "Comprehensive test covering multiple scenarios"
)]

use std::{fs, path::Path, time::Instant};

use git2::Repository;

use crate::{
    cli::Args,
    gitinfo::{self, status::Status},
};

/// Helper function to create a repository with a commit
fn create_repo_with_commit(path: &Path, file_content: &str) -> Repository {
    let repo = Repository::init(path).unwrap();
    {
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
        config.set_str("init.defaultBranch", "main").unwrap();
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
    _ = args.find_repositories();

    let duration = start.elapsed();

    // Performance assertion: should complete in reasonable time
    // 40 repositories should be checked in less than 2 seconds
    assert!(
        duration.as_secs() < 2,
        "Status check took too long: {duration:?}"
    );

    println!("✓ All 40 repositories checked correctly in {duration:?}");
}

struct RemoteTest {
    name: &'static str,
    remote_name: &'static str,
    remote_url: &'static str,
    should_prefer_origin: bool,
}

#[test]
fn test_remote_url_extraction_from_various_sources() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let base_path = tmp_dir.path();

    let test_cases = vec![
        RemoteTest {
            name: "origin_github",
            remote_name: "origin",
            remote_url: "https://github.com/user/repo.git",
            should_prefer_origin: true,
        },
        RemoteTest {
            name: "upstream_gitlab",
            remote_name: "upstream",
            remote_url: "https://gitlab.com/user/repo.git",
            should_prefer_origin: false,
        },
        RemoteTest {
            name: "custom_bitbucket",
            remote_name: "custom",
            remote_url: "https://bitbucket.org/user/repo.git",
            should_prefer_origin: false,
        },
    ];

    for test in &test_cases {
        let repo_path = base_path.join(test.name);
        fs::create_dir_all(&repo_path).unwrap();
        let repo = create_repo_with_commit(&repo_path, "content");

        if test.should_prefer_origin {
            add_remote(&repo, "upstream", "https://github.com/upstream/repo.git");
        }
        add_remote(&repo, test.remote_name, test.remote_url);

        let url = gitinfo::get_remote_url(&repo);
        assert_eq!(
            url.as_deref(),
            Some(test.remote_url),
            "Failed for {}",
            test.name
        );
    }
}

#[test]
fn test_concurrent_status_checks_performance() {
    use rayon::prelude::*;

    let tmp_dir = tempfile::tempdir().unwrap();
    let base_path = tmp_dir.path();

    // Create 20 repositories
    let repo_count = 20;
    let mut repo_paths = Vec::new();

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
        } else if i % 3 == 1 {
            add_remote(
                &repo,
                "upstream",
                &format!("https://github.com/upstream/repo_{i}.git"),
            );
            create_remote_branch(&repo, "upstream", "main", initial_commit);
            make_commit(&repo, &repo_path.join("test.txt"), "new", "Second");
        }
        // else: no remote

        repo_paths.push(repo_path);
    }

    // Sequential check
    let start_seq = Instant::now();
    for path in &repo_paths {
        let repo = Repository::open(path).unwrap();
        let _status = Status::new(&repo);
    }
    let duration_seq = start_seq.elapsed();

    // Parallel check (simulating real-world usage with rayon)
    let start_par = Instant::now();
    repo_paths.par_iter().for_each(|path| {
        let repo = Repository::open(path).unwrap();
        let _status = Status::new(&repo);
    });
    let duration_par = start_par.elapsed();

    println!(
        "Sequential: {:?}, Parallel: {:?}, Speedup: {:.2}x",
        duration_seq,
        duration_par,
        duration_seq.as_secs_f64() / duration_par.as_secs_f64()
    );

    // Both should complete in reasonable time
    assert!(duration_seq.as_secs() < 2);
    assert!(duration_par.as_secs() < 2);
}
