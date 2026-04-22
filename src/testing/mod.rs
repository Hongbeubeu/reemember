/// Phase 4 testing engine for quiz-style vocabulary review.
/// Supports EN-VI, VI-EN, and Hybrid modes with normalized answer checking.

use crate::db::DbError;
use crate::model::WordRecord;
use crate::repository::WordRepository;
use crate::srs;
use chrono::Utc;

/// Quiz modes requested by the Phase 4 TRD.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestMode {
    EnVi,
    ViEn,
    Hybrid,
}

/// Actual direction of one generated question.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionDirection {
    EnVi,
    ViEn,
}

/// A generated test question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Question {
    pub word_key: String,
    pub direction: QuestionDirection,
    pub prompt: String,
    pub word: Option<String>,
    pub phonetic: Option<String>,
    pub examples: Vec<String>,
    pub expected_answers: Vec<String>,
    pub synonyms: Vec<String>,
    pub antonyms: Vec<String>,
    pub family_words: Vec<String>,
}

/// Result returned after user submits one answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnswerResult {
    pub correct: bool,
    pub normalized_answer: String,
    pub accepted_answers: Vec<String>,
}

/// Runtime options for testing sessions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TestingOptions {
    pub srs_enabled: bool,
    pub topic_id: Option<i64>,
}

impl Default for TestingOptions {
    fn default() -> Self {
        Self { srs_enabled: false, topic_id: None }
    }
}

/// Stateless engine methods for question generation and grading.
pub struct TestingEngine;

impl TestingEngine {
    /// Generate one question from repository data for a selected mode.
    /// Returns `Ok(None)` if no words exist yet.
    pub fn generate_question(repo: &WordRepository, mode: TestMode) -> Result<Option<Question>, DbError> {
        Self::generate_question_with_options(repo, mode, TestingOptions::default())
    }

    /// Generate one question with optional SRS due-first selection.
    pub fn generate_question_with_options(
        repo: &WordRepository,
        mode: TestMode,
        options: TestingOptions,
    ) -> Result<Option<Question>, DbError> {
        let now = Utc::now().to_rfc3339();
        let maybe_record = repo.pick_next_word_scoped(options.srs_enabled, &now, options.topic_id)?;
        let record = match maybe_record {
            Some(record) => record,
            None => return Ok(None),
        };

        let direction = match mode {
            TestMode::EnVi => QuestionDirection::EnVi,
            TestMode::ViEn => QuestionDirection::ViEn,
            TestMode::Hybrid => {
                if current_nanos_even() {
                    QuestionDirection::EnVi
                } else {
                    QuestionDirection::ViEn
                }
            }
        };

        Ok(Some(Self::build_question(direction, record)))
    }

    /// Grade one answer and update review_count on wrong answers.
    pub fn submit_answer(
        repo: &WordRepository,
        question: &Question,
        user_answer: &str,
    ) -> Result<AnswerResult, DbError> {
        Self::submit_answer_with_options(repo, question, user_answer, TestingOptions::default())
    }

    /// Grade one answer with optional SRS persistence.
    pub fn submit_answer_with_options(
        repo: &WordRepository,
        question: &Question,
        user_answer: &str,
        options: TestingOptions,
    ) -> Result<AnswerResult, DbError> {
        let normalized = normalize_answer_text(user_answer);
        let accepted_normalized = question
            .expected_answers
            .iter()
            .map(|s| normalize_answer_text(s))
            .collect::<Vec<_>>();

        let is_correct = accepted_normalized.iter().any(|candidate| candidate == &normalized);

        let before_count = repo
            .get_by_word_key(&question.word_key)?
            .map(|w| w.metadata.review_count)
            .unwrap_or(0);

        if !is_correct {
            repo.increment_review_count_by_word_key(&question.word_key)?;
        }

        let effective_count = if is_correct { before_count } else { before_count + 1 };
        let now = Utc::now();

        repo.record_review_event(&question.word_key, is_correct, &now.to_rfc3339())?;

        if options.srs_enabled {
            let due_at = srs::compute_next_due(now, is_correct, effective_count);
            repo.set_due_at_by_word_key(&question.word_key, &due_at)?;
        }

        Ok(AnswerResult {
            correct: is_correct,
            normalized_answer: normalized,
            accepted_answers: accepted_normalized,
        })
    }

    fn build_question(direction: QuestionDirection, record: WordRecord) -> Question {
        let synonyms = record.synonyms.clone();
        let antonyms = record.antonyms.clone();
        let family_words = record.family_words.clone();

        match direction {
            QuestionDirection::EnVi => {
                let masked_examples = record.examples.iter()
                    .map(|ex| mask_word_in_example(ex, &record.word))
                    .collect::<Vec<_>>();

                Question {
                    word_key: record.word_key(),
                    direction,
                    prompt: format!("Give the Vietnamese meaning of '{}'.", record.word),
                    word: Some(record.word.clone()),
                    phonetic: record.phonetic.clone(),
                    examples: masked_examples,
                    expected_answers: meaning_candidates(&record),
                    synonyms,
                    antonyms,
                    family_words,
                }
            }
            QuestionDirection::ViEn => {
                let prompt_meaning = meaning_candidates(&record)
                    .into_iter()
                    .next()
                    .unwrap_or_else(|| record.word.clone());

                Question {
                    word_key: record.word_key(),
                    direction,
                    prompt: format!("Which English word matches: '{}' ?", prompt_meaning),
                    word: None,
                    phonetic: None,
                    examples: vec![],
                    expected_answers: vec![record.word.clone()],
                    synonyms,
                    antonyms,
                    family_words,
                }
            }
        }
    }
}

/// Normalize free-text answers for strict MVP matching.
/// Rules: trim + lowercase + collapse all whitespace to one space.
pub fn normalize_answer_text(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn meaning_candidates(record: &WordRecord) -> Vec<String> {
    let mut values = vec![];
    for definition in &record.definitions {
        for part in definition.meaning.split(';') {
            let candidate = part.trim();
            if !candidate.is_empty() {
                values.push(candidate.to_string());
            }
        }
    }

    values
}

fn mask_word_in_example(example: &str, word: &str) -> String {
    if word.trim().is_empty() {
        return example.to_string();
    }

    let lower_word = word.to_lowercase();
    example
        .split_whitespace()
        .map(|token| {
            let clean = token
                .trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase();
            if clean == lower_word {
                token.replace(word, "____")
                    .replace(&word.to_lowercase(), "____")
                    .replace(&word.to_uppercase(), "____")
            } else {
                token.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn current_nanos_even() -> bool {
    use std::time::{SystemTime, UNIX_EPOCH};

    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.subsec_nanos() % 2 == 0,
        Err(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Definition, Metadata};

    fn sample_record() -> WordRecord {
        WordRecord {
            word: "Consistency".to_string(),
            phonetic: Some("/kənˈsɪstənsi/".to_string()),
            definitions: vec![Definition {
                pos: Some("noun".to_string()),
                meaning: "Tính nhất quán; sự kiên định".to_string(),
            }],
            examples: vec!["Consistency is the key to success.".to_string()],
            synonyms: vec!["steadiness".to_string()],
            antonyms: vec!["inconsistency".to_string()],
            family_words: vec!["consistent".to_string(), "consistently".to_string()],
            metadata: Metadata::default(),
        }
    }

    #[test]
    fn normalize_answer_text_collapses_spaces_and_lowercases() {
        let actual = normalize_answer_text("  Tinh   NHAT   Quan ");
        assert_eq!(actual, "tinh nhat quan");
    }

    #[test]
    fn meaning_candidates_split_semicolon_values() {
        let values = meaning_candidates(&sample_record());
        assert_eq!(values.len(), 2);
        assert_eq!(values[0], "Tính nhất quán");
        assert_eq!(values[1], "sự kiên định");
    }

    #[test]
    fn build_en_vi_question_masks_word_in_examples() {
        let question = TestingEngine::build_question(QuestionDirection::EnVi, sample_record());
        assert_eq!(question.direction, QuestionDirection::EnVi);
        assert!(question.examples[0].contains("____"));
    }

    #[test]
    fn build_vi_en_question_has_word_answer() {
        let question = TestingEngine::build_question(QuestionDirection::ViEn, sample_record());
        assert_eq!(question.direction, QuestionDirection::ViEn);
        assert_eq!(question.expected_answers, vec!["Consistency".to_string()]);
    }
}

