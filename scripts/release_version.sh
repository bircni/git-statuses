#!/usr/bin/env bash

set -e

# Function to print errors only
err() { echo "$@" 1>&2; }

# enable push with follow tags
git config --global push.followTags true

# Check if git-cliff is installed
if ! command -v git-cliff &> /dev/null; then
    echo "git-cliff is not installed. Please install it first."
    exit 1
fi

# Check if cargo-verset is installed
if ! command -v cargo-verset &> /dev/null; then
    err "cargo-verset is not installed. Please install it with:"
    err "    cargo install cargo-verset"
    exit 1
fi

# Get the bumped version from git-cliff
version=$(git-cliff --bumped-version)
current_version=$(git describe --tags)

# if the version is the same as the current version, exit
if [ "$version" == "$current_version" ]; then
    echo "Version $version is already the current version. No changes made."
    exit 0
fi

echo "Calculated version: $version"
echo "Updating version in Cargo.toml..."
cargo verset package -v "$version"
echo "Version updated successfully in Cargo.toml and Cargo.lock."

# Generate the changelog
echo "Generating changelog..."
git cliff --output CHANGELOG.md -t "$version"
echo "Changelog generated successfully."

echo "Building the project to update Cargo.lock..."
cargo build --release --quiet
if [ $? -ne 0 ]; then
    err "Build failed. Please fix the issues and try again."
    exit 1
fi
echo "Project built successfully."
# Ask for confirmation before committing
echo "Do you want to commit the changes? (y/n)"
read -r answer
if [ "$answer" != "y" ]; then
    echo "Changes not committed. Exiting."
    exit 0
fi

# Commit changes
git add Cargo.toml CHANGELOG.md Cargo.lock
git commit -m "release($version)"
git tag -a "$version" -m "Release $version"

echo "Changes committed and tagged with version $version."
echo "Don't forget to push the changes to the remote repository."

# Ask for confirmation before pushing
echo "Do you want to push the changes? (y/n)"
read -r answer
if [ "$answer" != "y" ]; then
    echo "Changes not pushed. Exiting."
    exit 0
fi
git push
echo "Changes pushed successfully."