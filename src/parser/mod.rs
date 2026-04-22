mod json;

pub use json::{parse_json_canonical, parse_json_bundle, BundleImport};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("json parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("invalid data: {0}")]
    InvalidData(String),
}
