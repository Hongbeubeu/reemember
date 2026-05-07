use reemember::db::init_memory_db;
use reemember::grammar::{GrammarDocInput, GrammarRepository};
use serde_json::json;

fn grammar_doc(content: &str, exercise_type: &str) -> GrammarDocInput {
    GrammarDocInput {
        title: "Passive Voice".to_string(),
        category: Some("passive".to_string()),
        level: Some("A2".to_string()),
        content: content.to_string(),
        examples: vec![],
        exercises: vec![json!({ "type": exercise_type })],
    }
}

#[test]
fn test_grammar_upsert_updates_existing_doc_without_duplicate() {
    let db = init_memory_db().unwrap();
    let repo = GrammarRepository::new(db);

    let first = grammar_doc("first version", "fill_blank");
    let first_result = repo.upsert_doc(&first, None).unwrap();
    assert!(first_result.inserted);

    let second = grammar_doc("latest version", "true_false");
    let second_result = repo.upsert_doc(&second, None).unwrap();
    assert!(!second_result.inserted);
    assert_eq!(first_result.id, second_result.id);

    let docs = repo.list_docs().unwrap();
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].exercise_count, 1);

    let detail = repo
        .get_doc_with_exercises(first_result.id)
        .unwrap()
        .unwrap();
    assert_eq!(detail.doc.content, "latest version");
    assert_eq!(detail.exercises.len(), 1);
    assert_eq!(detail.exercises[0].exercise_type, "true_false");
}
