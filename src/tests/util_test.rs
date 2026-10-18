use crate::cli::Args;
use crate::gitinfo::{repoinfo::RepoInfo, status::Status};
use crate::printer;
use crate::scan;
use crate::util::{GitPathExt, initialize_logger};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Renders one printer call into a string so tests can assert on real output.
fn capture(f: impl FnOnce(&mut Vec<u8>)) -> String {
    let mut buf = Vec::new();
    f(&mut buf);
    String::from_utf8(buf).unwrap()
}

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
    let (repos, failed) = scan::find_repositories(&args);
    assert!(repos.is_empty());
    assert_eq!(failed, Vec::<String>::new());
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
        path: PathBuf::from("/path/to/dummy"),
        stash_count: 0,
        is_local_only: false,
        fast_forwarded: false,
        repo_path: "dummy".to_owned(),
        is_worktree: false,
        is_detached: false,
    };
    let args = Args {
        dir: Path::new(".").to_path_buf(),
        depth: 1,
        summary: true,
        ..Default::default()
    };
    let repos = vec![repo];
    let table = capture(|w| printer::repositories_table(&repos, &args, w));
    assert!(
        table.contains("dummy"),
        "the repository must appear in the table, got:\n{table}"
    );
    let summary = capture(|w| printer::summary(&repos, 0, w));
    assert!(
        summary.contains("Total repositories:   1"),
        "the summary must count the repository, got:\n{summary}"
    );
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
    let (repos, failed) = scan::find_repositories(&args);
    assert!(repos.is_empty());
    assert_eq!(failed, Vec::<String>::new());
}

// New tests for GitPathExt trait
#[test]
fn test_git_path_ext_is_git_directory() {
    let temp = TempDir::new().unwrap();

    // Test non-git directory
    let non_git_dir = temp.path().join("not-git");
    fs::create_dir_all(&non_git_dir).unwrap();
    assert!(!non_git_dir.is_git_directory());

    // Test git directory
    let git_dir = temp.path().join("git-repo");
    fs::create_dir_all(&git_dir).unwrap();
    fs::create_dir_all(git_dir.join(".git")).unwrap();
    assert!(git_dir.is_git_directory());

    // Test file (not directory)
    let file_path = temp.path().join("file.txt");
    fs::write(&file_path, "content").unwrap();
    assert!(!file_path.is_git_directory());

    // Test non-existent path
    let non_existent = temp.path().join("does-not-exist");
    assert!(!non_existent.is_git_directory());
}

#[test]
fn test_git_path_ext_dir_name() {
    // Test normal directory name
    let path = Path::new("/home/user/my-repo");
    assert_eq!(path.dir_name(), "my-repo");

    // Test root path
    let root = Path::new("/");
    assert_eq!(root.dir_name(), "unknown");

    // Test relative path
    let relative = Path::new("relative-path");
    assert_eq!(relative.dir_name(), "relative-path");

    // Test path with special characters
    let special = Path::new("/home/user/repo-with_special.chars");
    assert_eq!(special.dir_name(), "repo-with_special.chars");

    // Test empty path
    let empty = Path::new("");
    assert_eq!(empty.dir_name(), "unknown");
}

#[test]
fn test_git_path_ext_dir_name_unicode() {
    // Test Unicode characters in directory names
    let unicode_path = Path::new("/home/user/репозиторий");
    assert_eq!(unicode_path.dir_name(), "репозиторий");

    let emoji_path = Path::new("/home/user/🚀-repo");
    assert_eq!(emoji_path.dir_name(), "🚀-repo");
}

#[test]
fn test_find_repositories_with_failed_repos() {
    let temp = TempDir::new().unwrap();

    // Create a fake .git directory that's actually a file
    let fake_git_dir = temp.path().join("fake-repo");
    fs::create_dir_all(&fake_git_dir).unwrap();
    fs::write(fake_git_dir.join(".git"), "this is not a git directory").unwrap();

    let args = Args {
        dir: temp.path().to_path_buf(),
        depth: 1,
        ..Default::default()
    };

    let (repos, failed) = scan::find_repositories(&args);

    assert_eq!(repos.len(), 0);
    assert_eq!(failed.len(), 1);
    assert_eq!(failed[0], "fake-repo");
}
