use crate::cli::Args;
use crate::gitinfo::{RepoInfo, status::Status};
use crate::printer;
use crate::util::{find_repositories, initialize_logger};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn test_initialize_logger() {
    initialize_logger().unwrap();
}

#[test]
fn test_find_repositories_empty_dir() {
    let temp = TempDir::new().unwrap();
    let args = Args {
        dir: temp.path().to_path_buf(),
        depth: 1,
        ..Default::default()
    };
    let (repos, failed) = find_repositories(&args);
    assert!(repos.is_empty());
    assert!(failed.is_empty());
}

#[test]
fn test_print_repositories_and_summary() {
    // Dummy RepoInfo for smoke test
    let repo = RepoInfo {
        name: "dummy".to_owned(),
        branch: "main".to_owned(),
        ahead: 0,
        behind: 0,
        commits: 1,
        status: Status::Clean,
        has_unpushed: false,
        remote_url: None,
    };
    let args = Args {
        dir: Path::new(".").to_path_buf(),
        depth: 1,
        summary: true,
        ..Default::default()
    };
    let mut repos = vec![repo];
    printer::repositories_table(&mut repos, &args);
    printer::summary(&repos, 0);
}

#[test]
fn test_find_repositories_with_non_git_dir() {
    let temp = TempDir::new().unwrap();
    let subdir = temp.path().join("foo");
    fs::create_dir_all(&subdir).unwrap();
    let args = Args {
        dir: temp.path().to_path_buf(),
        depth: 1,
        ..Default::default()
    };
    let (repos, failed) = find_repositories(&args);
    assert!(repos.is_empty());
    assert!(failed.is_empty());
}

#[test]
fn test_print_repositories_with_remote() {
    let repo = RepoInfo {
        name: "dummy".to_owned(),
        branch: "main".to_owned(),
        ahead: 0,
        behind: 0,
        commits: 1,
        status: Status::Clean,
        has_unpushed: false,
        remote_url: Some("https://example.com".to_owned()),
    };
    let args = Args {
        dir: Path::new(".").to_path_buf(),
        depth: 1,
        remote: true,
        ..Default::default()
    };
    let mut repos = vec![repo];
    printer::repositories_table(&mut repos, &args);
}
