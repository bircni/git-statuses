use std::path::PathBuf;

use crate::cli::Args;
use crate::gitinfo::repoinfo::RepoInfo;
use crate::gitinfo::status::Status;
use crate::printer::{failed_summary, legend, repositories_table, summary};

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
        stash_count: 0,
        is_local_only: false,
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
    repositories_table(&mut repos, &args);
    // Assert that stash info and local-only status are displayed correctly
}

#[test]
fn test_repositories_table_with_path_option() {
    let mut repos = vec![RepoInfo {
        name: "test-repo".to_owned(),
        branch: "main".to_owned(),
        ahead: 0,
        behind: 0,
        commits: 5,
        status: Status::Clean,
        has_unpushed: false,
        remote_url: None,
        path: PathBuf::from("/very/long/path/to/repository"),
        stash_count: 0,
        is_local_only: true,
    }];
    let args = Args {
        dir: ".".into(),
        depth: 1,
        path: true,
        ..Default::default()
    };
    repositories_table(&mut repos, &args);
    // Should include path column
}

#[test]
fn test_repositories_table_condensed_layout() {
    let mut repos = vec![RepoInfo {
        name: "repo".to_owned(),
        branch: "develop".to_owned(),
        ahead: 2,
        behind: 1,
        commits: 15,
        status: Status::Merge,
        has_unpushed: true,
        remote_url: Some("git@github.com:user/repo.git".to_owned()),
        path: PathBuf::from("/path/to/repo"),
        stash_count: 1,
        is_local_only: false,
    }];
    let args = Args {
        dir: ".".into(),
        depth: 1,
        condensed: true,
        remote: true,
        path: true,
        ..Default::default()
    };
    repositories_table(&mut repos, &args);
    // Should use condensed table format
}

#[test]
fn test_repositories_table_non_clean_filter() {
    let mut repos = vec![
        RepoInfo {
            name: "clean-repo".to_owned(),
            branch: "main".to_owned(),
            ahead: 0,
            behind: 0,
            commits: 5,
            status: Status::Clean,
            has_unpushed: false,
            remote_url: None,
            path: PathBuf::from("/path/to/clean"),
            stash_count: 0,
            is_local_only: false,
        },
        RepoInfo {
            name: "dirty-repo".to_owned(),
            branch: "main".to_owned(),
            ahead: 0,
            behind: 0,
            commits: 5,
            status: Status::Dirty(3),
            has_unpushed: false,
            remote_url: None,
            path: PathBuf::from("/path/to/dirty"),
            stash_count: 0,
            is_local_only: false,
        },
    ];
    let args = Args {
        dir: ".".into(),
        depth: 1,
        non_clean: true,
        ..Default::default()
    };
    repositories_table(&mut repos, &args);
    // Should only display dirty repo
}

#[test]
fn test_repositories_table_sorting() {
    let mut repos = vec![
        RepoInfo {
            name: "zebra-repo".to_owned(),
            branch: "main".to_owned(),
            ahead: 0,
            behind: 0,
            commits: 5,
            status: Status::Clean,
            has_unpushed: false,
            remote_url: None,
            path: PathBuf::from("/path/to/zebra"),
            stash_count: 0,
            is_local_only: false,
        },
        RepoInfo {
            name: "Alpha-Repo".to_owned(), // Capital letter
            branch: "main".to_owned(),
            ahead: 0,
            behind: 0,
            commits: 5,
            status: Status::Clean,
            has_unpushed: false,
            remote_url: None,
            path: PathBuf::from("/path/to/alpha"),
            stash_count: 0,
            is_local_only: false,
        },
        RepoInfo {
            name: "beta-repo".to_owned(),
            branch: "main".to_owned(),
            ahead: 0,
            behind: 0,
            commits: 5,
            status: Status::Clean,
            has_unpushed: false,
            remote_url: None,
            path: PathBuf::from("/path/to/beta"),
            stash_count: 0,
            is_local_only: false,
        },
    ];
    let args = Args {
        dir: ".".into(),
        depth: 1,
        ..Default::default()
    };
    repositories_table(&mut repos, &args);
    // Should be sorted alphabetically (case-insensitive)
    assert_eq!(repos[0].name, "Alpha-Repo");
    assert_eq!(repos[1].name, "beta-repo");
    assert_eq!(repos[2].name, "zebra-repo");
}

#[test]
fn test_repositories_table_various_statuses() {
    let mut repos = vec![
        RepoInfo {
            name: "rebase-repo".to_owned(),
            branch: "feature".to_owned(),
            ahead: 0,
            behind: 0,
            commits: 5,
            status: Status::Rebase,
            has_unpushed: false,
            remote_url: None,
            path: PathBuf::from("/path/to/rebase"),
            stash_count: 0,
            is_local_only: false,
        },
        RepoInfo {
            name: "cherry-repo".to_owned(),
            branch: "hotfix".to_owned(),
            ahead: 1,
            behind: 0,
            commits: 8,
            status: Status::CherryPick,
            has_unpushed: true,
            remote_url: None,
            path: PathBuf::from("/path/to/cherry"),
            stash_count: 0,
            is_local_only: false,
        },
        RepoInfo {
            name: "bisect-repo".to_owned(),
            branch: "main".to_owned(),
            ahead: 0,
            behind: 2,
            commits: 12,
            status: Status::Bisect,
            has_unpushed: false,
            remote_url: None,
            path: PathBuf::from("/path/to/bisect"),
            stash_count: 1,
            is_local_only: false,
        },
    ];
    let args = Args {
        dir: ".".into(),
        depth: 1,
        ..Default::default()
    };
    repositories_table(&mut repos, &args);
    // Should display all different status types with appropriate colors
}

#[test]
fn test_legend_condensed() {
    legend(true);
    // Should print condensed legend format
}

#[test]
fn test_summary_comprehensive() {
    let repos = vec![
        RepoInfo {
            name: "clean1".to_owned(),
            branch: "main".to_owned(),
            ahead: 0,
            behind: 0,
            commits: 5,
            status: Status::Clean,
            has_unpushed: false,
            remote_url: None,
            path: PathBuf::from("/path/to/clean1"),
            stash_count: 0,
            is_local_only: false,
        },
        RepoInfo {
            name: "clean2".to_owned(),
            branch: "main".to_owned(),
            ahead: 0,
            behind: 0,
            commits: 3,
            status: Status::Clean,
            has_unpushed: false,
            remote_url: None,
            path: PathBuf::from("/path/to/clean2"),
            stash_count: 1,      // has stash
            is_local_only: true, // local only
        },
        RepoInfo {
            name: "dirty".to_owned(),
            branch: "feature".to_owned(),
            ahead: 2,
            behind: 1,
            commits: 8,
            status: Status::Dirty(3),
            has_unpushed: true, // has unpushed
            remote_url: Some("https://example.com".to_owned()),
            path: PathBuf::from("/path/to/dirty"),
            stash_count: 2, // has stashes
            is_local_only: false,
        },
    ];

    summary(&repos, 1); // 1 failed repo

    // Should show:
    // - 3 total repos
    // - 2 clean repos
    // - 1 dirty repo
    // - 1 with unpushed
    // - 2 with stashes
    // - 1 local-only
    // - 1 failed
}

#[test]
fn test_failed_summary_empty() {
    let failed_repos: Vec<String> = vec![];
    failed_summary(&failed_repos);
    // Should not print anything
}

#[test]
fn test_failed_summary_multiple() {
    let failed_repos = vec![
        "broken-repo-1".to_string(),
        "corrupted-repo-2".to_string(),
        "invalid-git-dir".to_string(),
    ];
    failed_summary(&failed_repos);
    // Should print warning about failed repos
}

#[test]
fn test_summary_edge_cases() {
    // Test with no repos
    let empty_repos: Vec<RepoInfo> = vec![];
    summary(&empty_repos, 0);

    // Test with only failed repos
    summary(&empty_repos, 5);

    // Test with mixed edge cases
    let edge_repos = vec![RepoInfo {
        name: "unknown-status".to_owned(),
        branch: "detached".to_owned(),
        ahead: 0,
        behind: 0,
        commits: 0,
        status: Status::Unknown,
        has_unpushed: false,
        remote_url: None,
        path: PathBuf::from("/path/to/unknown"),
        stash_count: 0,
        is_local_only: true,
    }];
    summary(&edge_repos, 0);
}
