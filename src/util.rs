use std::path::Path;

use anyhow::Context as _;
use log::LevelFilter;
use simplelog::{ColorChoice, ConfigBuilder, TermLogger, TerminalMode};

/// Initializes the logger for the application.
///
/// # Errors
/// Returns an error if logger initialization fails.
pub fn initialize_logger() -> anyhow::Result<()> {
    TermLogger::init(
        #[cfg(debug_assertions)]
        LevelFilter::max(),
        #[cfg(not(debug_assertions))]
        LevelFilter::Info,
        ConfigBuilder::new()
            .add_filter_allow_str("git_statuses")
            .build(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .context("Failed to initialize logger")
}

/// Extension trait for working with Git repository paths.
pub trait GitPathExt {
    /// Checks if the path is a Git repository directory.
    ///
    /// This checks if the directory exists and contains a `.git` subdirectory.
    ///
    /// # Returns
    ///
    /// `true` if the path is a Git repository, `false` otherwise.
    fn is_git_directory(&self) -> bool;

    /// Extracts the repository name from the path.
    ///
    /// # Returns
    ///
    /// The final component of the path (i.e., the directory name) as a `String`,
    /// which typically corresponds to the repository name. Returns `"unknown"` if
    /// the path has no final component or cannot be converted to a valid UTF-8 string.
    fn dir_name(&self) -> String;
}

impl GitPathExt for Path {
    fn is_git_directory(&self) -> bool {
        self.is_dir() && self.join(".git").exists()
    }

    fn dir_name(&self) -> String {
        self.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_owned()
    }
}
