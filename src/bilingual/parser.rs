use crate::bilingual::{BilingualSegment, BilingualStructure};
use crate::parser::ParseError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BilingualArticleInput {
    pub title: String,
    #[serde(default = "default_book")]
    pub book: String,
    #[serde(default)]
    pub level: Option<String>,
    #[serde(default)]
    pub paragraphs: Vec<Vec<BilingualSegment>>,
    #[serde(default)]
    pub structures: Vec<BilingualStructure>,
}

pub fn parse_bilingual_json(input: &str) -> Result<Vec<BilingualArticleInput>, ParseError> {
    let mut articles: Vec<BilingualArticleInput> = serde_json::from_str(input)?;
    if articles.is_empty() {
        return Err(ParseError::InvalidData(
            "bilingual import must contain at least one article".into(),
        ));
    }

    for article in &mut articles {
        normalize_article(article)?;
    }

    Ok(articles)
}

fn normalize_article(article: &mut BilingualArticleInput) -> Result<(), ParseError> {
    article.title = required(article.title.as_str(), "title")?;
    article.book = required(article.book.as_str(), "book")?;
    article.level = article
        .level
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    if article.paragraphs.is_empty() {
        return Err(ParseError::InvalidData(format!(
            "{}: paragraphs must not be empty",
            article.title
        )));
    }

    for (paragraph_index, paragraph) in article.paragraphs.iter_mut().enumerate() {
        if paragraph.is_empty() {
            return Err(ParseError::InvalidData(format!(
                "{}: paragraph {} must contain at least one segment",
                article.title,
                paragraph_index + 1
            )));
        }

        for (segment_index, segment) in paragraph.iter_mut().enumerate() {
            segment.en = required(
                segment.en.as_str(),
                &format!(
                    "{}: paragraphs[{}][{}].en",
                    article.title, paragraph_index, segment_index
                ),
            )?;
            segment.vi = required(
                segment.vi.as_str(),
                &format!(
                    "{}: paragraphs[{}][{}].vi",
                    article.title, paragraph_index, segment_index
                ),
            )?;
        }
    }

    for (structure_index, structure) in article.structures.iter_mut().enumerate() {
        structure.pattern = required(
            structure.pattern.as_str(),
            &format!("{}: structures[{}].pattern", article.title, structure_index),
        )?;
        structure.example = required(
            structure.example.as_str(),
            &format!("{}: structures[{}].example", article.title, structure_index),
        )?;
        structure.note = structure
            .note
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
    }

    Ok(())
}

fn required(value: &str, field_name: &str) -> Result<String, ParseError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ParseError::InvalidData(format!(
            "{} is required",
            field_name
        )));
    }
    Ok(trimmed.to_string())
}

fn default_book() -> String {
    "Bilingual Books".to_string()
}
