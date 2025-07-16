#[derive(Debug, Clone, PartialEq, Eq)]
pub enum View {
    RepositoryList,
    RepositoryActions(usize, usize), // repository index, selected action index
    CommandRunning(usize, String),   // repository index, command name
    CommandOutput(usize, String, String), // repository index, command name, output
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitAction {
    Status,
    Push,
    Fetch,
    Pull,
    Back,
}

impl GitAction {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Status => "📋 See status",
            Self::Push => "📤 Push",
            Self::Fetch => "📥 Fetch",
            Self::Pull => "⬇️ Pull",
            Self::Back => "🔙 Back to repository list",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            Self::Status,
            Self::Push,
            Self::Fetch,
            Self::Pull,
            Self::Back,
        ]
    }
}
