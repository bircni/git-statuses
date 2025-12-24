use std::fmt::{self, Display, Formatter};

use comfy_table::Cell;
use git2::{Repository, RepositoryState, StatusOptions};
use strum_macros::EnumIter;

use crate::gitinfo;

/// Represents the status of a Git repository.
#[derive(Default, Clone, Debug, PartialEq, Eq, EnumIter, serde::Serialize, serde::Deserialize)]
pub enum Status {
    /// The repository is clean, with no changes or untracked files.
    Clean,
    /// The repository has changes or untracked files.
    Dirty(usize), // Number of untracked files
    /// The repository is in a merge state.
    Merge,
    /// The repository is in a revert state.
    Revert,
    /// The repository is in a rebase state.
    Rebase,
    /// The repository is in a bisect state.
    Bisect,
    /// The repository is in a cherry-pick state.
    CherryPick,
    /// Unpushed commits or changes are present.
    Unpushed,
    /// The branch is not published.
    Unpublished,
    /// The repository is in a detached HEAD state or has no upstream branch.
    Detached,
    /// The status of the repository is unknown or not recognized.
    #[default]
    Unknown,
}

impl Status {
    /// Returns the `Status` of the repository.
    /// # Arguments
    /// * `repo` - The Git repository to check the status of.
    /// # Returns
    /// A `Status` enum indicating the state of the repository:
    /// * `Clean` - No changes, no untracked files.
    /// * `Dirty` - There are changes or untracked files.
    pub fn new(repo: &Repository) -> Self {
        // Step 1: Handle explicit git states
        match repo.state() {
            RepositoryState::Clean => {}
            RepositoryState::Merge => return Self::Merge,
            RepositoryState::Revert | RepositoryState::RevertSequence => return Self::Revert,
            RepositoryState::CherryPick | RepositoryState::CherryPickSequence => {
                return Self::CherryPick;
            }
            RepositoryState::Bisect => return Self::Bisect,
            RepositoryState::Rebase
            | RepositoryState::RebaseInteractive
            | RepositoryState::RebaseMerge => return Self::Rebase,
            RepositoryState::ApplyMailbox | RepositoryState::ApplyMailboxOrRebase => {
                return Self::Unknown;
            }
        }

        // Step 2: Check working directory status
        let mut opts = StatusOptions::new();
        opts.include_untracked(true).include_ignored(false);

        repo.statuses(Some(&mut opts))
            .map_or(Self::Unknown, |statuses| {
                if statuses.iter().all(|e| {
                    e.status().is_ignored()
                        || !e.status().intersects(
                            git2::Status::WT_NEW
                                | git2::Status::WT_MODIFIED
                                | git2::Status::WT_DELETED
                                | git2::Status::INDEX_NEW
                                | git2::Status::INDEX_MODIFIED
                                | git2::Status::INDEX_DELETED
                                | git2::Status::CONFLICTED,
                        )
                }) {
                    // Clean working directory – check branch push state
                    gitinfo::get_branch_push_status(repo)
                } else {
                    // Dirty working directory – report how many changes
                    Self::Dirty(gitinfo::get_changed_count(repo))
                }
            })
    }

    /// Get the color associated with the status.
    /// This is used for terminal output to visually distinguish different statuses.
    pub const fn comfy_color(&self) -> comfy_table::Color {
        use comfy_table::Color;
        match self {
            Self::Clean => Color::Reset,
            Self::Dirty(_) | Self::Unpushed | Self::Unpublished => Color::Red,
            Self::Merge => Color::Blue,
            Self::Revert => Color::Magenta,
            Self::Rebase => Color::Cyan,
            Self::Bisect => Color::Yellow,
            Self::CherryPick => Color::DarkYellow,
            Self::Detached =>
            // Purple color for detached HEAD state
            {
                Color::Rgb {
                    r: 255,
                    g: 0,
                    b: 255,
                }
            }
            Self::Unknown =>
            // Orange color for unknown status
            {
                Color::Rgb {
                    r: 255,
                    g: 165,
                    b: 0,
                }
            }
        }
    }

    /// Converts the status to a `Cell` for use in a table.
    /// This allows the status to be displayed with its associated color and attributes.
    pub fn as_cell(&self) -> Cell {
        Cell::new(self.to_string())
            .fg(self.comfy_color())
            .add_attribute(comfy_table::Attribute::Bold)
    }

    /// Gets a description of the status.
    /// This provides a human-readable explanation of what the status means.
    pub const fn description(&self) -> &str {
        match self {
            Self::Clean => "No changes, no unpushed commits.",
            Self::Detached => {
                "The repository is in a detached HEAD state or has no upstream branch."
            }
            Self::Dirty(_) => "Working directory has changes.",
            Self::Merge => "Merge in progress.",
            Self::Revert => "Revert in progress.",
            Self::Rebase => "Rebase in progress.",
            Self::Bisect => "Bisecting in progress.",
            Self::CherryPick => "Cherry-pick in progress.",
            Self::Unpublished => "The branch is not published.",
            Self::Unpushed => "There are unpushed commits.",
            Self::Unknown => "Status is unknown or not recognized.",
        }
    }
}

impl Display for Status {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Clean => write!(f, "Clean"),
            Self::Detached => write!(f, "Detached"),
            Self::Dirty(count) => write!(f, "Dirty ({count})"),
            Self::Merge => write!(f, "Merge"),
            Self::Revert => write!(f, "Revert"),
            Self::Rebase => write!(f, "Rebase"),
            Self::Bisect => write!(f, "Bisect"),
            Self::CherryPick => write!(f, "Cherry Pick"),
            Self::Unpushed => write!(f, "Unpushed"),
            Self::Unpublished => write!(f, "Unpublished"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}
