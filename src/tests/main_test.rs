use std::{fs, io, path::Path, path::PathBuf};

use clap::Parser;
use clap_complete::Shell;
use git2::Repository;
use tempfile::TempDir;

use crate::{
    cli::Args,
    completions,
    gitinfo::{repoinfo::RepoInfo, status::Status},
    run,
};

fn repo_info_with_status(status: Status, stash_count: usize, fast_forwarded: bool) -> RepoInfo {
    RepoInfo {
        name: "repo".to_owned(),
        branch: "main".to_owned(),
        ahead: 3,
        behind: 1,
        commits: 42,
        status,
        has_unpushed: true,
        remote_url: Some("https://example.com/repo.git".to_owned()),
        path: PathBuf::from("/tmp/repo"),
        stash_count,
        is_local_only: false,
        fast_forwarded,
        repo_path: "repo".to_owned(),
        is_worktree: false,
    }
}

#[test]
fn test_repo_info_format_local_status_local_only() {
    let mut repo = repo_info_with_status(Status::Clean, 0, false);
    repo.is_local_only = true;
    assert_eq!(repo.format_local_status(), "local-only");
}

#[test]
fn test_repo_info_format_local_status_with_upstream_counts() {
    let repo = repo_info_with_status(Status::Clean, 0, false);
    assert_eq!(repo.format_local_status(), "↑3 ↓1");
}

#[test]
fn test_repo_info_format_status_with_stash_only() {
    let repo = repo_info_with_status(Status::Dirty(2), 4, false);
    assert_eq!(repo.format_status_with_stash_and_ff(), "Dirty (2) (4*)");
}

#[test]
fn test_repo_info_format_status_with_fast_forward_only() {
    let repo = repo_info_with_status(Status::Clean, 0, true);
    assert_eq!(repo.format_status_with_stash_and_ff(), "Clean ↑↑");
}

#[test]
fn test_repo_info_format_status_with_stash_and_fast_forward() {
    let repo = repo_info_with_status(Status::Unpushed, 2, true);
    assert_eq!(repo.format_status_with_stash_and_ff(), "Unpushed (2*) ↑↑");
}

#[test]
fn test_args_parse_json_fast_forward_and_subdir() {
    let args = Args::parse_from([
        "git-statuses",
        "--json",
        "--ff",
        "--subdir",
        "checkout",
        "--depth=-1",
        ".",
    ]);
    assert!(args.json);
    assert!(args.fast_forward);
    assert_eq!(args.subdir.as_deref(), Some("checkout"));
    assert_eq!(args.depth, -1);
}

/// Creates a repository with one commit, and optionally an uncommitted file.
fn create_repo(parent: &Path, name: &str, dirty: bool) {
    let repo_path = parent.join(name);
    fs::create_dir_all(&repo_path).unwrap();
    let repo = Repository::init(&repo_path).unwrap();

    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test User").unwrap();
    config.set_str("user.email", "test@example.com").unwrap();
    drop(config);

    fs::write(repo_path.join("README.md"), "# Test\n").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("README.md")).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = repo.signature().unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
        .unwrap();

    if dirty {
        fs::write(repo_path.join("uncommitted.txt"), "work in progress").unwrap();
    }
}

/// A scan directory holding one clean and one dirty repository.
fn scan_dir() -> TempDir {
    let temp = TempDir::new().unwrap();
    create_repo(temp.path(), "clean-repo", false);
    create_repo(temp.path(), "dirty-repo", true);
    temp
}

#[test]
fn test_run_prints_table() {
    let temp = scan_dir();
    let args = Args {
        dir: temp.path().to_path_buf(),
        depth: 1,
        ..Default::default()
    };
    run(&args, &mut io::sink());
}

#[test]
fn test_run_with_all_display_options() {
    let temp = scan_dir();
    let args = Args {
        dir: temp.path().to_path_buf(),
        depth: 1,
        remote: true,
        path: true,
        condensed: true,
        summary: true,
        ..Default::default()
    };
    run(&args, &mut io::sink());
}

#[test]
fn test_run_with_non_clean_filter() {
    let temp = scan_dir();
    let args = Args {
        dir: temp.path().to_path_buf(),
        depth: 1,
        non_clean: true,
        summary: true,
        ..Default::default()
    };
    run(&args, &mut io::sink());
}

#[test]
fn test_run_json() {
    let temp = scan_dir();
    let args = Args {
        dir: temp.path().to_path_buf(),
        depth: 1,
        json: true,
        ..Default::default()
    };
    run(&args, &mut io::sink());
}

#[test]
fn test_run_on_directory_without_repositories() {
    let temp = TempDir::new().unwrap();
    let args = Args {
        dir: temp.path().to_path_buf(),
        depth: 1,
        summary: true,
        ..Default::default()
    };
    run(&args, &mut io::sink());
}

/// A directory whose `.git` git cannot open is reported as failed, not as a hard error.
#[test]
fn test_run_reports_failed_repositories() {
    let temp = TempDir::new().unwrap();
    let broken = temp.path().join("broken-repo");
    fs::create_dir_all(&broken).unwrap();
    fs::write(broken.join(".git"), "not a git directory").unwrap();

    let args = Args {
        dir: temp.path().to_path_buf(),
        depth: 1,
        summary: true,
        ..Default::default()
    };
    run(&args, &mut io::sink());
}

#[test]
fn test_run_legend() {
    for condensed in [false, true] {
        let args = Args {
            legend: true,
            condensed,
            ..Default::default()
        };
        run(&args, &mut io::sink());
    }
}

/// `--completions` must short-circuit before anything is scanned or printed, so that it
/// stays usable from a shell's startup files no matter which directory it runs in.
#[test]
fn test_run_completions_short_circuits_the_scan() {
    let args = Args {
        dir: PathBuf::from("/nonexistent/directory/that/does/not/exist"),
        depth: -1,
        completions: Some(Shell::Bash),
        // Would both be honoured if the scan were reached.
        legend: true,
        json: true,
        ..Default::default()
    };

    let mut out = Vec::new();
    run(&args, &mut out);

    let script = String::from_utf8(out).unwrap();
    assert!(
        script.contains("git-statuses"),
        "completion script must mention the binary name"
    );
    assert!(
        !script.contains("Cherry Pick"),
        "the legend must not be printed when generating completions"
    );
}

#[test]
fn test_completions_for_every_supported_shell() {
    for shell in [
        Shell::Bash,
        Shell::Zsh,
        Shell::Fish,
        Shell::PowerShell,
        Shell::Elvish,
    ] {
        let mut out = Vec::new();
        completions(shell, &mut out);
        assert!(
            !out.is_empty(),
            "completion script for {shell} must not be empty"
        );
    }
}
