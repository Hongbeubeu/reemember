use reemember::parser::parse_json_canonical;

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
