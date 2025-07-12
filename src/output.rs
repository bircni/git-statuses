use std::str::FromStr;

use anyhow::{anyhow, Result};
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
                Err(anyhow!(
                    "Unsupported output format '{}'. Valid formats: {}",
                    s,
                    valid_formats.join(", ")
                ))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_format_from_str() {
        assert_eq!(OutputFormat::from_str("table").unwrap(), OutputFormat::Table);
        assert_eq!(OutputFormat::from_str("json").unwrap(), OutputFormat::Json);
        assert_eq!(OutputFormat::from_str("html").unwrap(), OutputFormat::Html);
        assert_eq!(OutputFormat::from_str("TABLE").unwrap(), OutputFormat::Table);
        assert_eq!(OutputFormat::from_str("Json").unwrap(), OutputFormat::Json);
        assert!(OutputFormat::from_str("csv").is_err());
        assert!(OutputFormat::from_str("invalid").is_err());
    }

    #[test]
    fn test_supports_file_output() {
        assert!(!OutputFormat::Table.supports_file_output());
        assert!(OutputFormat::Json.supports_file_output());
        assert!(OutputFormat::Html.supports_file_output());
    }

    #[test]
    fn test_default_extension() {
        assert_eq!(OutputFormat::Table.default_extension(), None);
        assert_eq!(OutputFormat::Json.default_extension(), Some("json"));
        assert_eq!(OutputFormat::Html.default_extension(), Some("html"));
    }
}