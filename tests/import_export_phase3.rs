use reemember::db::init_memory_db;
use reemember::repository::WordRepository;
use reemember::service::VocabularyService;

fn create_test_json() -> String {
    r#"[
{
  "word": "Consistency",
  "phonetic": "/kənˈsɪstənsi/",
  "definitions": [{"pos": "noun", "meaning": "Tính nhất quán"}],
  "examples": ["Consistency is key"],
  "metadata": {"tags": ["mindset"], "created_at": "2026-04-20T10:00:00Z", "review_count": 0}
},
{
  "word": "Resilient",
  "definitions": [{"pos": "adjective", "meaning": "kiên cường"}],
  "examples": [],
  "metadata": {"tags": ["mindset"], "created_at": "2026-04-21T10:00:00Z", "review_count": 0}
}
]"#
        .to_string()
}

#[test]
fn test_import_json_success() {
    let db = init_memory_db().unwrap();
    let repo = WordRepository::new(db);
    let json = create_test_json();

    let report = VocabularyService::import_json(&repo, &json).unwrap();

    assert_eq!(report.inserted_count, 2);
    assert_eq!(report.updated_count, 0);
    assert_eq!(report.skipped_count, 0);
    assert_eq!(report.total_processed(), 2);
}

#[test]
fn test_import_json_with_merge() {
    let db = init_memory_db().unwrap();
    let repo = WordRepository::new(db);
    let json1 = create_test_json();

    let report1 = VocabularyService::import_json(&repo, &json1).unwrap();
    assert_eq!(report1.inserted_count, 2);

    let json2 = r#"[
{
  "word": "Consistency",
  "phonetic": "/kənˈsɪstənsi/",
  "definitions": [{"pos": "verb", "meaning": "duy trì"}],
  "examples": ["Additional example"],
  "metadata": {"tags": ["professional"], "created_at": "2026-04-20T10:00:00Z", "review_count": 2}
}
]"#;

    let report2 = VocabularyService::import_json(&repo, &json2).unwrap();
    assert_eq!(report2.inserted_count, 0);
    assert_eq!(report2.updated_count, 1);

    let loaded = repo.get_by_word_key("consistency").unwrap().unwrap();
    assert_eq!(loaded.definitions.len(), 2);
    assert_eq!(loaded.examples.len(), 2);
    assert_eq!(loaded.metadata.tags.len(), 2);
    assert_eq!(loaded.metadata.review_count, 2);
}

#[test]
fn test_export_json_contains_all_fields() {
    let db = init_memory_db().unwrap();
    let repo = WordRepository::new(db);

    let json_import = create_test_json();
    VocabularyService::import_json(&repo, &json_import).unwrap();

    let json_export = VocabularyService::export_json(&repo).unwrap();

    let exported: Vec<serde_json::Value> = serde_json::from_str(&json_export).unwrap();
    assert_eq!(exported.len(), 2);
    assert_eq!(exported[0]["word"], "Consistency");
    assert_eq!(exported[0]["phonetic"], "/kənˈsɪstənsi/");
    assert_eq!(exported[0]["definitions"][0]["meaning"], "Tính nhất quán");
    assert_eq!(exported[0]["examples"][0], "Consistency is key");
}

#[test]
fn test_round_trip_json_import_export_import() {
    let db = init_memory_db().unwrap();
    let repo = WordRepository::new(db);

    let original_json = create_test_json();

    let report1 = VocabularyService::import_json(&repo, &original_json).unwrap();
    assert_eq!(report1.inserted_count, 2);

    let exported_json = VocabularyService::export_json(&repo).unwrap();

    let db2 = init_memory_db().unwrap();
    let repo2 = WordRepository::new(db2);
    let report2 = VocabularyService::import_json(&repo2, &exported_json).unwrap();

    assert_eq!(report2.inserted_count, 2);

    let original = repo.get_by_word_key("consistency").unwrap().unwrap();
    let reimported = repo2.get_by_word_key("consistency").unwrap().unwrap();
    assert_eq!(original.word, reimported.word);
    assert_eq!(original.phonetic, reimported.phonetic);
    assert_eq!(original.definitions.len(), reimported.definitions.len());
    assert_eq!(original.examples, reimported.examples);
    assert_eq!(original.metadata.tags, reimported.metadata.tags);
}

#[test]
fn test_import_json_invalid_format() {
    let db = init_memory_db().unwrap();
    let repo = WordRepository::new(db);

    let invalid_json = "{ not valid json";
    let result = VocabularyService::import_json(&repo, invalid_json);

    assert!(result.is_err());
}
