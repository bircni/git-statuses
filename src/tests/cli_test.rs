use insta::_macro_support;
use std::path::Path;
use std::str;

use clap::ColorChoice;
use clap::Parser;

use crate::cli::Args;

/// From <https://github.com/EmbarkStudios/cargo-deny/blob/f6e40d8eff6a507977b20588c842c53bc0bfd427/src/cargo-deny/main.rs#L369>
/// Snapshot tests for the CLI commands
#[expect(clippy::panic, reason = "Snapshot failed")]
fn snapshot_test_cli_command(app: clap::Command, cmd_name: &str) {
    let mut app_cmd = app
        .color(ColorChoice::Never)
        .version("0.0.0")
        .long_version("0.0.0");

    let mut buffer = Vec::new();
    app_cmd.write_long_help(&mut buffer).unwrap();
    let help_text = str::from_utf8(&buffer).unwrap();

    let snapshot = _macro_support::SnapshotValue::FileText {
        name: Some(cmd_name.into()),
        content: help_text,
    };

    if _macro_support::assert_snapshot(
        snapshot,
        Path::new(env!("CARGO_MANIFEST_DIR")),
        "cli-cmd",
        module_path!(),
        file!(),
        line!(),
        "help_text",
    )
    .is_err()
    {
        panic!("Snapshot failed");
    }

    for cmd in app_cmd.get_subcommands() {
        if cmd.get_name() == "help" {
            continue;
        }

        snapshot_test_cli_command(cmd.clone(), &format!("{cmd_name}-{}", cmd.get_name()));
    }
}

#[test]
fn test_cli_snapshot() {
    use clap::CommandFactory as _;

    insta::with_settings!({
        snapshot_path => "../tests/snapshots",
    }, {
        snapshot_test_cli_command(
            Args::command().name("git-statuses"),
            "git-statuses",
        );
    });
}

#[test]
fn test_cli_default_args() {
    let args = Args::parse_from(["git-statuses"]);
    assert_eq!(args.dir, Path::new("."));
    assert_eq!(args.depth, 1);
    assert!(!args.remote);
    assert!(!args.condensed);
    assert!(!args.summary);
    assert!(!args.fetch);
    assert!(!args.legend);
    assert!(args.subdir.is_none());
    assert!(args.completions.is_none());
    assert!(!args.path);
    assert!(!args.non_clean);
}

#[test]
fn test_cli_directory_argument() {
    let args = Args::parse_from(["git-statuses", "/path/to/repos"]);
    assert_eq!(args.dir, Path::new("/path/to/repos"));
}

#[test]
fn test_cli_depth_argument() {
    let args = Args::parse_from(["git-statuses", "--depth", "5"]);
    assert_eq!(args.depth, 5);

    let args = Args::parse_from(["git-statuses", "-d", "10"]);
    assert_eq!(args.depth, 10);
}

#[test]
fn test_cli_boolean_flags() {
    let args = Args::parse_from([
        "git-statuses",
        "--remote",
        "--condensed",
        "--summary",
        "--fetch",
        "--legend",
        "--path",
        "--non-clean",
    ]);
    assert!(args.remote);
    assert!(args.condensed);
    assert!(args.summary);
    assert!(args.fetch);
    assert!(args.legend);
    assert!(args.path);
    assert!(args.non_clean);
}

#[test]
fn test_cli_short_flags() {
    let args = Args::parse_from(["git-statuses", "-r", "-c", "-f", "-l", "-p"]);
    assert!(args.remote);
    assert!(args.condensed);
    assert!(args.fetch);
    assert!(args.legend);
    assert!(args.path);
}

#[test]
fn test_cli_subdir_argument() {
    let args = Args::parse_from(["git-statuses", "--subdir", "checkout"]);
    assert_eq!(args.subdir, Some("checkout".to_owned()));
}

#[test]
fn test_cli_complex_combinations() {
    let args = Args::parse_from([
        "git-statuses",
        "/home/user/projects",
        "--depth",
        "3",
        "--remote",
        "--summary",
        "--subdir",
        "checkout",
        "--condensed",
    ]);

    assert_eq!(args.dir, Path::new("/home/user/projects"));
    assert_eq!(args.depth, 3);
    assert!(args.remote);
    assert!(args.summary);
    assert_eq!(args.subdir, Some("checkout".to_owned()));
    assert!(args.condensed);
    assert!(!args.fetch); // not specified
    assert!(!args.legend); // not specified
}

#[test]
fn test_cli_completions_argument() {
    use clap_complete::Shell;

    let args = Args::parse_from(["git-statuses", "--completions", "bash"]);
    assert_eq!(args.completions, Some(Shell::Bash));

    let args = Args::parse_from(["git-statuses", "--completions", "zsh"]);
    assert_eq!(args.completions, Some(Shell::Zsh));

    let args = Args::parse_from(["git-statuses", "--completions", "fish"]);
    assert_eq!(args.completions, Some(Shell::Fish));
}

#[test]
fn test_cli_edge_case_depth_zero() {
    let args = Args::parse_from(["git-statuses", "--depth", "0"]);
    assert_eq!(args.depth, 0);
}

#[test]
fn test_cli_filter_combination() {
    // Test that non-clean filter works with other options
    let args = Args::parse_from(["git-statuses", "--non-clean", "--remote", "--summary"]);

    assert!(args.non_clean);
    assert!(args.remote);
    assert!(args.summary);
}

#[test]
fn test_cli_path_variations() {
    // Test relative path
    let args = Args::parse_from(["git-statuses", "."]);
    assert_eq!(args.dir, Path::new("."));

    // Test path with tilde (will be treated literally by clap)
    let args = Args::parse_from(["git-statuses", "~/projects"]);
    assert_eq!(args.dir, Path::new("~/projects"));

    // Test absolute path
    let args = Args::parse_from(["git-statuses", "/absolute/path"]);
    assert_eq!(args.dir, Path::new("/absolute/path"));
}
