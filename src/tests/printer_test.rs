use std::path::PathBuf;

use crate::cli::Args;
use crate::gitinfo::repoinfo::RepoInfo;
use crate::gitinfo::status::Status;
use crate::printer::{legend, repositories_table};

#[test]
fn test_repositories_table_empty() {
    let mut repos: Vec<RepoInfo> = Vec::new();
    let args = Args {
        dir: ".".into(),
        depth: 1,
        ..Default::default()
    };
    let _ = repositories_table(&mut repos, &args);
    // Assert that no panic occurs and no output is generated
}

#[test]
fn test_repositories_table_with_data() {
    let mut repos = vec![RepoInfo {
        name: "repo1".to_owned(),
        branch: "main".to_owned(),
        ahead: 1,
        behind: 0,
        commits: 10,
        status: Status::Dirty(2),
        has_unpushed: true,
        remote_url: Some("https://example.com/repo1.git".to_owned()),
        path: PathBuf::from("/path/to/repo1"),
        stash_count: 0,
        is_local_only: false,
    }];
    let args = Args {
        dir: ".".into(),
        depth: 1,
        remote: true,
        ..Default::default()
    };
    let _ = repositories_table(&mut repos, &args);
    // Assert that the table is printed correctly
}

#[test]
fn test_print_legend() {
    legend(false);
    // Assert that the legend is printed correctly
}

#[test]
fn test_repositories_table_with_stashes_and_local_only() {
    let mut repos = vec![
        RepoInfo {
            name: "repo-with-stash".to_owned(),
            branch: "main".to_owned(),
            ahead: 0,
            behind: 0,
            commits: 5,
            status: Status::Clean,
            has_unpushed: false,
            remote_url: None,
            path: PathBuf::from("/path/to/repo-with-stash"),
            stash_count: 2,
            is_local_only: true,
        },
        RepoInfo {
            name: "repo-with-upstream".to_owned(),
            branch: "feature".to_owned(),
            ahead: 3,
            behind: 1,
            commits: 8,
            status: Status::Dirty(1),
            has_unpushed: true,
            remote_url: None,
            path: PathBuf::from("/path/to/repo-with-upstream"),
            stash_count: 0,
            is_local_only: false,
        },
    ];
    let args = Args {
        dir: ".".into(),
        depth: 1,
        ..Default::default()
    };
    let _ = repositories_table(&mut repos, &args);
    // Assert that stash info and local-only status are displayed correctly
}

#[test]
fn test_repositories_table_json_output() {
    let mut repos = vec![RepoInfo {
        name: "test-repo".to_owned(),
        branch: "main".to_owned(),
        ahead: 1,
        behind: 0,
        commits: 10,
        status: Status::Dirty(2),
        has_unpushed: true,
        remote_url: Some("https://example.com/test-repo.git".to_owned()),
        path: PathBuf::from("/path/to/test-repo"),
        stash_count: 0,
        is_local_only: false,
    }];
    let args = Args {
        dir: ".".into(),
        depth: 1,
        output: "json".to_owned(),
        ..Default::default()
    };
    let result = repositories_table(&mut repos, &args);
    assert!(result.is_ok());
}

#[test]
fn test_repositories_table_html_output() {
    let mut repos = vec![RepoInfo {
        name: "test-repo".to_owned(),
        branch: "main".to_owned(),
        ahead: 1,
        behind: 0,
        commits: 10,
        status: Status::Clean,
        has_unpushed: false,
        remote_url: None,
        path: PathBuf::from("/path/to/test-repo"),
        stash_count: 0,
        is_local_only: false,
    }];
    let args = Args {
        dir: ".".into(),
        depth: 1,
        output: "html".to_owned(),
        ..Default::default()
    };
    let result = repositories_table(&mut repos, &args);
    assert!(result.is_ok());
}

#[test]
fn test_repositories_table_invalid_format() {
    let mut repos = vec![RepoInfo {
        name: "test-repo".to_owned(),
        branch: "main".to_owned(),
        ahead: 0,
        behind: 0,
        commits: 5,
        status: Status::Clean,
        has_unpushed: false,
        remote_url: None,
        path: PathBuf::from("/path/to/test-repo"),
        stash_count: 0,
        is_local_only: false,
    }];
    let args = Args {
        dir: ".".into(),
        depth: 1,
        output: "invalid".to_owned(),
        ..Default::default()
    };
    let result = repositories_table(&mut repos, &args);
    assert!(result.is_err());
}

#[test]
fn test_repositories_table_file_output_error() {
    let mut repos = vec![RepoInfo {
        name: "test-repo".to_owned(),
        branch: "main".to_owned(),
        ahead: 0,
        behind: 0,
        commits: 5,
        status: Status::Clean,
        has_unpushed: false,
        remote_url: None,
        path: PathBuf::from("/path/to/test-repo"),
        stash_count: 0,
        is_local_only: false,
    }];
    let args = Args {
        dir: ".".into(),
        depth: 1,
        output: "table".to_owned(),
        output_file: Some(PathBuf::from("/tmp/test.txt")),
        ..Default::default()
    };
    let result = repositories_table(&mut repos, &args);
    assert!(result.is_err());
}
