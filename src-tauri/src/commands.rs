use reemember::db::init_db;
use reemember::grammar::{GrammarRepository, parse_grammar_json, parse_grammar_md};
use reemember::model::{Collection, Topic};
use reemember::repository::{QueryOptions, SortBy, WordRepository};
use reemember::service::VocabularyService;
use reemember::testing::QuestionDirection;
use reemember::testing::{Question, TestMode, TestingOptions};
use serde::{Deserialize, Serialize};

#[tauri::command]
pub fn set_app_theme(window: tauri::WebviewWindow, mode: String) -> Result<(), String> {
    let theme = match mode.as_str() {
        "light" => Some(tauri::Theme::Light),
        "dark" => Some(tauri::Theme::Dark),
        "system" => None,
        other => return Err(format!("unsupported theme mode: {}", other)),
    };

    window.set_theme(theme).map_err(|e| e.to_string())
}

// ── Study DTOs ────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NextQuestionRequest {
    pub mode: String,
    pub srs_enabled: bool,
    pub topic_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitAnswerRequest {
    pub answer: String,
    pub srs_enabled: bool,
    pub question: QuestionDto,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct QuestionDto {
    pub word_key: String,
    pub direction: String,
    pub prompt: String,
    pub word: Option<String>,
    pub phonetic: Option<String>,
    pub examples: Vec<String>,
    pub expected_answers: Vec<String>,
    pub synonyms: Vec<String>,
    pub antonyms: Vec<String>,
    pub family_words: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnswerResultDto {
    pub correct: bool,
    pub normalized_answer: String,
    pub accepted_answers: Vec<String>,
}

// ── Library DTOs ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WordSummaryDto {
    pub word_key: String,
    pub word: String,
    pub phonetic: Option<String>,
    pub meanings: Vec<String>,
    pub tags: Vec<String>,
    pub review_count: u32,
    pub examples_count: usize,
    pub synonyms: Vec<String>,
    pub antonyms: Vec<String>,
    pub family_words: Vec<String>,
}

// ── Collection / Topic DTOs ───────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionDto {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopicDto {
    pub id: i64,
    pub collection_id: i64,
    pub name: String,
    pub description: Option<String>,
}

// ── Import/Export DTOs ────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReportDto {
    pub inserted_count: usize,
    pub updated_count: usize,
    pub skipped_count: usize,
    pub error_count: usize,
    pub errors: Vec<String>,
}

// ── Study commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub fn next_question(payload: NextQuestionRequest) -> Result<Option<QuestionDto>, String> {
    let mode = parse_mode(&payload.mode)?;
    let conn = init_db("reemember.db").map_err(|e| e.to_string())?;
    let repo = WordRepository::new(conn);

    let options = TestingOptions {
        srs_enabled: payload.srs_enabled,
        topic_id: payload.topic_id,
    };
    let maybe_question =
        reemember::testing::TestingEngine::generate_question_with_options(&repo, mode, options)
            .map_err(|e| e.to_string())?;

    Ok(maybe_question.map(from_question))
}

#[tauri::command]
pub fn submit_answer(payload: SubmitAnswerRequest) -> Result<AnswerResultDto, String> {
    let conn = init_db("reemember.db").map_err(|e| e.to_string())?;
    let repo = WordRepository::new(conn);

    let question = to_question(&payload.question)?;
    let result = VocabularyService::submit_answer_with_srs(
        &repo,
        &question,
        &payload.answer,
        payload.srs_enabled,
    )
    .map_err(|e| e.to_string())?;

    Ok(AnswerResultDto {
        correct: result.correct,
        normalized_answer: result.normalized_answer,
        accepted_answers: result.accepted_answers,
    })
}

// ── Library commands ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListWordsRequest {
    pub collection_id: Option<i64>,
    pub topic_id: Option<i64>,
}

#[tauri::command]
pub fn list_words(payload: Option<ListWordsRequest>) -> Result<Vec<WordSummaryDto>, String> {
    let conn = init_db("reemember.db").map_err(|e| e.to_string())?;
    let repo = WordRepository::new(conn);

    let mut options = QueryOptions {
        sort: Some(SortBy::Word),
        limit: None,
        tag_filter: None,
        collection_id: None,
        topic_id: None,
    };
    if let Some(p) = payload {
        options.collection_id = p.collection_id;
        options.topic_id = p.topic_id;
    }

    let records = repo.query(&options).map_err(|e| e.to_string())?;

    Ok(records
        .into_iter()
        .map(|r| {
            let meanings: Vec<String> = r
                .definitions
                .iter()
                .map(|d| match &d.pos {
                    Some(pos) => format!("[{}] {}", pos, d.meaning),
                    None => d.meaning.clone(),
                })
                .collect();
            let examples_count = r.examples.len();
            WordSummaryDto {
                word_key: r.word_key(),
                word: r.word,
                phonetic: r.phonetic,
                meanings,
                tags: r.metadata.tags,
                review_count: r.metadata.review_count,
                examples_count,
                synonyms: r.synonyms,
                antonyms: r.antonyms,
                family_words: r.family_words,
            }
        })
        .collect())
}

#[tauri::command]
pub fn delete_word(word_key: String) -> Result<(), String> {
    let conn = init_db("reemember.db").map_err(|e| e.to_string())?;
    let repo = WordRepository::new(conn);
    repo.delete_by_word_key(&word_key)
        .map_err(|e| e.to_string())
}

// ── Collection commands ───────────────────────────────────────────────────────

#[tauri::command]
pub fn list_collections() -> Result<Vec<CollectionDto>, String> {
    let conn = init_db("reemember.db").map_err(|e| e.to_string())?;
    let repo = WordRepository::new(conn);
    repo.list_collections()
        .map_err(|e| e.to_string())
        .map(|cols| cols.into_iter().map(from_collection).collect())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCollectionRequest {
    pub name: String,
    pub description: Option<String>,
}

#[tauri::command]
pub fn create_collection(payload: CreateCollectionRequest) -> Result<CollectionDto, String> {
    let conn = init_db("reemember.db").map_err(|e| e.to_string())?;
    let repo = WordRepository::new(conn);
    repo.create_collection(&payload.name, payload.description.as_deref())
        .map_err(|e| e.to_string())
        .map(from_collection)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCollectionRequest {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
}

#[tauri::command]
pub fn update_collection(payload: UpdateCollectionRequest) -> Result<(), String> {
    let conn = init_db("reemember.db").map_err(|e| e.to_string())?;
    let repo = WordRepository::new(conn);
    repo.update_collection(payload.id, &payload.name, payload.description.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_collection(id: i64) -> Result<(), String> {
    let conn = init_db("reemember.db").map_err(|e| e.to_string())?;
    let repo = WordRepository::new(conn);
    repo.delete_collection(id).map_err(|e| e.to_string())
}

// ── Topic commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub fn list_topics(collection_id: i64) -> Result<Vec<TopicDto>, String> {
    let conn = init_db("reemember.db").map_err(|e| e.to_string())?;
    let repo = WordRepository::new(conn);
    repo.list_topics(collection_id)
        .map_err(|e| e.to_string())
        .map(|topics| topics.into_iter().map(from_topic).collect())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTopicRequest {
    pub collection_id: i64,
    pub name: String,
    pub description: Option<String>,
}

#[tauri::command]
pub fn create_topic(payload: CreateTopicRequest) -> Result<TopicDto, String> {
    let conn = init_db("reemember.db").map_err(|e| e.to_string())?;
    let repo = WordRepository::new(conn);
    repo.create_topic(
        payload.collection_id,
        &payload.name,
        payload.description.as_deref(),
    )
    .map_err(|e| e.to_string())
    .map(from_topic)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTopicRequest {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
}

#[tauri::command]
pub fn update_topic(payload: UpdateTopicRequest) -> Result<(), String> {
    let conn = init_db("reemember.db").map_err(|e| e.to_string())?;
    let repo = WordRepository::new(conn);
    repo.update_topic(payload.id, &payload.name, payload.description.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_topic(id: i64) -> Result<(), String> {
    let conn = init_db("reemember.db").map_err(|e| e.to_string())?;
    let repo = WordRepository::new(conn);
    repo.delete_topic(id).map_err(|e| e.to_string())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignWordToTopicRequest {
    pub word_key: String,
    pub topic_id: i64,
}

#[tauri::command]
pub fn assign_word_to_topic(payload: AssignWordToTopicRequest) -> Result<(), String> {
    let conn = init_db("reemember.db").map_err(|e| e.to_string())?;
    let repo = WordRepository::new(conn);
    repo.assign_word_to_topic(&payload.word_key, payload.topic_id)
        .map_err(|e| e.to_string())
}

// ── Import / Export commands ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportVocabularyRequest {
    pub content: String,
    pub collection_name: Option<String>,
    pub topic_name: Option<String>,
}

#[tauri::command]
pub fn import_vocabulary(payload: ImportVocabularyRequest) -> Result<ImportReportDto, String> {
    let conn = init_db("reemember.db").map_err(|e| e.to_string())?;
    let repo = WordRepository::new(conn);

    let report = VocabularyService::import_json_scoped(
        &repo,
        &payload.content,
        payload.collection_name.as_deref(),
        payload.topic_name.as_deref(),
    )
    .map_err(|e| e.to_string())?;

    Ok(ImportReportDto {
        inserted_count: report.inserted_count,
        updated_count: report.updated_count,
        skipped_count: report.skipped_count,
        error_count: 0,
        errors: vec![],
    })
}

#[tauri::command]
pub async fn save_export(app: tauri::AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::{DialogExt, FilePath};

    let conn = init_db("reemember.db").map_err(|e| e.to_string())?;
    let repo = WordRepository::new(conn);
    let content = VocabularyService::export_json(&repo).map_err(|e| e.to_string())?;

    let (tx, rx) = std::sync::mpsc::sync_channel::<Option<FilePath>>(1);

    app.dialog()
        .file()
        .set_file_name("reemember-export.json")
        .add_filter("JSON file", &["json"])
        .save_file(move |path| {
            let _ = tx.send(path);
        });

    let chosen = tauri::async_runtime::spawn_blocking(move || rx.recv())
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

    match chosen {
        None => Ok(None),
        Some(fp) => {
            let path = fp.as_path().ok_or("Could not resolve save path")?;
            std::fs::write(path, content.as_bytes()).map_err(|e| e.to_string())?;
            Ok(Some(path.to_string_lossy().to_string()))
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn parse_mode(value: &str) -> Result<TestMode, String> {
    match value.trim().to_lowercase().as_str() {
        "envi" | "en-vi" => Ok(TestMode::EnVi),
        "vien" | "vi-en" => Ok(TestMode::ViEn),
        "hybrid" => Ok(TestMode::Hybrid),
        other => Err(format!("unsupported mode: {}", other)),
    }
}

fn from_question(question: Question) -> QuestionDto {
    QuestionDto {
        word_key: question.word_key,
        direction: match question.direction {
            QuestionDirection::EnVi => "en-vi".to_string(),
            QuestionDirection::ViEn => "vi-en".to_string(),
        },
        prompt: question.prompt,
        word: question.word,
        phonetic: question.phonetic,
        examples: question.examples,
        expected_answers: question.expected_answers,
        synonyms: question.synonyms,
        antonyms: question.antonyms,
        family_words: question.family_words,
    }
}

fn to_question(dto: &QuestionDto) -> Result<Question, String> {
    let direction = match dto.direction.as_str() {
        "en-vi" => QuestionDirection::EnVi,
        "vi-en" => QuestionDirection::ViEn,
        _ => return Err("invalid question direction".to_string()),
    };
    Ok(Question {
        word_key: dto.word_key.clone(),
        direction,
        prompt: dto.prompt.clone(),
        word: dto.word.clone(),
        phonetic: dto.phonetic.clone(),
        examples: dto.examples.clone(),
        expected_answers: dto.expected_answers.clone(),
        synonyms: dto.synonyms.clone(),
        antonyms: dto.antonyms.clone(),
        family_words: dto.family_words.clone(),
    })
}

// ── Grammar DTOs ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GrammarDocDto {
    pub id: i64,
    pub title: String,
    pub category: Option<String>,
    pub level: Option<String>,
    pub exercise_count: usize,
    pub group_id: Option<i64>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GrammarGroupDto {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub sort_order: i64,
    pub doc_count: usize,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateGrammarGroupRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateGrammarGroupRequest {
    pub id: i64,
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveGrammarDocRequest {
    pub doc_id: i64,
    pub group_id: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GrammarExerciseDto {
    pub id: i64,
    pub exercise_type: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GrammarDocDetailDto {
    pub id: i64,
    pub title: String,
    pub category: Option<String>,
    pub level: Option<String>,
    pub content: String,
    pub examples: Vec<String>,
    pub exercises: Vec<GrammarExerciseDto>,
    pub group_id: Option<i64>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GrammarImportResultDto {
    pub imported_count: usize,
    pub errors: Vec<String>,
}

// ── Grammar commands ──────────────────────────────────────────────────────────

#[tauri::command]
pub fn list_grammar_docs() -> Result<Vec<GrammarDocDto>, String> {
    let conn = init_db("reemember.db").map_err(|e| e.to_string())?;
    let repo = GrammarRepository::new(conn);
    repo.list_docs().map_err(|e| e.to_string()).map(|docs| {
        docs.into_iter()
            .map(|d| GrammarDocDto {
                id: d.id,
                title: d.title,
                category: d.category,
                level: d.level,
                exercise_count: d.exercise_count,
                group_id: d.group_id,
                created_at: d.created_at,
            })
            .collect()
    })
}

#[tauri::command]
pub fn get_grammar_doc(id: i64) -> Result<Option<GrammarDocDetailDto>, String> {
    let conn = init_db("reemember.db").map_err(|e| e.to_string())?;
    let repo = GrammarRepository::new(conn);
    let detail = repo.get_doc_with_exercises(id).map_err(|e| e.to_string())?;
    Ok(detail.map(|d| GrammarDocDetailDto {
        id: d.doc.id,
        title: d.doc.title,
        category: d.doc.category,
        level: d.doc.level,
        content: d.doc.content,
        examples: d.doc.examples,
        exercises: d
            .exercises
            .into_iter()
            .map(|e| GrammarExerciseDto {
                id: e.id,
                exercise_type: e.exercise_type,
                data: e.data,
            })
            .collect(),
        group_id: d.doc.group_id,
        created_at: d.doc.created_at,
    }))
}

/// Resolve a `category` string to an existing-or-newly-created group id.
/// Used by import to auto-assign group based on frontmatter `category`.
fn resolve_group_for_category(repo: &GrammarRepository, category: &Option<String>) -> Option<i64> {
    let name = category.as_ref()?.trim();
    if name.is_empty() {
        return None;
    }
    repo.find_or_create_group(name).ok()
}

#[tauri::command]
pub fn import_grammar(content: String) -> Result<GrammarImportResultDto, String> {
    let trimmed = content.trim_start();
    let conn = init_db("reemember.db").map_err(|e| e.to_string())?;
    let repo = GrammarRepository::new(conn);
    let mut imported_count = 0;
    let mut errors = vec![];

    // Auto-detect format: frontmatter "---" or leading "# " → Markdown; otherwise JSON array
    if trimmed.starts_with("---") || trimmed.starts_with("# ") {
        match parse_grammar_md(&content) {
            Ok(doc) => {
                let group_id = resolve_group_for_category(&repo, &doc.category);
                match repo.insert_doc(&doc, group_id) {
                    Ok(_) => imported_count += 1,
                    Err(e) => errors.push(format!("{}: {}", doc.title, e)),
                }
            }
            Err(e) => errors.push(e.to_string()),
        }
    } else {
        match parse_grammar_json(&content) {
            Ok(docs) => {
                for doc in &docs {
                    let group_id = resolve_group_for_category(&repo, &doc.category);
                    match repo.insert_doc(doc, group_id) {
                        Ok(_) => imported_count += 1,
                        Err(e) => errors.push(format!("{}: {}", doc.title, e)),
                    }
                }
            }
            Err(e) => errors.push(e.to_string()),
        }
    }

    Ok(GrammarImportResultDto {
        imported_count,
        errors,
    })
}

#[tauri::command]
pub fn list_grammar_groups() -> Result<Vec<GrammarGroupDto>, String> {
    let conn = init_db("reemember.db").map_err(|e| e.to_string())?;
    let repo = GrammarRepository::new(conn);
    repo.list_groups().map_err(|e| e.to_string()).map(|groups| {
        groups
            .into_iter()
            .map(|g| GrammarGroupDto {
                id: g.id,
                name: g.name,
                description: g.description,
                sort_order: g.sort_order,
                doc_count: g.doc_count,
                created_at: g.created_at,
            })
            .collect()
    })
}

#[tauri::command]
pub fn create_grammar_group(payload: CreateGrammarGroupRequest) -> Result<GrammarGroupDto, String> {
    let conn = init_db("reemember.db").map_err(|e| e.to_string())?;
    let repo = GrammarRepository::new(conn);
    let g = repo
        .create_group(&payload.name, payload.description.as_deref())
        .map_err(|e| e.to_string())?;
    Ok(GrammarGroupDto {
        id: g.id,
        name: g.name,
        description: g.description,
        sort_order: g.sort_order,
        doc_count: 0,
        created_at: g.created_at,
    })
}

#[tauri::command]
pub fn update_grammar_group(payload: UpdateGrammarGroupRequest) -> Result<(), String> {
    let conn = init_db("reemember.db").map_err(|e| e.to_string())?;
    let repo = GrammarRepository::new(conn);
    let desc_arg = payload.description.as_ref().map(|s| Some(s.as_str()));
    repo.update_group(payload.id, payload.name.as_deref(), desc_arg)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_grammar_group(id: i64) -> Result<(), String> {
    let conn = init_db("reemember.db").map_err(|e| e.to_string())?;
    let repo = GrammarRepository::new(conn);
    repo.delete_group(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn move_grammar_doc(payload: MoveGrammarDocRequest) -> Result<(), String> {
    let conn = init_db("reemember.db").map_err(|e| e.to_string())?;
    let repo = GrammarRepository::new(conn);
    repo.move_doc(payload.doc_id, payload.group_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_grammar_doc(id: i64) -> Result<(), String> {
    let conn = init_db("reemember.db").map_err(|e| e.to_string())?;
    let repo = GrammarRepository::new(conn);
    repo.delete_doc(id).map_err(|e| e.to_string())
}

// ─────────────────────────────────────────────────────────────────────────────

fn from_collection(c: Collection) -> CollectionDto {
    CollectionDto {
        id: c.id,
        name: c.name,
        description: c.description,
        created_at: c.created_at,
    }
}

fn from_topic(t: Topic) -> TopicDto {
    TopicDto {
        id: t.id,
        collection_id: t.collection_id,
        name: t.name,
        description: t.description,
    }
}
