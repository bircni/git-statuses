# git-statuses

[![Crates.io](https://img.shields.io/crates/v/git-statuses.svg)](https://crates.io/crates/git-statuses)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/bircni/git-statuses/blob/main/LICENSE)
[![CI](https://github.com/bircni/git-statuses/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/bircni/git-statuses/actions/workflows/ci.yml)

A command-line tool to display the status of multiple Git repositories in a clear, tabular format.

## Features

- Scans directories recursively for Git repositories
- Displays status (clean/dirty, branch, etc.) in a table
- Fast and user-friendly CLI
- Useful for developers managing many repositories

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
Usage: git-statuses [OPTIONS] [DIR]

Arguments:
  [DIR]  Directory to scan [default: .]

Options:
  -a, --all      Recursively scan all subdirectories
  -r, --remote   Show remote URL
  -h, --help     Print help
  -V, --version  Print version
```

## Output

The tool prints a table with the following columns:

- Path
- Branch
- Status (clean/dirty)
- Ahead/Behind

## Development

- Requires Rust 1.85+ (edition 2024)
- Linting: `cargo clippy`
- Tests: `cargo test`

## Contributing

Contributions are welcome! Please open issues or pull requests.
