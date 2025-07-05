use crate::cli::Args;
use crate::gitinfo::RepoInfo;
use crate::printer;
use crate::util::find_repositories;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn test_find_repositories_empty_dir() {
    let temp = TempDir::new().unwrap();
    let args = Args {
        dir: temp.path().to_path_buf(),
        depth: 1,
        fetch: false,
        remote: false,
        summary: false,
        legend: false,
    };
    let (repos, _) = find_repositories(&args).unwrap();
    assert!(repos.is_empty());
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
        untracked: 0,
        status: "Clean".to_owned(),
        changed: 0,
        has_unpushed: false,
        remote_url: None,
    };
    let args = Args {
        dir: Path::new(".").to_path_buf(),
        depth: 1,
        fetch: false,
        remote: false,
        summary: true,
        legend: false,
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
        fetch: false,
        remote: false,
        summary: false,
        legend: false,
    };
    let (repos, _) = find_repositories(&args).unwrap();
    assert!(repos.is_empty());
}

#[test]
fn test_print_repositories_with_remote() {
    let repo = RepoInfo {
        name: "dummy".to_owned(),
        branch: "main".to_owned(),
        ahead: 0,
        behind: 0,
        commits: 1,
        untracked: 0,
        status: "Clean".to_owned(),
        changed: 0,
        has_unpushed: false,
        remote_url: Some("https://example.com".to_owned()),
    };
    let args = Args {
        dir: Path::new(".").to_path_buf(),
        depth: 1,
        fetch: false,
        remote: true,
        summary: false,
        legend: false,
    };
    let mut repos = vec![repo];
    printer::repositories_table(&mut repos, &args);
}
