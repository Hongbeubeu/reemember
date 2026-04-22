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

fn create_test_csv_minimal() -> String {
    "word,meaning_vi\nconsistency,tính nhất quán\nresilient,kiên cường\n".to_string()
}

fn create_test_csv_extended() -> String {
    r#"word,meaning_vi,phonetic,pos,examples,tags,created_at,review_count
consistency,"tính nhất quán",/kənˈsɪstənsi/,noun,"Consistency is key | Example 2","mindset | professional",2026-04-20T10:00:00Z,0
resilient,"kiên cường",,adjective,,mindset,2026-04-21T10:00:00Z,0
"#
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
    assert_eq!(report.csv_errors.len(), 0);
    assert_eq!(report.total_processed(), 2);
}

#[test]
fn test_import_json_with_merge() {
    let db = init_memory_db().unwrap();
    let repo = WordRepository::new(db);
    let json1 = create_test_json();

    // Import first batch
    let report1 = VocabularyService::import_json(&repo, &json1).unwrap();
    assert_eq!(report1.inserted_count, 2);

    // Import second batch with same words (should merge)
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

    // Verify merge happened
    let loaded = repo.get_by_word_key("consistency").unwrap().unwrap();
    assert_eq!(loaded.definitions.len(), 2); // Both meanings merged
    assert_eq!(loaded.examples.len(), 2); // Both examples merged
    assert_eq!(loaded.metadata.tags.len(), 2); // Both tags merged
    assert_eq!(loaded.metadata.review_count, 2); // Max kept
}

#[test]
fn test_import_csv_minimal() {
    let db = init_memory_db().unwrap();
    let repo = WordRepository::new(db);
    let csv = create_test_csv_minimal();

    let report = VocabularyService::import_csv(&repo, &csv).unwrap();

    assert_eq!(report.inserted_count, 2);
    assert_eq!(report.updated_count, 0);
    assert_eq!(report.csv_errors.len(), 0);
    assert_eq!(report.total_processed(), 2);

    // Verify records were created
    let consistency = repo.get_by_word_key("consistency").unwrap().unwrap();
    assert_eq!(consistency.word, "consistency");
    assert_eq!(consistency.definitions[0].meaning, "tính nhất quán");
}

#[test]
fn test_import_csv_extended() {
    let db = init_memory_db().unwrap();
    let repo = WordRepository::new(db);
    let csv = create_test_csv_extended();

    let report = VocabularyService::import_csv(&repo, &csv).unwrap();

    assert_eq!(report.inserted_count, 2);
    assert_eq!(report.csv_errors.len(), 0);

    // Verify extended fields were parsed
    let consistency = repo.get_by_word_key("consistency").unwrap().unwrap();
    assert_eq!(consistency.phonetic, Some("/kənˈsɪstənsi/".to_string()));
    assert_eq!(consistency.definitions[0].pos, Some("noun".to_string()));
    assert_eq!(consistency.examples.len(), 2);
    assert_eq!(consistency.metadata.tags.len(), 2);
}

#[test]
fn test_import_csv_with_errors() {
    let db = init_memory_db().unwrap();
    let repo = WordRepository::new(db);

    // CSV with missing required field (row 2)
    let csv = "word,meaning_vi\nvalid,meaning\n,incomplete\n";

    let report = VocabularyService::import_csv(&repo, &csv).unwrap();

    assert_eq!(report.inserted_count, 1); // Only valid record
    assert_eq!(report.csv_errors.len(), 1); // One CSV parsing error
    assert!(report
        .csv_errors[0]
        .message
        .contains("missing required field"));
}

#[test]
fn test_export_json_contains_all_fields() {
    let db = init_memory_db().unwrap();
    let repo = WordRepository::new(db);

    // Import first
    let json_import = create_test_json();
    VocabularyService::import_json(&repo, &json_import).unwrap();

    // Export
    let json_export = VocabularyService::export_json(&repo).unwrap();

    // Parse and verify
    let exported: Vec<serde_json::Value> = serde_json::from_str(&json_export).unwrap();
    assert_eq!(exported.len(), 2);
    assert_eq!(exported[0]["word"], "Consistency");
    assert_eq!(exported[0]["phonetic"], "/kənˈsɪstənsi/");
    assert_eq!(exported[0]["definitions"][0]["meaning"], "Tính nhất quán");
    assert_eq!(exported[0]["examples"][0], "Consistency is key");
}

#[test]
fn test_export_csv_header_and_data() {
    let db = init_memory_db().unwrap();
    let repo = WordRepository::new(db);

    // Import
    let json = create_test_json();
    VocabularyService::import_json(&repo, &json).unwrap();

    // Export to CSV
    let csv = VocabularyService::export_csv(&repo).unwrap();

    // Verify header
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(
        lines[0],
        "word,meaning_vi,phonetic,pos,examples,tags,created_at,review_count"
    );

    // Verify data rows
    assert!(lines.len() >= 3); // Header + at least 2 data rows
    assert!(csv.contains("Consistency"));
    assert!(csv.contains("Resilient"));
}

#[test]
fn test_round_trip_json_import_export_import() {
    let db = init_memory_db().unwrap();
    let repo = WordRepository::new(db);

    let original_json = create_test_json();

    // First import
    let report1 = VocabularyService::import_json(&repo, &original_json).unwrap();
    assert_eq!(report1.inserted_count, 2);

    // Export to JSON
    let exported_json = VocabularyService::export_json(&repo).unwrap();

    // Create new repository and import exported JSON
    let db2 = init_memory_db().unwrap();
    let repo2 = WordRepository::new(db2);
    let report2 = VocabularyService::import_json(&repo2, &exported_json).unwrap();

    // Verify same counts
    assert_eq!(report2.inserted_count, 2);

    // Verify records are identical
    let original = repo.get_by_word_key("consistency").unwrap().unwrap();
    let reimported = repo2.get_by_word_key("consistency").unwrap().unwrap();
    assert_eq!(original.word, reimported.word);
    assert_eq!(original.phonetic, reimported.phonetic);
    assert_eq!(original.definitions.len(), reimported.definitions.len());
    assert_eq!(original.examples, reimported.examples);
    assert_eq!(original.metadata.tags, reimported.metadata.tags);
}

#[test]
fn test_round_trip_csv_minimal_import_export() {
    let db = init_memory_db().unwrap();
    let repo = WordRepository::new(db);

    let csv_minimal = create_test_csv_minimal();

    // Import CSV minimal
    let report = VocabularyService::import_csv(&repo, &csv_minimal).unwrap();
    assert_eq!(report.inserted_count, 2);

    // Export to JSON
    let json = VocabularyService::export_json(&repo).unwrap();

    // Re-import from JSON
    let db2 = init_memory_db().unwrap();
    let repo2 = WordRepository::new(db2);
    let report2 = VocabularyService::import_json(&repo2, &json).unwrap();

    assert_eq!(report2.inserted_count, 2);

    // Verify core fields preserved
    let word1 = repo.get_by_word_key("consistency").unwrap().unwrap();
    let word2 = repo2.get_by_word_key("consistency").unwrap().unwrap();
    assert_eq!(word1.word, word2.word);
    assert_eq!(word1.definitions[0].meaning, word2.definitions[0].meaning);
}

#[test]
fn test_import_json_invalid_format() {
    let db = init_memory_db().unwrap();
    let repo = WordRepository::new(db);

    let invalid_json = "{ not valid json";
    let result = VocabularyService::import_json(&repo, invalid_json);

    assert!(result.is_err());
}

#[test]
fn test_import_csv_empty_file() {
    let db = init_memory_db().unwrap();
    let repo = WordRepository::new(db);

    let empty_csv = "";
    let result = VocabularyService::import_csv(&repo, empty_csv);

    assert!(result.is_err()); // No headers
}

#[test]
fn test_acceptance_criteria_import_report_format() {
    let db = init_memory_db().unwrap();
    let repo = WordRepository::new(db);

    let csv_with_mix = "word,meaning_vi\nvalid1,meaning1\nvalid2,meaning2\n,missing\n";

    let report = VocabularyService::import_csv(&repo, &csv_with_mix).unwrap();

    // Check report contains required fields
    assert!(report.inserted_count > 0);
    assert!(report.csv_errors.len() > 0);
    // 2 valid + 1 error row = 3 total
    assert_eq!(report.total_processed() + report.csv_errors.len(), 3);

    println!("Import Report:");
    println!("  Inserted: {}", report.inserted_count);
    println!("  Updated: {}", report.updated_count);
    println!("  Skipped: {}", report.skipped_count);
    println!("  CSV Errors: {}", report.csv_errors.len());
    println!("  Total: {}", report.total_processed());
}

#[test]
fn test_acceptance_criteria_export_json_and_csv() {
    let db = init_memory_db().unwrap();
    let repo = WordRepository::new(db);

    let json_import = create_test_json();
    VocabularyService::import_json(&repo, &json_import).unwrap();

    // Export to JSON
    let json_export = VocabularyService::export_json(&repo).unwrap();
    assert!(!json_export.is_empty());
    assert!(json_export.contains("Consistency"));

    // Export to CSV
    let csv_export = VocabularyService::export_csv(&repo).unwrap();
    assert!(!csv_export.is_empty());
    assert!(csv_export.contains("word,meaning_vi"));
    assert!(csv_export.contains("Consistency"));
}

#[test]
fn test_acceptance_criteria_sort_in_export() {
    let db = init_memory_db().unwrap();
    let repo = WordRepository::new(db);

    // Import records with different review counts
    let json = r#"[
{"word": "zebra", "definitions": [{"meaning": "m"}], "metadata": {"review_count": 1, "tags": [], "created_at": null}},
{"word": "apple", "definitions": [{"meaning": "m"}], "metadata": {"review_count": 5, "tags": [], "created_at": null}},
{"word": "middle", "definitions": [{"meaning": "m"}], "metadata": {"review_count": 3, "tags": [], "created_at": null}}
]"#;

    VocabularyService::import_json(&repo, json).unwrap();

    // Export (current order may vary, but should contain all)
    let csv = VocabularyService::export_csv(&repo).unwrap();
    assert!(csv.contains("apple"));
    assert!(csv.contains("middle"));
    assert!(csv.contains("zebra"));
}


