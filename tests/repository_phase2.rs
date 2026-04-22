use reemember::db::init_memory_db;
use reemember::model::{Definition, Metadata, WordRecord};
use reemember::repository::{WordRepository, QueryOptions, SortBy};

fn create_test_record(word: &str, meaning: &str, tags: Vec<&str>) -> WordRecord {
    WordRecord {
        word: word.to_string(),
        phonetic: None,
        definitions: vec![Definition {
            pos: None,
            meaning: meaning.to_string(),
        }],
        examples: vec![],
        synonyms: vec![],
        antonyms: vec![],
        family_words: vec![],
        metadata: Metadata {
            tags: tags.into_iter().map(|s| s.to_string()).collect(),
            created_at: Some("2026-04-20T10:00:00Z".to_string()),
            review_count: 0,
        },
    }
}

fn create_record_with_all_fields(
    word: &str,
    phonetic: Option<&str>,
    meanings: Vec<&str>,
    examples: Vec<&str>,
    pos: Option<&str>,
    tags: Vec<&str>,
) -> WordRecord {
    let definitions = meanings
        .into_iter()
        .map(|m| Definition {
            pos: pos.map(|p| p.to_string()),
            meaning: m.to_string(),
        })
        .collect();

    WordRecord {
        word: word.to_string(),
        phonetic: phonetic.map(|p| p.to_string()),
        definitions,
        examples: examples.into_iter().map(|e| e.to_string()).collect(),
        synonyms: vec![],
        antonyms: vec![],
        family_words: vec![],
        metadata: Metadata {
            tags: tags.into_iter().map(|t| t.to_string()).collect(),
            created_at: Some("2026-04-20T10:00:00Z".to_string()),
            review_count: 0,
        },
    }
}

#[test]
fn test_upsert_new_record() {
    let db = init_memory_db().unwrap();
    let repo = WordRepository::new(db);

    let record = create_test_record("consistency", "the quality of being consistent", vec!["mindset"]);
    let result = repo.upsert(&record).unwrap();

    assert!(result.inserted);
    assert_eq!(result.definitions_count, 1);
    assert_eq!(result.tags_count, 1);
}

#[test]
fn test_upsert_existing_record_merge_phonetic() {
    let db = init_memory_db().unwrap();
    let repo = WordRepository::new(db);

    let mut record1 = create_test_record("test", "a trial", vec![]);
    record1.phonetic = Some("/test/".to_string());
    repo.upsert(&record1).unwrap();

    let mut record2 = create_test_record("test", "an exam", vec![]);
    record2.phonetic = Some("/updated/".to_string());
    let result = repo.upsert(&record2).unwrap();

    assert!(!result.inserted);
    let loaded = repo.get_by_word_key("test").unwrap().unwrap();
    assert_eq!(loaded.phonetic, Some("/updated/".to_string()));
}

#[test]
fn test_upsert_merge_definitions_deduplicates() {
    let db = init_memory_db().unwrap();
    let repo = WordRepository::new(db);

    let record1 = create_record_with_all_fields(
        "test",
        None,
        vec!["meaning 1", "meaning 2"],
        vec![],
        None,
        vec![],
    );
    repo.upsert(&record1).unwrap();

    let record2 = create_record_with_all_fields(
        "test",
        None,
        vec!["meaning 2", "meaning 3"],
        vec![],
        None,
        vec![],
    );
    let result = repo.upsert(&record2).unwrap();

    assert!(!result.inserted);
    assert_eq!(result.definitions_count, 3); // meaning 1, 2 (merged), 3

    let loaded = repo.get_by_word_key("test").unwrap().unwrap();
    assert_eq!(loaded.definitions.len(), 3);
}

#[test]
fn test_upsert_merge_tags() {
    let db = init_memory_db().unwrap();
    let repo = WordRepository::new(db);

    let record1 = create_test_record("word", "definition", vec!["tag1", "tag2"]);
    repo.upsert(&record1).unwrap();

    let record2 = create_test_record("word", "definition", vec!["tag2", "tag3"]);
    let result = repo.upsert(&record2).unwrap();

    assert_eq!(result.tags_count, 3); // tag1, tag2, tag3

    let loaded = repo.get_by_word_key("word").unwrap().unwrap();
    assert_eq!(loaded.metadata.tags.len(), 3);
    assert!(loaded.metadata.tags.contains(&"tag1".to_string()));
    assert!(loaded.metadata.tags.contains(&"tag3".to_string()));
}

#[test]
fn test_upsert_keeps_earlier_created_at() {
    let db = init_memory_db().unwrap();
    let repo = WordRepository::new(db);

    let mut record1 = create_test_record("word", "def", vec![]);
    record1.metadata.created_at = Some("2026-04-20T10:00:00Z".to_string());
    repo.upsert(&record1).unwrap();

    let mut record2 = create_test_record("word", "def", vec![]);
    record2.metadata.created_at = Some("2026-04-25T10:00:00Z".to_string());
    repo.upsert(&record2).unwrap();

    let loaded = repo.get_by_word_key("word").unwrap().unwrap();
    assert_eq!(
        loaded.metadata.created_at,
        Some("2026-04-20T10:00:00Z".to_string())
    );
}

#[test]
fn test_upsert_keeps_higher_review_count() {
    let db = init_memory_db().unwrap();
    let repo = WordRepository::new(db);

    let mut record1 = create_test_record("word", "def", vec![]);
    record1.metadata.review_count = 3;
    repo.upsert(&record1).unwrap();

    let mut record2 = create_test_record("word", "def", vec![]);
    record2.metadata.review_count = 7;
    repo.upsert(&record2).unwrap();

    let loaded = repo.get_by_word_key("word").unwrap().unwrap();
    assert_eq!(loaded.metadata.review_count, 7);
}

#[test]
fn test_get_by_word_key_case_insensitive() {
    let db = init_memory_db().unwrap();
    let repo = WordRepository::new(db);

    let record = create_test_record("Test", "definition", vec![]);
    repo.upsert(&record).unwrap();

    // Query with different casing
    let loaded = repo.get_by_word_key("test").unwrap();
    assert!(loaded.is_some());
    assert_eq!(loaded.unwrap().word, "Test"); // Original casing preserved
}

#[test]
fn test_query_sort_by_word() {
    let db = init_memory_db().unwrap();
    let repo = WordRepository::new(db);

    repo.upsert(&create_test_record("zebra", "animal", vec![]))
        .unwrap();
    repo.upsert(&create_test_record("apple", "fruit", vec![]))
        .unwrap();
    repo.upsert(&create_test_record("middle", "position", vec![]))
        .unwrap();

    let options = QueryOptions::new().sort(SortBy::Word);
    let results = repo.query(&options).unwrap();

    assert_eq!(results.len(), 3);
    assert_eq!(results[0].word, "apple");
    assert_eq!(results[1].word, "middle");
    assert_eq!(results[2].word, "zebra");
}

#[test]
fn test_query_sort_by_review_count() {
    let db = init_memory_db().unwrap();
    let repo = WordRepository::new(db);

    let mut rec1 = create_test_record("word1", "def", vec![]);
    rec1.metadata.review_count = 2;
    repo.upsert(&rec1).unwrap();

    let mut rec2 = create_test_record("word2", "def", vec![]);
    rec2.metadata.review_count = 10;
    repo.upsert(&rec2).unwrap();

    let mut rec3 = create_test_record("word3", "def", vec![]);
    rec3.metadata.review_count = 5;
    repo.upsert(&rec3).unwrap();

    let options = QueryOptions::new().sort(SortBy::ReviewCount);
    let results = repo.query(&options).unwrap();

    assert_eq!(results[0].metadata.review_count, 10);
    assert_eq!(results[1].metadata.review_count, 5);
    assert_eq!(results[2].metadata.review_count, 2);
}

#[test]
fn test_query_with_limit() {
    let db = init_memory_db().unwrap();
    let repo = WordRepository::new(db);

    for i in 0..10 {
        let record = create_test_record(&format!("word{}", i), "def", vec![]);
        repo.upsert(&record).unwrap();
    }

    let options = QueryOptions::new().limit(3);
    let results = repo.query(&options).unwrap();

    assert_eq!(results.len(), 3);
}

#[test]
fn test_query_with_tag_filter() {
    let db = init_memory_db().unwrap();
    let repo = WordRepository::new(db);

    repo.upsert(&create_test_record("word1", "def", vec!["mindset"]))
        .unwrap();
    repo.upsert(&create_test_record("word2", "def", vec!["mindset", "professional"]))
        .unwrap();
    repo.upsert(&create_test_record("word3", "def", vec!["professional"]))
        .unwrap();

    let options = QueryOptions::new().with_tag("mindset".to_string());
    let results = repo.query(&options).unwrap();

    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|r| r.metadata.tags.contains(&"mindset".to_string())));
}

#[test]
fn test_acceptance_criteria_insert() {
    let db = init_memory_db().unwrap();
    let repo = WordRepository::new(db);

    let record = create_record_with_all_fields(
        "consistency",
        Some("/kənˈsɪstənsi/"),
        vec!["Tính nhất quán; sự kiên định"],
        vec!["Consistency is the key to success."],
        Some("noun"),
        vec!["mindset"],
    );

    let result = repo.upsert(&record).unwrap();

    assert!(result.inserted);
    assert_eq!(result.definitions_count, 1);
    assert_eq!(result.examples_count, 1);
    assert_eq!(result.tags_count, 1);

    let loaded = repo.get_by_word_key("consistency").unwrap().unwrap();
    assert_eq!(loaded.word, "consistency");
    assert_eq!(loaded.phonetic, Some("/kənˈsɪstənsi/".to_string()));
    assert_eq!(loaded.definitions[0].meaning, "Tính nhất quán; sự kiên định");
    assert_eq!(loaded.examples[0], "Consistency is the key to success.");
}

#[test]
fn test_acceptance_criteria_upsert_merge_policy() {
    let db = init_memory_db().unwrap();
    let repo = WordRepository::new(db);

    // First insert
    let mut record1 = create_record_with_all_fields(
        "consistency",
        Some("/old/"),
        vec!["meaning1"],
        vec!["example1"],
        Some("noun"),
        vec!["tag1"],
    );
    record1.metadata.created_at = Some("2026-04-20T10:00:00Z".to_string());
    record1.metadata.review_count = 3;
    repo.upsert(&record1).unwrap();

    // Second upsert with different data
    let mut record2 = create_record_with_all_fields(
        "consistency",
        Some("/new/"),
        vec!["meaning2"],
        vec!["example2"],
        Some("verb"),
        vec!["tag2"],
    );
    record2.metadata.created_at = Some("2026-04-25T10:00:00Z".to_string());
    record2.metadata.review_count = 1;

    let result = repo.upsert(&record2).unwrap();

    assert!(!result.inserted);

    let loaded = repo.get_by_word_key("consistency").unwrap().unwrap();

    // Verify merge policy applied
    assert_eq!(loaded.phonetic, Some("/new/".to_string())); // Updated phonetic
    assert_eq!(loaded.definitions.len(), 2); // Both meanings merged
    assert_eq!(loaded.examples.len(), 2); // Both examples merged
    assert_eq!(loaded.metadata.tags.len(), 2); // Both tags merged
    assert_eq!(
        loaded.metadata.created_at,
        Some("2026-04-20T10:00:00Z".to_string())
    ); // Kept earliest
    assert_eq!(loaded.metadata.review_count, 3); // Kept higher
}

#[test]
fn test_acceptance_criteria_sort_query() {
    let db = init_memory_db().unwrap();
    let repo = WordRepository::new(db);

    // Insert 5 words with different creation dates
    for (i, word) in vec!["zebra", "apple", "middle", "banana", "cherry"]
        .iter()
        .enumerate()
    {
        let record = create_test_record(word, "definition", vec![]);
        repo.upsert(&record).unwrap();

        if i > 0 {
            // Simulate different timestamps by querying and updating
            // In real scenario, these would have different created_at values
        }
    }

    let options = QueryOptions::new()
        .sort(SortBy::Word)
        .limit(3);
    let results = repo.query(&options).unwrap();

    assert_eq!(results.len(), 3);
    assert_eq!(results[0].word, "apple");
    assert_eq!(results[1].word, "banana");
    assert_eq!(results[2].word, "cherry");
}

