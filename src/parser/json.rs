use crate::model::WordRecord;
use serde::Deserialize;

use super::ParseError;

/// Bundle format: a collection/topic wrapper around an array of words.
#[derive(Debug, Deserialize)]
pub struct BundleImport {
    pub collection: Option<String>,
    pub topic: Option<String>,
    pub words: Vec<WordRecord>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ImportPayload {
    Array(Vec<WordRecord>),
    Bundle(BundleImport),
}

/// Parse JSON (flat array or bundle) into a list of WordRecords.
pub fn parse_json_canonical(input: &str) -> Result<Vec<WordRecord>, ParseError> {
    let bundle = parse_json_bundle(input)?;
    Ok(bundle.words)
}

/// Parse JSON into a BundleImport, normalizing flat arrays into a bundle with no collection/topic.
pub fn parse_json_bundle(input: &str) -> Result<BundleImport, ParseError> {
    let payload: ImportPayload = serde_json::from_str(input)?;
    let bundle = match payload {
        ImportPayload::Array(words) => BundleImport { collection: None, topic: None, words },
        ImportPayload::Bundle(b) => b,
    };

    for (idx, record) in bundle.words.iter().enumerate() {
        record.validate()
            .map_err(|e| ParseError::InvalidData(format!("record {}: {}", idx + 1, e)))?;
    }

    Ok(bundle)
}
