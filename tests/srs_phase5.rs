use chrono::{Duration, Utc};
use reemember::db::init_memory_db;
use reemember::model::{Definition, Metadata, WordRecord};
use reemember::repository::WordRepository;
use reemember::service::{TestMode, VocabularyService};
use reemember::testing::QuestionDirection;

fn seed(repo: &WordRepository, word: &str, meaning: &str) {
    let record = WordRecord {
        word: word.to_string(),
        phonetic: Some("/p/".to_string()),
        definitions: vec![Definition {
            pos: Some("noun".to_string()),
            meaning: meaning.to_string(),
        }],
        examples: vec![format!("{} example sentence.", word)],
        synonyms: vec![],
        antonyms: vec![],
        family_words: vec![],
        metadata: Metadata {
            tags: vec![],
            created_at: Some("2026-04-21T00:00:00Z".to_string()),
            review_count: 0,
        },
    };

    repo.upsert(&record).unwrap();
}

#[test]
fn srs_enabled_prioritizes_due_word() {
    let db = init_memory_db().unwrap();
    let repo = WordRepository::new(db);

    seed(&repo, "Alpha", "nghia a");
    seed(&repo, "Beta", "nghia b");

    let past = (Utc::now() - Duration::minutes(1)).to_rfc3339();
    let future = (Utc::now() + Duration::days(2)).to_rfc3339();

    repo.set_due_at_by_word_key("alpha", &past).unwrap();
    repo.set_due_at_by_word_key("beta", &future).unwrap();

    let question = VocabularyService::next_question_with_srs(&repo, TestMode::EnVi, true)
        .unwrap()
        .unwrap();

    assert_eq!(question.direction, QuestionDirection::EnVi);
    assert_eq!(question.word.as_deref(), Some("Alpha"));
}

#[test]
fn srs_toggle_off_still_returns_question() {
    let db = init_memory_db().unwrap();
    let repo = WordRepository::new(db);

    seed(&repo, "Gamma", "nghia g");

    let question = VocabularyService::next_question_with_srs(&repo, TestMode::Hybrid, false)
        .unwrap()
        .unwrap();
    assert!(!question.word_key.is_empty());
}

#[test]
fn submit_with_srs_records_history_and_due() {
    let db = init_memory_db().unwrap();
    let repo = WordRepository::new(db);

    seed(&repo, "Delta", "nghia d");
    let before_history = repo.review_history_count_by_word_key("delta").unwrap();

    let question = VocabularyService::next_question_with_srs(&repo, TestMode::EnVi, true)
        .unwrap()
        .unwrap();
    let result = VocabularyService::submit_answer_with_srs(&repo, &question, "wrong", true).unwrap();

    assert!(!result.correct);

    let after_history = repo.review_history_count_by_word_key("delta").unwrap();
    assert_eq!(after_history, before_history + 1);

    let due = repo.get_due_at_by_word_key("delta").unwrap();
    assert!(due.is_some());

    let review_count = repo
        .get_by_word_key("delta")
        .unwrap()
        .unwrap()
        .metadata
        .review_count;
    assert_eq!(review_count, 1);
}

#[test]
fn submit_without_srs_records_history_but_does_not_reschedule() {
    let db = init_memory_db().unwrap();
    let repo = WordRepository::new(db);

    seed(&repo, "Echo", "nghia e");
    let initial_due = repo.get_due_at_by_word_key("echo").unwrap().unwrap();

    let question = VocabularyService::next_question_with_srs(&repo, TestMode::EnVi, false)
        .unwrap()
        .unwrap();
    let _ = VocabularyService::submit_answer_with_srs(&repo, &question, "nghia e", false).unwrap();

    let next_due = repo.get_due_at_by_word_key("echo").unwrap().unwrap();
    assert_eq!(initial_due, next_due);

    let history_count = repo.review_history_count_by_word_key("echo").unwrap();
    assert_eq!(history_count, 1);
}

