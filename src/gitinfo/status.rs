use std::fmt::{self, Display, Formatter};

use comfy_table::{Cell, Color};
use git2::{Repository, RepositoryState, StatusOptions};
use strum_macros::EnumIter;

use crate::gitinfo;

/// Represents the status of a Git repository.
#[derive(Default, Clone, Debug, PartialEq, Eq, EnumIter)]
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
        let mut state = match repo.state() {
            RepositoryState::Clean => Self::Clean,
            RepositoryState::Merge => Self::Merge,
            RepositoryState::Revert | RepositoryState::RevertSequence => Self::Revert,
            RepositoryState::CherryPick | RepositoryState::CherryPickSequence => Self::CherryPick,
            RepositoryState::Bisect => Self::Bisect,
            RepositoryState::Rebase
            | RepositoryState::RebaseInteractive
            | RepositoryState::RebaseMerge => Self::Rebase,
            RepositoryState::ApplyMailbox | RepositoryState::ApplyMailboxOrRebase => Self::Unknown,
        };
        if matches!(state, Self::Clean | Self::Unknown) {
            let mut opts = StatusOptions::new();
            opts.include_untracked(true).include_ignored(false);
            state = repo.statuses(Some(&mut opts)).map_or_else(
                |_| Self::Unknown,
                |statuses| {
                    if statuses.iter().all(|e| {
                        e.status().is_ignored()
                            || !e.status().is_wt_new()
                                && !e.status().is_index_new()
                                && !e.status().is_wt_modified()
                                && !e.status().is_index_modified()
                                && !e.status().is_wt_deleted()
                                && !e.status().is_index_deleted()
                                && !e.status().is_conflicted()
                    }) {
                        if gitinfo::get_ahead_behind(repo).0 == 0
                            && !gitinfo::is_current_branch_unpublished(repo)
                        {
                            Self::Clean
                        } else if gitinfo::is_current_branch_unpublished(repo) {
                            // If the branch is unpublished, we consider it unpublished
                            Self::Unpublished
                        } else {
                            // If there are unpushed commits, we consider it unpushed
                            Self::Unpushed
                        }
                    } else {
                        Self::Dirty(gitinfo::get_changed_count(repo))
                    }
                },
            );
        }

        state
    }

    /// Get the color associated with the status.
    /// This is used for terminal output to visually distinguish different statuses.
    pub const fn color(&self) -> Color {
        match self {
            Self::Clean => Color::Reset,
            Self::Dirty(_) | Self::Unpushed | Self::Unpublished => Color::Red,
            Self::Merge => Color::Blue,
            Self::Revert => Color::Magenta,
            Self::Rebase => Color::Cyan,
            Self::Bisect => Color::Yellow,
            Self::CherryPick => Color::DarkYellow,
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
            .fg(self.color())
            .add_attribute(comfy_table::Attribute::Bold)
    }

    /// Gets a description of the status.
    /// This provides a human-readable explanation of what the status means.
    pub const fn description(&self) -> &str {
        match self {
            Self::Clean => "No changes, no unpushed commits.",
            Self::Dirty(_) => "Working directory has changes.",
            Self::Merge => "Merge in progress.",
            Self::Revert => "Revert in progress.",
            Self::Rebase => "Rebase in progress.",
            Self::Bisect => "Bisecting in progress.",
            Self::CherryPick => "Cherry-pick in progress.",
            Self::Unpushed => "There are unpushed commits.",
            Self::Unpublished => "The branch is not published.",
            Self::Unknown => "Status is unknown or not recognized.",
        }
    }
}

impl Display for Status {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Clean => write!(f, "Clean"),
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
