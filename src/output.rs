use std::str::FromStr;

use anyhow::{bail, Result};
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter};

/// Supported output formats for displaying repository information.
#[derive(Debug, Clone, PartialEq, Eq, Display, EnumIter)]
#[strum(serialize_all = "lowercase")]
pub enum OutputFormat {
    /// Table format (default) - displays data in a human-readable table
    Table,
    /// JSON format - outputs structured JSON data
    Json,
    /// HTML format - generates an HTML table
    Html,
}

impl Default for OutputFormat {
    fn default() -> Self {
        Self::Table
    }
}

impl FromStr for OutputFormat {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "table" => Ok(Self::Table),
            "json" => Ok(Self::Json),
            "html" => Ok(Self::Html),
            _ => {
                let valid_formats: Vec<String> = Self::iter()
                    .map(|f| f.to_string())
                    .collect();
                bail!(
                    "Unsupported output format '{}'. Valid formats: {}",
                    s,
                    valid_formats.join(", ")
                )
            }
        }
    }
}

impl OutputFormat {
    /// Returns true if this format supports file output
    pub fn supports_file_output(&self) -> bool {
        matches!(self, Self::Json | Self::Html)
    }

    /// Returns the default file extension for this format
    pub fn default_extension(&self) -> Option<&'static str> {
        match self {
            Self::Table => None,
            Self::Json => Some("json"),
            Self::Html => Some("html"),
        }
    }
}