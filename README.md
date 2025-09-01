# git-statuses

[![Crates.io](https://img.shields.io/crates/v/git-statuses.svg)](https://crates.io/crates/git-statuses)
[![Github All Releases](https://img.shields.io/github/downloads/bircni/git-statuses/total.svg)](https://github.com/bircni/git-statuses/releases)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/bircni/git-statuses/blob/main/LICENSE)
[![CI](https://github.com/bircni/git-statuses/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/bircni/git-statuses/actions/workflows/ci.yml)

A command-line tool to display the status of multiple Git repositories in a clear, tabular format.

## Features

- Scans directories recursively for Git repositories
- Displays status (clean/dirty, branch, etc.) in a table
- Fast and user-friendly CLI
- Useful for developers managing many repositories

![Example](https://github.com/user-attachments/assets/ba90b3ad-affa-44e5-8fef-7204ba49fd68)

## Installation

You need [Rust](https://www.rust-lang.org/tools/install) installed.

```sh
cargo install git-statuses
```

Installation with `cargo-binstall`:

```sh
cargo binstall git-statuses
```

Or clone and build manually:

```sh
git clone https://github.com/bircni/git-statuses.git
cd git-statuses
cargo build --release
```

## Usage

Run in any directory to scan for Git repositories:

```text
A tool to display git repository statuses in a table format

Usage: git-statuses.exe [OPTIONS] [DIR]

Arguments:
  [DIR]  Directory to scan [default: .]

Options:
  -d, --depth <DEPTH>        Recursively scan all subdirectories to the given depth. If set to 1, only the current directory is scanned. If set to -1, all subdirectories are scanned. (this may take a while) [default: 1]
  -r, --remote               Show remote URL
  -c, --condensed            Use a condensed layout
  -s, --summary              Show a summary of the scan
  -f, --fetch                Run a fetch before scanning to update the repository state Note: This may take a while for large repositories
  -l, --legend               Print a legend explaining the color codes and statuses used in the output
      --subdir <SUBDIR>      Look in a specific subdir if it exists for each folder This can be useful, if you don't checkout in a folder directly but in a subfolder like `repo-name/checkout`
      --completions <SHELL>  Generate shell completions [possible values: bash, elvish, fish, powershell, zsh]
  -p, --path                 Show the path to the repository
  -n, --non-clean            Only show non clean repositories
  -i, --interactive          Enable interactive mode to select and interact with repositories
  -h, --help                 Print help
  -V, --version              Print version
```

## Output

The tool prints a table with the following columns:

- Path
- Branch
- Status (clean/dirty)
- Ahead/Behind

## Development

- Requires Rust 1.88+ (edition 2024)
- Linting: `cargo clippy`
- Tests: `cargo test`

## Contributing

Contributions are welcome! Please open issues or pull requests.
