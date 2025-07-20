use std::fs;
use std::process::{Command, Stdio};
use tempfile::TempDir;

/// Test the main binary execution with various flags
/// These are more like end-to-end tests

#[test]
fn test_help_flag() {
    let output = Command::new("cargo")
        .args(["run", "--", "--help"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("A tool to display git repository statuses"));
    assert!(stdout.contains("--depth"));
    assert!(stdout.contains("--remote"));
}

#[test]
fn test_version_flag() {
    let output = Command::new("cargo")
        .args(["run", "--", "--version"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("git-statuses"));
}

#[test]
fn test_legend_flag() {
    let output = Command::new("cargo")
        .args(["run", "--", "--legend"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Status"));
    assert!(stdout.contains("Description"));
    assert!(stdout.contains("Clean"));
    assert!(stdout.contains("Dirty"));
}

#[test]
fn test_legend_condensed() {
    let output = Command::new("cargo")
        .args(["run", "--", "--legend", "--condensed"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Status"));
    assert!(stdout.contains("Clean"));
    // Should be in condensed format
}

#[test]
fn test_completions_bash() {
    let output = Command::new("cargo")
        .args(["run", "--", "--completions", "bash"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("_git-statuses"));
    assert!(stdout.contains("complete"));
}

#[test]
fn test_empty_directory() {
    let temp_dir = TempDir::new().unwrap();

    let output = Command::new("cargo")
        .args(["run", "--", temp_dir.path().to_str().unwrap()])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    assert!(output.status.success());
    // Should complete without error even with no repos
}

#[test]
fn test_nonexistent_directory() {
    let output = Command::new("cargo")
        .args(["run", "--", "/path/that/does/not/exist"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    // Should handle nonexistent directory gracefully
    let _stderr = String::from_utf8_lossy(&output.stderr);
    // May contain error message about directory not existing
    // but shouldn't crash
}

#[test]
fn test_with_actual_git_repo() {
    let temp_dir = TempDir::new().unwrap();

    // Create a git repository
    let repo_path = temp_dir.path().join("test-repo");
    fs::create_dir_all(&repo_path).unwrap();

    // Initialize git repo
    Command::new("git")
        .args(["init"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    // Configure git
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    // Create and commit a file
    fs::write(repo_path.join("README.md"), "# Test Repo").unwrap();
    Command::new("git")
        .args(["add", "README.md"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    // Run git-statuses on the directory
    let output = Command::new("cargo")
        .args(["run", "--", temp_dir.path().to_str().unwrap()])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("test-repo"));
    assert!(stdout.contains("main") || stdout.contains("master")); // branch name
}

#[test]
fn test_with_summary_flag() {
    let temp_dir = TempDir::new().unwrap();

    let output = Command::new("cargo")
        .args(["run", "--", "--summary", temp_dir.path().to_str().unwrap()])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Summary:"));
    assert!(stdout.contains("Total repositories:"));
}

#[test]
fn test_depth_flag_integration() {
    let temp_dir = TempDir::new().unwrap();

    // Create nested structure
    let level1 = temp_dir.path().join("level1");
    let level2 = level1.join("level2");
    fs::create_dir_all(&level2).unwrap();

    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "--depth",
            "3",
            temp_dir.path().to_str().unwrap(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    assert!(output.status.success());
    // Should scan deeper directories
}

#[test]
fn test_remote_flag_integration() {
    let temp_dir = TempDir::new().unwrap();

    let output = Command::new("cargo")
        .args(["run", "--", "--remote", temp_dir.path().to_str().unwrap()])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    assert!(output.status.success());
    // Should include remote column in output (even if empty)
}

#[test]
fn test_condensed_flag_integration() {
    let temp_dir = TempDir::new().unwrap();

    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "--condensed",
            temp_dir.path().to_str().unwrap(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    assert!(output.status.success());
    // Should use condensed table format
}

#[test]
fn test_multiple_flags_combination() {
    let temp_dir = TempDir::new().unwrap();

    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "--remote",
            "--condensed",
            "--summary",
            "--depth",
            "2",
            temp_dir.path().to_str().unwrap(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Summary:"));
    // Multiple flags should work together
}

#[test]
fn test_path_flag_integration() {
    let temp_dir = TempDir::new().unwrap();

    let output = Command::new("cargo")
        .args(["run", "--", "--path", temp_dir.path().to_str().unwrap()])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    assert!(output.status.success());
    // Should include path column in output
}

#[test]
fn test_non_clean_flag_integration() {
    let temp_dir = TempDir::new().unwrap();

    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "--non-clean",
            temp_dir.path().to_str().unwrap(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    assert!(output.status.success());
    // Should only show non-clean repositories
}
