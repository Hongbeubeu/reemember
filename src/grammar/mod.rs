mod parser;
pub mod repository;

pub use parser::{parse_grammar_json, parse_grammar_md, GrammarDocInput};
pub use repository::GrammarRepository;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrammarDoc {
    pub id: i64,
    pub title: String,
    pub category: Option<String>,
    pub level: Option<String>,
    pub content: String,
    pub examples: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrammarDocSummary {
    pub id: i64,
    pub title: String,
    pub category: Option<String>,
    pub level: Option<String>,
    pub exercise_count: usize,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrammarExercise {
    pub id: i64,
    pub doc_id: i64,
    pub order_index: i32,
    pub exercise_type: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrammarDocDetail {
    pub doc: GrammarDoc,
    pub exercises: Vec<GrammarExercise>,
}
