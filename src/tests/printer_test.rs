use std::path::PathBuf;

use strum::IntoEnumIterator as _;

use crate::cli::Args;
use crate::gitinfo::repoinfo::RepoInfo;
use crate::gitinfo::status::Status;
use crate::printer::{
    failed_summary, json_output, json_value, legend, repositories_table, summary,
};

/// Renders one printer call into a string so tests can assert on real output.
fn capture(f: impl FnOnce(&mut Vec<u8>)) -> String {
    let mut buf = Vec::new();
    f(&mut buf);
    String::from_utf8(buf).unwrap()
}

#[test]
fn test_repositories_table_empty() {
    let repos: Vec<RepoInfo> = Vec::new();
    let args = Args {
        dir: ".".into(),
        depth: 1,
        ..Default::default()
    };
    let rendered = capture(|w| repositories_table(&repos, &args, w));
    assert!(
        rendered.is_empty(),
        "an empty scan must print no table at all, got: {rendered}"
    );
}

#[test]
fn test_repositories_table_with_data() {
    let repos = vec![RepoInfo {
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
        fast_forwarded: false,
        repo_path: "repo1".to_owned(),
        is_worktree: false,
    }];
    let args = Args {
        dir: ".".into(),
        depth: 1,
        remote: true,
        ..Default::default()
    };
    let rendered = capture(|w| repositories_table(&repos, &args, w));
    for expected in [
        "Directory",
        "Branch",
        "Local",
        "Commits",
        "Status",
        "Remote",
        "repo1",
        "main",
        "↑1 ↓0",
        "10",
        "Dirty (2)",
        "https://example.com/repo1.git",
    ] {
        assert!(
            rendered.contains(expected),
            "table must contain `{expected}`, got:\n{rendered}"
        );
    }
    assert!(
        !rendered.contains("Path"),
        "the Path column must stay hidden without `--path`, got:\n{rendered}"
    );
}

#[test]
fn test_print_legend() {
    let rendered = capture(|w| legend(false, w));
    // Every status must be documented, otherwise the legend silently goes stale when a
    // new variant is added.
    for status in Status::iter() {
        assert!(
            rendered.contains(&status.to_string()),
            "legend must list `{status}`, got:\n{rendered}"
        );
        assert!(
            rendered.contains(status.description()),
            "legend must describe `{status}`, got:\n{rendered}"
        );
    }
    assert!(
        rendered.contains("⎇ indicates a Git worktree"),
        "legend must explain the worktree marker, got:\n{rendered}"
    );
}

#[test]
fn test_repositories_table_with_stashes_and_local_only() {
    let repos = vec![
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
            fast_forwarded: false,
            repo_path: "repo-with-stash".to_owned(),
            is_worktree: false,
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
            fast_forwarded: false,
            repo_path: "repo-with-upstream".to_owned(),
            is_worktree: false,
        },
    ];
    let args = Args {
        dir: ".".into(),
        depth: 1,
        ..Default::default()
    };
    let rendered = capture(|w| repositories_table(&repos, &args, w));
    assert!(
        rendered.contains("Clean (2*)"),
        "a stash count must be shown next to the status, got:\n{rendered}"
    );
    assert!(
        rendered.contains("local-only"),
        "a branch without upstream must render as local-only, got:\n{rendered}"
    );
}

#[test]
fn test_repositories_table_with_path_option() {
    let repos = vec![RepoInfo {
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
        fast_forwarded: false,
        repo_path: "test-repo".to_owned(),
        is_worktree: false,
    }];
    let args = Args {
        dir: ".".into(),
        depth: 1,
        path: true,
        ..Default::default()
    };
    let rendered = capture(|w| repositories_table(&repos, &args, w));
    assert!(
        rendered.contains("Path") && rendered.contains("/very/long/path/to/repository"),
        "`--path` must add the Path column with the repository path, got:\n{rendered}"
    );
    assert!(
        rendered.contains("local-only"),
        "a branch without upstream must render as local-only, got:\n{rendered}"
    );
}

#[test]
fn test_repositories_table_condensed_layout() {
    let repos = vec![RepoInfo {
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
        fast_forwarded: false,
        repo_path: "repo".to_owned(),
        is_worktree: false,
    }];
    let args = Args {
        dir: ".".into(),
        depth: 1,
        condensed: true,
        remote: true,
        path: true,
        ..Default::default()
    };
    let rendered = capture(|w| repositories_table(&repos, &args, w));
    for expected in [
        "Directory",
        "Remote",
        "Path",
        "repo",
        "develop",
        "Merge (1*)",
    ] {
        assert!(
            rendered.contains(expected),
            "condensed table must contain `{expected}`, got:\n{rendered}"
        );
    }
    // The condensed preset drops the separator line drawn between rows; with a single row
    // there is none to drop, so only the multi-row case can tell the presets apart.
    assert!(
        !rendered.contains('╌'),
        "the condensed preset must not draw inter-row separators, got:\n{rendered}"
    );
}

#[test]
fn test_repositories_table_non_clean_filter() {
    let repos = vec![
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
            fast_forwarded: false,
            repo_path: "clean-repo".to_owned(),
            is_worktree: false,
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
            fast_forwarded: false,
            repo_path: "dirty-repo".to_owned(),
            is_worktree: false,
        },
    ];
    let args = Args {
        dir: ".".into(),
        depth: 1,
        non_clean: true,
        ..Default::default()
    };
    let displayed = args.filter_repos(&repos);
    assert_eq!(displayed.len(), 1);
    assert_eq!(displayed[0].name, "dirty-repo");
    let rendered = capture(|w| repositories_table(&displayed, &args, w));
    assert!(
        rendered.contains("dirty-repo") && !rendered.contains("clean-repo"),
        "`--non-clean` must show only the dirty repository, got:\n{rendered}"
    );
}

/// Sorting is the responsibility of `Args::find_repositories`, which hands the printer an
/// already ordered slice. The printer must render the rows in exactly that order.
#[test]
fn test_repositories_table_renders_rows_in_given_order() {
    let repos = vec![
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
            fast_forwarded: false,
            repo_path: "zebra-repo".to_owned(),
            is_worktree: false,
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
            fast_forwarded: false,
            repo_path: "Alpha-Repo".to_owned(),
            is_worktree: false,
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
            fast_forwarded: false,
            repo_path: "beta-repo".to_owned(),
            is_worktree: false,
        },
    ];
    let args = Args {
        dir: ".".into(),
        depth: 1,
        ..Default::default()
    };
    let rendered = capture(|w| repositories_table(&repos, &args, w));
    // The printer must not reorder what it was given.
    let zebra = rendered.find("zebra-repo").unwrap();
    let alpha = rendered.find("Alpha-Repo").unwrap();
    let beta = rendered.find("beta-repo").unwrap();
    assert!(
        zebra < alpha && alpha < beta,
        "the printer must keep the order it was given, got:\n{rendered}"
    );
    assert_eq!(repos[0].name, "zebra-repo");
    assert_eq!(repos[1].name, "Alpha-Repo");
    assert_eq!(repos[2].name, "beta-repo");
}

#[test]
fn test_repositories_table_various_statuses() {
    let repos = vec![
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
            fast_forwarded: false,
            repo_path: "rebase-repo".to_owned(),
            is_worktree: false,
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
            fast_forwarded: false,
            repo_path: "cherry-repo".to_owned(),
            is_worktree: false,
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
            fast_forwarded: false,
            repo_path: "bisect-repo".to_owned(),
            is_worktree: false,
        },
    ];
    let args = Args {
        dir: ".".into(),
        depth: 1,
        ..Default::default()
    };
    let rendered = capture(|w| repositories_table(&repos, &args, w));
    for repo in &repos {
        assert!(
            rendered.contains(&repo.status.to_string()),
            "status `{}` must appear in the table, got:\n{rendered}",
            repo.status
        );
    }
}

#[test]
fn test_legend_condensed() {
    let rendered = capture(|w| legend(true, w));
    assert_ne!(
        rendered,
        capture(|w| legend(false, w)),
        "the condensed legend must differ from the full one"
    );
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
            fast_forwarded: false,
            repo_path: "clean1".to_owned(),
            is_worktree: false,
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
            fast_forwarded: false,
            repo_path: "clean2".to_owned(),
            is_worktree: false,
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
            fast_forwarded: false,
            repo_path: "dirty".to_owned(),
            is_worktree: false,
        },
    ];

    let rendered = capture(|w| summary(&repos, 1, w)); // 1 failed repo
    for expected in [
        "Total repositories:   3",
        "Clean:                2",
        "With changes:         1",
        "With unpushed:        1",
        "With stashes:         2",
        "Failed to process:    1",
    ] {
        assert!(
            rendered.contains(expected),
            "summary must report `{expected}`, got:\n{rendered}"
        );
    }

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
        "broken-repo-1".to_owned(),
        "corrupted-repo-2".to_owned(),
        "invalid-git-dir".to_owned(),
    ];
    failed_summary(&failed_repos);
    // Should print warning about failed repos
}

#[test]
fn test_summary_edge_cases() {
    // Test with no repos
    let empty_repos: Vec<RepoInfo> = vec![];
    let rendered = capture(|w| summary(&empty_repos, 0, w));
    assert!(
        rendered.contains("Total repositories:   0"),
        "an empty scan must still report a total, got:\n{rendered}"
    );
    assert!(
        !rendered.contains("Failed to process"),
        "the failed line must be omitted when nothing failed, got:\n{rendered}"
    );

    // Test with only failed repos
    let rendered = capture(|w| summary(&empty_repos, 5, w));
    assert!(
        rendered.contains("Failed to process:    5"),
        "failures must be reported, got:\n{rendered}"
    );

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
        fast_forwarded: false,
        repo_path: "unknown-status".to_owned(),
        is_worktree: false,
    }];
    let rendered = capture(|w| summary(&edge_repos, 0, w));
    assert!(
        rendered.contains("Total repositories:   1")
            && rendered.contains("Local-only branches:  1"),
        "the summary must count a local-only repository, got:\n{rendered}"
    );
}

#[test]
fn test_repositories_table_marks_worktree_rows() {
    let repos = vec![RepoInfo {
        name: "worktree-repo".to_owned(),
        branch: "feature".to_owned(),
        ahead: 0,
        behind: 0,
        commits: 3,
        status: Status::Clean,
        has_unpushed: false,
        remote_url: None,
        path: PathBuf::from("/path/to/worktree-repo"),
        stash_count: 0,
        is_local_only: false,
        fast_forwarded: false,
        repo_path: "worktree-repo".to_owned(),
        is_worktree: true,
    }];
    let args = Args {
        dir: ".".into(),
        depth: 1,
        ..Default::default()
    };
    let rendered = capture(|w| repositories_table(&repos, &args, w));
    assert!(
        rendered.contains("⎇ worktree-repo"),
        "a worktree must be marked with ⎇ in the Directory column, got:\n{rendered}"
    );
}

#[test]
fn test_json_output_smoke() {
    let repos = vec![RepoInfo {
        name: "json-repo".to_owned(),
        branch: "main".to_owned(),
        ahead: 0,
        behind: 0,
        commits: 1,
        status: Status::Clean,
        has_unpushed: false,
        remote_url: None,
        path: PathBuf::from("/path/to/json-repo"),
        stash_count: 0,
        is_local_only: false,
        fast_forwarded: false,
        repo_path: "json-repo".to_owned(),
        is_worktree: false,
    }];
    let failed = vec!["broken-repo".to_owned()];
    let rendered = capture(|w| json_output(&repos, &failed, w));

    let value = json_value(&repos, &failed);
    assert_eq!(value["repositories"][0]["name"], "json-repo");
    assert_eq!(value["failed"][0], "broken-repo");
    // What is printed must be exactly the value the tests assert against, otherwise these
    // assertions say nothing about the real `--json` output.
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&rendered).unwrap(),
        value,
        "`json_output` must print `json_value` verbatim"
    );
}

fn repo_named(name: &str, status: Status) -> RepoInfo {
    RepoInfo {
        name: name.to_owned(),
        branch: "main".to_owned(),
        ahead: 0,
        behind: 0,
        commits: 1,
        status,
        has_unpushed: false,
        remote_url: None,
        path: PathBuf::from("/path/to").join(name),
        stash_count: 0,
        is_local_only: false,
        fast_forwarded: false,
        repo_path: name.to_owned(),
        is_worktree: false,
    }
}

/// `--non-clean` used to be applied by the table printer only, so `--json --non-clean`
/// still emitted every clean repository. Both output formats must show the same selection.
#[test]
fn test_non_clean_filter_applies_to_json_output() {
    let repos = vec![
        repo_named("clean-repo", Status::Clean),
        repo_named("dirty-repo", Status::Dirty(2)),
        repo_named("unpushed-repo", Status::Unpushed),
    ];
    let args = Args {
        dir: ".".into(),
        depth: 1,
        non_clean: true,
        json: true,
        ..Default::default()
    };

    let displayed = args.filter_repos(&repos);
    let names: Vec<&str> = displayed.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(
        names,
        ["dirty-repo", "unpushed-repo"],
        "clean repositories must be filtered out"
    );

    let value = json_value(&displayed, &[]);
    let json_names: Vec<&str> = value["repositories"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        json_names,
        ["dirty-repo", "unpushed-repo"],
        "JSON output must honour --non-clean"
    );
}

/// Without `--non-clean` the filter must be a no-op and must not clone the input.
#[test]
fn test_filter_repos_without_non_clean_borrows_everything() {
    let repos = vec![
        repo_named("clean-repo", Status::Clean),
        repo_named("dirty-repo", Status::Dirty(1)),
    ];
    let args = Args {
        dir: ".".into(),
        depth: 1,
        ..Default::default()
    };

    let displayed = args.filter_repos(&repos);
    assert!(
        matches!(displayed, std::borrow::Cow::Borrowed(_)),
        "the unfiltered path must not copy the scan result"
    );
    assert_eq!(displayed.len(), 2);
}

/// When `--non-clean` filters everything away, the printer must say so instead of drawing
/// a table with a header and no rows.
#[test]
fn test_non_clean_filter_removing_everything_prints_no_repositories() {
    let repos = vec![
        repo_named("clean-a", Status::Clean),
        repo_named("clean-b", Status::Clean),
    ];
    let args = Args {
        dir: ".".into(),
        depth: 1,
        non_clean: true,
        ..Default::default()
    };

    let displayed = args.filter_repos(&repos);
    assert!(displayed.is_empty());
    // Hits the "No repositories found." branch rather than rendering an empty table.
    let rendered = capture(|w| repositories_table(&displayed, &args, w));
    assert!(
        rendered.is_empty(),
        "a fully filtered scan must print no table, got:\n{rendered}"
    );
}
