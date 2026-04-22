use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WordRecord {
    pub word: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phonetic: Option<String>,
    pub definitions: Vec<Definition>,
    #[serde(default)]
    pub examples: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub synonyms: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub antonyms: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub family_words: Vec<String>,
    #[serde(default)]
    pub metadata: Metadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Definition {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pos: Option<String>,
    pub meaning: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Metadata {
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(default)]
    pub review_count: u32,
}

/// A named collection of words (top-level group, e.g. "1000 Common Words").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Collection {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
}

/// A topic inside a collection (sub-group, e.g. "Food & Drink").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Topic {
    pub id: i64,
    pub collection_id: i64,
    pub name: String,
    pub description: Option<String>,
}

impl WordRecord {
    pub fn word_key(&self) -> String {
        normalize_key(&self.word)
    }

    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.word.trim().is_empty() {
            return Err(ValidationError::MissingWord);
        }
        if self.definitions.is_empty() {
            return Err(ValidationError::MissingDefinition);
        }
        if self.definitions.iter().all(|def| def.meaning.trim().is_empty()) {
            return Err(ValidationError::MissingMeaning);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    MissingWord,
    MissingDefinition,
    MissingMeaning,
}

impl core::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ValidationError::MissingWord       => write!(f, "missing required field: word"),
            ValidationError::MissingDefinition => write!(f, "missing required field: definitions"),
            ValidationError::MissingMeaning    => write!(f, "missing required field: definitions[].meaning"),
        }
    }
}

impl std::error::Error for ValidationError {}

pub fn normalize_key(value: &str) -> String {
    value.trim().to_lowercase()
}
