mod csv;
mod json;

pub use csv::{parse_csv, CsvImportReport, CsvRowError};
pub use json::{parse_json_canonical, parse_json_bundle, BundleImport};

use thiserror::Error;

/// ParseError represents all possible errors that can occur during data parsing.
/// This includes both format-level errors (invalid JSON/CSV syntax) and
/// semantic-level errors (missing required fields, invalid values).
#[derive(Debug, Error)]
pub enum ParseError {
    /// Error parsing JSON syntax (e.g., malformed JSON, unexpected structure).
    /// The error details are provided by serde_json.
    #[error("json parse error: {0}")]
    Json(#[from] serde_json::Error),

    /// Error reading CSV format (e.g., mismatched column count, encoding issues).
    /// The error details are provided by the csv crate.
    #[error("csv parse error: {0}")]
    Csv(#[from] ::csv::Error),

    /// Semantic validation error (e.g., missing required fields, invalid field values).
    /// This includes missing CSV headers, invalid data in cells, etc.
    #[error("invalid data: {0}")]
    InvalidData(String),
}


