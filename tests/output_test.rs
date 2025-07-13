use std::str::FromStr;

use git_statuses::output::OutputFormat;

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