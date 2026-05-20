use reemember::bilingual::{BilingualRepository, parse_bilingual_json};
use reemember::db::init_memory_db;

const SAMPLE: &str = r#"
[
  {
    "book": "Everyday Stories",
    "level": "A2",
    "title": "A Rainy Morning",
    "paragraphs": [
      [
        { "en": "When Lina woke up,", "vi": "Khi Lina th\u1ee9c d\u1eady," },
        { "en": "rain was tapping gently on the window.", "vi": "m\u01b0a \u0111ang g\u00f5 nh\u1eb9 l\u00ean c\u1eeda s\u1ed5." }
      ]
    ],
    "structures": [
      {
        "pattern": "When + past simple, past continuous",
        "example": "When Lina woke up, rain was tapping gently on the window.",
        "note": "D\u00f9ng when \u0111\u1ec3 \u0111\u1eb7t m\u1ed1c th\u1eddi gian."
      }
    ]
  }
]
"#;

#[test]
fn parse_bilingual_json_success() {
    let articles = parse_bilingual_json(SAMPLE).unwrap();
    assert_eq!(articles.len(), 1);
    assert_eq!(articles[0].title, "A Rainy Morning");
    assert_eq!(articles[0].paragraphs[0].len(), 2);
    assert_eq!(
        articles[0].structures[0].pattern,
        "When + past simple, past continuous"
    );
}

#[test]
fn parse_bilingual_json_requires_segments() {
    let err = parse_bilingual_json(
        r#"[{"title":"Empty","book":"Book","paragraphs":[[{"en":"","vi":"nghia"}]]}]"#,
    )
    .unwrap_err();
    assert!(err.to_string().contains(".en"));
}

#[test]
fn upsert_bilingual_article_updates_existing_doc_without_duplicate() {
    let conn = init_memory_db().unwrap();
    let repo = BilingualRepository::new(conn);
    let mut articles = parse_bilingual_json(SAMPLE).unwrap();

    let first = repo.upsert_article(&articles[0]).unwrap();
    assert!(first.inserted);

    articles[0].level = Some("B1".to_string());
    articles[0]
        .structures
        .push(reemember::bilingual::BilingualStructure {
            pattern: "If + present simple, will + verb".to_string(),
            example: "If I learn a little every day, I will remember more.".to_string(),
            note: None,
        });

    let second = repo.upsert_article(&articles[0]).unwrap();
    assert_eq!(first.id, second.id);
    assert!(!second.inserted);

    let summaries = repo.list_articles().unwrap();
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].level.as_deref(), Some("B1"));
    assert_eq!(summaries[0].structure_count, 2);
}
