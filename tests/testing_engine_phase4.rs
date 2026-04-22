use reemember::db::init_memory_db;
use reemember::model::{Definition, Metadata, WordRecord};
use reemember::repository::WordRepository;
use reemember::service::{TestMode, VocabularyService};
use reemember::testing::QuestionDirection;

fn seed_word(repo: &WordRepository, word: &str, meaning: &str, examples: &[&str]) {
    let record = WordRecord {
        word: word.to_string(),
        phonetic: Some("/sample/".to_string()),
        definitions: vec![Definition {
            pos: Some("noun".to_string()),
            meaning: meaning.to_string(),
        }],
        examples: examples.iter().map(|v| v.to_string()).collect(),
        synonyms: vec![],
        antonyms: vec![],
        family_words: vec![],
        metadata: Metadata {
            tags: vec!["test".to_string()],
            created_at: Some("2026-04-21T00:00:00Z".to_string()),
            review_count: 0,
        },
    };

    repo.upsert(&record).expect("seed data should upsert");
}

#[test]
fn en_vi_mode_generates_question_and_accepts_matching_meaning() {
    let db = init_memory_db().unwrap();
    let repo = WordRepository::new(db);
    seed_word(
        &repo,
        "Consistency",
        "Tính nhất quán; sự kiên định",
        &["Consistency is the key to success."],
    );

    let question = VocabularyService::next_question(&repo, TestMode::EnVi)
        .unwrap()
        .expect("question should exist");

    assert_eq!(question.direction, QuestionDirection::EnVi);
    assert!(question.prompt.contains("Vietnamese meaning"));
    assert!(question.examples[0].contains("____"));

    let result = VocabularyService::submit_answer(&repo, &question, "  tính   nhất  quán ").unwrap();
    assert!(result.correct);
}

#[test]
fn vi_en_mode_generates_question_and_accepts_matching_word() {
    let db = init_memory_db().unwrap();
    let repo = WordRepository::new(db);
    seed_word(&repo, "Resilient", "kiên cường", &[]);

    let question = VocabularyService::next_question(&repo, TestMode::ViEn)
        .unwrap()
        .expect("question should exist");

    assert_eq!(question.direction, QuestionDirection::ViEn);
    assert!(question.prompt.contains("English word"));

    let result = VocabularyService::submit_answer(&repo, &question, " resilient ").unwrap();
    assert!(result.correct);
}

#[test]
fn hybrid_mode_returns_supported_directions() {
    let db = init_memory_db().unwrap();
    let repo = WordRepository::new(db);
    seed_word(&repo, "Focus", "tập trung", &[]);

    let question = VocabularyService::next_question(&repo, TestMode::Hybrid)
        .unwrap()
        .expect("question should exist");

    assert!(
        question.direction == QuestionDirection::EnVi || question.direction == QuestionDirection::ViEn
    );
}

#[test]
fn wrong_answer_increments_review_count() {
    let db = init_memory_db().unwrap();
    let repo = WordRepository::new(db);
    seed_word(&repo, "Discipline", "kỷ luật", &[]);

    let before = repo
        .get_by_word_key("discipline")
        .unwrap()
        .expect("record should exist")
        .metadata
        .review_count;

    let question = VocabularyService::next_question(&repo, TestMode::EnVi)
        .unwrap()
        .expect("question should exist");

    let result = VocabularyService::submit_answer(&repo, &question, "wrong answer").unwrap();
    assert!(!result.correct);

    let after = repo
        .get_by_word_key("discipline")
        .unwrap()
        .expect("record should exist")
        .metadata
        .review_count;

    assert_eq!(after, before + 1);
}

#[test]
fn correct_answer_does_not_increment_review_count() {
    let db = init_memory_db().unwrap();
    let repo = WordRepository::new(db);
    seed_word(&repo, "Patience", "kiên nhẫn", &[]);

    let before = repo
        .get_by_word_key("patience")
        .unwrap()
        .expect("record should exist")
        .metadata
        .review_count;

    let question = VocabularyService::next_question(&repo, TestMode::EnVi)
        .unwrap()
        .expect("question should exist");

    let result = VocabularyService::submit_answer(&repo, &question, "kiên nhẫn").unwrap();
    assert!(result.correct);

    let after = repo
        .get_by_word_key("patience")
        .unwrap()
        .expect("record should exist")
        .metadata
        .review_count;

    assert_eq!(after, before);
}

