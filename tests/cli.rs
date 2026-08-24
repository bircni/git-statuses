//! End-to-end tests that run the compiled binary.
//!
//! These exist for behaviour that only exists at the process boundary - which stream
//! output lands on, and what the exit code is - and which an in-process test driving
//! `run` with a buffer cannot observe.

#![expect(
    clippy::unwrap_used,
    reason = "Test setup: a failure here is a broken test, and the panic message is the report. \
              clippy.toml allows this inside `#[test]` functions but not in their helpers."
)]

use std::{fs, process::Command};

use tempfile::TempDir;

/// Path to the binary built for this test run.
const BIN: &str = env!("CARGO_BIN_EXE_git-statuses");

/// Builds a scan directory holding one readable repository and one that cannot be opened.
fn fixture() -> TempDir {
    let temp = TempDir::new().unwrap();

    let healthy = temp.path().join("healthy");
    fs::create_dir_all(&healthy).unwrap();
    let repo = git2::Repository::init(&healthy).unwrap();
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test User").unwrap();
    config.set_str("user.email", "test@example.com").unwrap();
    fs::write(healthy.join("file.txt"), "content").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("file.txt")).unwrap();
    index.write().unwrap();
    let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
    let sig = repo.signature().unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
        .unwrap();

    // A `.git` file pointing nowhere: the directory looks like a repository but cannot be
    // opened, which makes the binary log a diagnostic while scanning.
    let broken = temp.path().join("broken");
    fs::create_dir_all(&broken).unwrap();
    fs::write(broken.join(".git"), "gitdir: /nowhere").unwrap();

    temp
}

/// Diagnostics must not share stdout with the report.
///
/// The logger used to run in mixed mode, which put everything below `Warn` on stdout. In
/// a debug build that meant `--json` emitted debug lines ahead of the opening brace and
/// nothing downstream could parse it.
#[test]
fn json_output_is_the_only_thing_on_stdout() {
    let temp = fixture();
    let output = Command::new(BIN)
        .args(["--json", "--depth", "2"])
        .arg(temp.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    serde_json::from_str::<serde_json::Value>(&stdout)
        .unwrap_or_else(|e| panic!("`--json` stdout must parse as JSON ({e}), got:\n{stdout}"));

    // The diagnostic about the unreadable repository still has to be reported somewhere.
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("broken"),
        "the unreadable repository must be reported on stderr, got:\n{stderr}"
    );
}

/// The table is a report too and must not be interleaved with diagnostics.
#[test]
fn table_output_is_the_only_thing_on_stdout() {
    let temp = fixture();
    let output = Command::new(BIN)
        .args(["--depth", "2"])
        .arg(temp.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        !stdout.contains("DEBUG") && !stdout.contains("WARN"),
        "log records must not appear on stdout, got:\n{stdout}"
    );
}

/// A repository that cannot be processed makes the run fail, so scripts can notice.
#[test]
fn unreadable_repositories_make_the_process_exit_non_zero() {
    let temp = fixture();
    let output = Command::new(BIN)
        .args(["--depth", "2"])
        .arg(temp.path())
        .output()
        .unwrap();

    assert_eq!(
        output.status.code(),
        Some(1),
        "a scan with an unreadable repository must exit 1"
    );
}

/// A scan where everything was readable exits zero.
#[test]
fn a_healthy_scan_exits_zero() {
    let temp = TempDir::new().unwrap();
    let healthy = temp.path().join("healthy");
    fs::create_dir_all(&healthy).unwrap();
    git2::Repository::init(&healthy).unwrap();

    let output = Command::new(BIN)
        .args(["--depth", "2"])
        .arg(temp.path())
        .output()
        .unwrap();

    assert_eq!(
        output.status.code(),
        Some(0),
        "a scan with nothing broken must exit 0"
    );
}
