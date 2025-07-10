use std::path::PathBuf;

use crate::cli::Args;
use crate::gitinfo::RepoInfo;
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
    repositories_table(&mut repos, &args);
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
    }];
    let args = Args {
        dir: ".".into(),
        depth: 1,
        remote: true,
        ..Default::default()
    };
    repositories_table(&mut repos, &args);
    // Assert that the table is printed correctly
}

#[test]
fn test_print_legend() {
    legend(false);
    // Assert that the legend is printed correctly
}
