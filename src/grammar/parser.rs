use serde::Deserialize;
use serde_json::Value;
use crate::parser::ParseError;

#[derive(Debug, Deserialize)]
struct RawGrammarDoc {
    title: String,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    level: Option<String>,
    #[serde(default)]
    content: String,
    #[serde(default)]
    examples: Vec<String>,
    #[serde(default)]
    exercises: Vec<Value>,
}

pub struct GrammarDocInput {
    pub title: String,
    pub category: Option<String>,
    pub level: Option<String>,
    pub content: String,
    pub examples: Vec<String>,
    pub exercises: Vec<Value>,
}

/// Parse a JSON array of grammar documents (legacy format).
pub fn parse_grammar_json(input: &str) -> Result<Vec<GrammarDocInput>, ParseError> {
    let raw: Vec<RawGrammarDoc> = serde_json::from_str(input)?;
    Ok(raw.into_iter().map(|r| GrammarDocInput {
        title: r.title,
        category: r.category,
        level: r.level,
        content: r.content,
        examples: r.examples,
        exercises: r.exercises,
    }).collect())
}

/// Parse a single `.md` file with the format:
///
/// ```
/// ---
/// title: Present Simple
/// category: tenses
/// level: A1
/// ---
///
/// [Markdown content]
///
/// <!-- EXERCISES
/// [ { ... }, ... ]
/// -->
/// ```
pub fn parse_grammar_md(input: &str) -> Result<GrammarDocInput, ParseError> {
    // --- Extract frontmatter ---
    let (frontmatter, body) = if input.trim_start().starts_with("---") {
        let after_open = input.trim_start().trim_start_matches('-').trim_start_matches('\n');
        let close = after_open.find("\n---")
            .ok_or_else(|| ParseError::InvalidData("frontmatter closing '---' not found".into()))?;
        let fm = &after_open[..close];
        let rest = after_open[close..].trim_start_matches('-').trim_start_matches('\n');
        (fm, rest)
    } else {
        ("", input)
    };

    let mut title = String::new();
    let mut category: Option<String> = None;
    let mut level: Option<String> = None;

    for line in frontmatter.lines() {
        if let Some(v) = line.strip_prefix("title:") {
            title = v.trim().trim_matches('"').to_string();
        } else if let Some(v) = line.strip_prefix("category:") {
            category = Some(v.trim().trim_matches('"').to_string());
        } else if let Some(v) = line.strip_prefix("level:") {
            level = Some(v.trim().trim_matches('"').to_string());
        }
    }

    if title.is_empty() {
        return Err(ParseError::InvalidData("missing 'title:' in frontmatter".into()));
    }

    // --- Extract exercises from <!-- EXERCISES ... --> ---
    const MARKER_OPEN: &str = "<!-- EXERCISES";
    const MARKER_CLOSE: &str = "-->";

    let (content, exercises) = if let Some(start) = body.find(MARKER_OPEN) {
        let inner_start = start + MARKER_OPEN.len();
        let inner_end = body[inner_start..].find(MARKER_CLOSE)
            .ok_or_else(|| ParseError::InvalidData("unclosed <!-- EXERCISES --> block".into()))?;
        let json_str = body[inner_start..inner_start + inner_end].trim();
        let exercises: Vec<Value> = serde_json::from_str(json_str)?;
        let content_raw = format!("{}\n{}", &body[..start].trim(), body[inner_start + inner_end + MARKER_CLOSE.len()..].trim());
        (content_raw.trim().to_string(), exercises)
    } else {
        (body.trim().to_string(), vec![])
    };

    Ok(GrammarDocInput {
        title,
        category,
        level,
        content,
        examples: vec![],
        exercises,
    })
}
