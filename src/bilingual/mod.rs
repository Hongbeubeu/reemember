mod parser;
pub mod repository;

pub use parser::{BilingualArticleInput, parse_bilingual_json};
pub use repository::BilingualRepository;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BilingualSegment {
    pub en: String,
    pub vi: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BilingualStructure {
    pub pattern: String,
    pub example: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BilingualArticle {
    pub id: i64,
    pub title: String,
    pub book: String,
    pub level: Option<String>,
    pub paragraphs: Vec<Vec<BilingualSegment>>,
    pub structures: Vec<BilingualStructure>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BilingualArticleSummary {
    pub id: i64,
    pub title: String,
    pub book: String,
    pub level: Option<String>,
    pub paragraph_count: usize,
    pub structure_count: usize,
    pub created_at: String,
}
