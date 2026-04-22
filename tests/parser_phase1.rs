use reemember::parser::{parse_csv, parse_json_canonical};

#[test]
fn parse_json_canonical_success() {
    let input = r#"
[
  {
    "word": "Consistency",
    "definitions": [
      { "meaning": "being consistent" }
    ],
    "metadata": {
      "review_count": 0
    }
  }
]
"#;

    let records = parse_json_canonical(input).expect("json should parse");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].word_key(), "consistency");
}

#[test]
fn parse_json_fails_when_word_is_missing() {
    let input = r#"
[
  {
    "word": "   ",
    "definitions": [
      { "meaning": "test" }
    ]
  }
]
"#;

    let err = parse_json_canonical(input).expect_err("json should fail validation");
    assert!(err.to_string().contains("missing required field: word"));
}

#[test]
fn parse_json_fails_when_meaning_is_missing() {
    let input = r#"
[
  {
    "word": "consistency",
    "definitions": [
      { "meaning": "   " }
    ]
  }
]
"#;

    let err = parse_json_canonical(input).expect_err("json should fail validation");
    assert!(
        err.to_string()
            .contains("missing required field: definitions[].meaning")
    );
}

#[test]
fn parse_csv_minimal_success_and_skip_blank_lines() {
    let input = "word,meaning_vi\nconsistency,being consistent\n\nresilient,strong\n";

    let report = parse_csv(input).expect("csv should parse");
    assert_eq!(report.errors.len(), 0);
    assert_eq!(report.records.len(), 2);
    assert_eq!(report.records[0].word, "consistency");
    assert_eq!(report.records[1].definitions[0].meaning, "strong");
}

#[test]
fn parse_csv_reports_missing_required_fields_per_row() {
    let input = "word,meaning_vi\n,meaning only\nword_only,\nvalid,ok\n";

    let report = parse_csv(input).expect("csv should parse with row errors");
    assert_eq!(report.records.len(), 1);
    assert_eq!(report.errors.len(), 2);
    assert!(report.errors[0].message.contains("missing required field: word"));
    assert!(
        report.errors[1]
            .message
            .contains("missing required field: meaning_vi")
    );
}

#[test]
fn parse_csv_extended_success_with_split_fields() {
    let input = concat!(
        "word,meaning_vi,phonetic,pos,examples,tags,created_at,review_count\n",
        "consistency,meaning one; meaning two,/k/,noun,ex1 | ex2,tag1 | tag2,2026-04-20T10:00:00Z,2\n"
    );

    let report = parse_csv(input).expect("csv should parse");
    assert_eq!(report.errors.len(), 0);
    assert_eq!(report.records.len(), 1);

    let record = &report.records[0];
    assert_eq!(record.definitions.len(), 2);
    assert_eq!(record.examples.len(), 2);
    assert_eq!(record.metadata.tags.len(), 2);
    assert_eq!(record.metadata.review_count, 2);
}

#[test]
fn parse_csv_reports_invalid_review_count() {
    let input = "word,meaning_vi,review_count\nconsistency,valid,not_number\n";

    let report = parse_csv(input).expect("csv should parse with row errors");
    assert_eq!(report.records.len(), 0);
    assert_eq!(report.errors.len(), 1);
    assert!(report.errors[0].message.contains("invalid review_count"));
}

