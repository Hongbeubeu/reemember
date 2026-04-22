# Reemember — Technical Documentation

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Project Structure](#2-project-structure)
3. [Data Models](#3-data-models)
4. [Database Layer](#4-database-layer)
5. [Parser Layer](#5-parser-layer)
6. [Repository Layer](#6-repository-layer)
7. [Import / Export Layer](#7-import--export-layer)
8. [Testing Engine](#8-testing-engine)
9. [SRS Module](#9-srs-module)
10. [Service Layer](#10-service-layer)
11. [Tauri Desktop Layer](#11-tauri-desktop-layer)
12. [Error Handling](#12-error-handling)
13. [Test Coverage](#13-test-coverage)
14. [Dependencies](#14-dependencies)

---

## 1. Architecture Overview

Reemember follows a strict **layered architecture**. Each layer has a single responsibility and only depends on layers below it.

```
┌──────────────────────────────────────────────────────────┐
│              Tauri Desktop (src-tauri/)                  │
│     main.rs · commands.rs · tauri.conf.json              │
├──────────────────────────────────────────────────────────┤
│                   UI (ui/index.html)                     │
│           Single-page HTML/CSS/JS application            │
├──────────────────────────────────────────────────────────┤
│               Service Layer  (src/service/)              │
│           VocabularyService — unified high-level API     │
├────────────┬────────────────┬────────────────────────────┤
│ Import     │ Export         │ TestingEngine  │ SRS       │
│ Service    │ Service        │ (src/testing/) │ (src/srs/)│
│(src/import)│ (src/export/)  │                │           │
├────────────┴────────────────┴────────────────┴───────────┤
│              Repository  (src/repository/)               │
│       WordRepository · QueryOptions · SortBy             │
├──────────────────────────────────────────────────────────┤
│               Database Layer  (src/db/)                  │
│          schema.rs · merge.rs · mod.rs (DbError)         │
├──────────────────────────────────────────────────────────┤
│                Parser Layer  (src/parser/)               │
│              json.rs · csv.rs · ParseError               │
├──────────────────────────────────────────────────────────┤
│                 Core Models  (src/model.rs)               │
│    WordRecord · Definition · Metadata · Collection · Topic│
└──────────────────────────────────────────────────────────┘
```

**Key design decisions:**

- `WordRecord` is the canonical in-memory representation used across all layers.
- The `word_key` (lowercase-trimmed word) is the stable identity for upsert and lookup.
- All database mutations go through transactions for atomicity.
- Merge policy functions are pure (no DB access) and unit-testable in isolation.
- `RefCell<Connection>` gives interior mutability on the repository so it can be owned by the Tauri `State`.

---

## 2. Project Structure

```
reemember/
├── src/                        # Core Rust library (lib crate)
│   ├── lib.rs                  # Crate root, re-exports public API
│   ├── model.rs                # Core data structures
│   ├── db/
│   │   ├── mod.rs              # init_db(), init_memory_db(), DbError
│   │   ├── schema.rs           # CREATE TABLE / CREATE INDEX statements
│   │   └── merge.rs            # Pure merge policy functions
│   ├── parser/
│   │   ├── mod.rs              # ParseError umbrella type
│   │   ├── json.rs             # JSON parsing
│   │   └── csv.rs              # CSV parsing + CsvImportReport
│   ├── repository/
│   │   ├── mod.rs              # WordRepository (CRUD + SRS queries)
│   │   └── query.rs            # QueryOptions, SortBy
│   ├── import/
│   │   └── mod.rs              # ImportService, ImportReport
│   ├── export/
│   │   └── mod.rs              # ExportService
│   ├── testing/
│   │   └── mod.rs              # TestingEngine, Question, AnswerResult
│   ├── srs/
│   │   └── mod.rs              # compute_next_due()
│   └── service/
│       └── mod.rs              # VocabularyService (facade)
├── src-tauri/                  # Tauri desktop application
│   ├── src/
│   │   ├── main.rs             # Tauri app setup, window config
│   │   └── commands.rs         # IPC command handlers + DTOs
│   ├── Cargo.toml              # Tauri-specific dependencies
│   ├── tauri.conf.json         # App name, window size, bundle ID
│   ├── build.rs                # Tauri build script
│   └── capabilities/
│       └── default.json        # Capability declarations
├── ui/
│   └── index.html              # Single-page UI
├── tests/                      # Integration tests
│   ├── parser_phase1.rs
│   ├── repository_phase2.rs
│   ├── import_export_phase3.rs
│   ├── testing_engine_phase4.rs
│   └── srs_phase5.rs
└── Cargo.toml                  # Library crate manifest
```

---

## 3. Data Models

**File:** `src/model.rs`

### WordRecord

The central data structure. All layers read and write this type.

```rust
pub struct WordRecord {
    pub word: String,                  // Display form, case-preserved
    pub phonetic: Option<String>,      // IPA pronunciation string
    pub definitions: Vec<Definition>,  // At least one required
    pub examples: Vec<String>,
    pub synonyms: Vec<String>,
    pub antonyms: Vec<String>,
    pub family_words: Vec<String>,
    pub metadata: Metadata,
}
```

**`word_key()`** — normalizes `word` by trimming and lowercasing. Used as the unique key in the database (`word_key` column). Allows "Apple" and "apple" to resolve to the same record.

**`validate()`** — returns `Err(ValidationError)` if:
- `word` is blank
- `definitions` is empty
- All definitions have a blank `meaning`

### Definition

```rust
pub struct Definition {
    pub pos: Option<String>,  // Part of speech (e.g. "noun", "verb")
    pub meaning: String,      // Vietnamese meaning
}
```

Deduplication key is `(pos, meaning)` — two definitions are equal only if both fields match.

### Metadata

```rust
pub struct Metadata {
    pub tags: Vec<String>,
    pub created_at: Option<String>,  // RFC3339 timestamp
    pub review_count: u32,
}
```

`Metadata` derives `Default` — missing metadata in JSON/CSV produces zero-value metadata without error.

### Collection / Topic

```rust
pub struct Collection {
    pub id: i64,
    pub name: String,           // UNIQUE in database
    pub description: Option<String>,
    pub created_at: String,     // RFC3339
}

pub struct Topic {
    pub id: i64,
    pub collection_id: i64,     // FK to collections
    pub name: String,           // UNIQUE within collection
    pub description: Option<String>,
}
```

### ValidationError

```rust
pub enum ValidationError {
    MissingWord,
    MissingDefinition,
    MissingMeaning,
}
```

---

## 4. Database Layer

**Files:** `src/db/mod.rs`, `src/db/schema.rs`, `src/db/merge.rs`

### Schema

The database is SQLite. All tables are created with `IF NOT EXISTS` so `init_schema()` is idempotent.

#### Table: `words`

| Column | Type | Notes |
|--------|------|-------|
| `id` | INTEGER PK AUTOINCREMENT | Internal row ID |
| `word_key` | TEXT UNIQUE NOT NULL | Normalized key (`trim().to_lowercase()`) |
| `word` | TEXT NOT NULL | Display form |
| `phonetic` | TEXT | Nullable |
| `created_at` | TEXT | RFC3339, nullable |
| `review_count` | INTEGER DEFAULT 0 | |
| `updated_at` | TEXT NOT NULL | RFC3339, set on every upsert |

#### Table: `definitions`

Normalized: one row per definition. Cascades delete when parent word is deleted.

| Column | Type |
|--------|------|
| `id` | INTEGER PK |
| `word_id` | INTEGER FK → words.id ON DELETE CASCADE |
| `pos` | TEXT (nullable) |
| `meaning` | TEXT NOT NULL |

#### Table: `examples`

| Column | Type |
|--------|------|
| `id` | INTEGER PK |
| `word_id` | INTEGER FK → words.id ON DELETE CASCADE |
| `example` | TEXT NOT NULL |

#### Table: `tags` + `word_tags`

Tag dictionary + many-to-many junction. Tags are stored once in `tags(name UNIQUE)`, then referenced via `word_tags(word_id, tag_id)`.

#### Table: `review_schedule`

One row per word. `word_id` is the PRIMARY KEY (one due-date per word).

| Column | Type |
|--------|------|
| `word_id` | INTEGER PK FK → words.id |
| `due_at` | TEXT NOT NULL (RFC3339) |

On insert, the initial `due_at` is set to `now`. The SRS module updates this after each review.

#### Table: `review_history`

Append-only event log. Never updated or deleted (except cascading word delete).

| Column | Type |
|--------|------|
| `id` | INTEGER PK |
| `word_id` | INTEGER FK → words.id |
| `reviewed_at` | TEXT NOT NULL (RFC3339) |
| `was_correct` | INTEGER NOT NULL (0 or 1) |

#### Table: `word_relations`

Stores synonyms, antonyms, and family words.

| Column | Type | Notes |
|--------|------|-------|
| `word_id` | INTEGER FK → words.id | |
| `related_word` | TEXT | The related word string |
| `relation_type` | TEXT | CHECK: `'synonym'`, `'antonym'`, `'family'` |
| PK | `(word_id, related_word, relation_type)` | |

#### Tables: `collections`, `topics`, `word_topics`

Organization hierarchy. `topics.name` is UNIQUE within a `collection_id`. `word_topics` is a junction table (word_id, topic_id PK).

### Indexes

| Index | On |
|-------|----|
| `idx_words_word_key` | `words(word_key)` |
| `idx_definitions_word_id` | `definitions(word_id)` |
| `idx_examples_word_id` | `examples(word_id)` |
| `idx_word_tags_word_id` | `word_tags(word_id)` |
| `idx_word_tags_tag_id` | `word_tags(tag_id)` |
| `idx_review_schedule_due_at` | `review_schedule(due_at)` |
| `idx_review_history_word_id` | `review_history(word_id)` |
| `idx_review_history_reviewed_at` | `review_history(reviewed_at)` |
| `idx_word_relations_word_id` | `word_relations(word_id)` |
| `idx_topics_collection_id` | `topics(collection_id)` |
| `idx_word_topics_topic_id` | `word_topics(topic_id)` |
| `idx_word_topics_word_id` | `word_topics(word_id)` |

### Database Initialization

```rust
// File database (persistent)
pub fn init_db(path: &str) -> Result<Connection, DbError>

// In-memory database (tests only)
pub fn init_memory_db() -> Result<Connection, DbError>
```

Both call `init_schema()` after opening the connection.

### Merge Policy (`src/db/merge.rs`)

All functions are pure (no I/O). They implement the data-preservation rules applied during upsert.

| Function | Rule |
|----------|------|
| `merge_phonetic(old, new)` | Use `new` if non-empty; otherwise keep `old` |
| `merge_definitions(old, new)` | Concatenate; deduplicate by `(pos, meaning)` |
| `merge_examples(old, new)` | Concatenate; deduplicate by exact string |
| `merge_tags(old, new)` | Concatenate; deduplicate; sort alphabetically |
| `merge_created_at(old, new)` | Keep the lexicographically earlier timestamp (earliest date) |
| `merge_review_count(old, new)` | `max(old, new)` |
| `merge_string_vec(old, new)` | Concatenate; deduplicate; preserve insertion order |

---

## 5. Parser Layer

**Files:** `src/parser/json.rs`, `src/parser/csv.rs`, `src/parser/mod.rs`

### JSON Parser

```rust
// Parse a flat JSON array: [{ "word": ..., "definitions": [...], ... }, ...]
pub fn parse_json_canonical(input: &str) -> Result<Vec<WordRecord>, ParseError>

// Parse a bundle with optional collection/topic wrapper
pub fn parse_json_bundle(input: &str) -> Result<Vec<WordRecord>, ParseError>
```

`parse_json_canonical` deserializes directly via `serde_json`. Each record is validated after deserialization; invalid records include their 1-based index in the error message.

`parse_json_bundle` accepts either a bare array or an object with a `words` key (and optional `collection`, `topic` fields for scoped import).

### CSV Parser

```rust
pub fn parse_csv(input: &str) -> CsvImportReport
```

Returns a report rather than a `Result` — per-row errors are collected instead of aborting.

```rust
pub struct CsvImportReport {
    pub records: Vec<WordRecord>,
    pub errors: Vec<CsvRowError>,
}

pub struct CsvRowError {
    pub row: usize,
    pub message: String,
}
```

**Supported column sets:**

| Mode | Required columns |
|------|-----------------|
| Minimal | `word`, `meaning_vi` |
| Extended | `word`, `phonetic`, `meaning_vi`, `pos`, `examples`, `synonyms`, `antonyms`, `family_words`, `tags`, `review_count` |

Multi-value fields in extended CSV use `|` as the delimiter: `"transient|fleeting"`.

Blank rows are skipped silently. Rows where `word` or `meaning_vi` are empty are recorded as `CsvRowError`.

### ParseError

```rust
pub enum ParseError {
    Json(serde_json::Error),
    Csv(csv::Error),
    InvalidData(String),   // validation error with context
}
```

---

## 6. Repository Layer

**Files:** `src/repository/mod.rs`, `src/repository/query.rs`

### WordRepository

```rust
pub struct WordRepository {
    conn: RefCell<Connection>,  // Interior mutability — single-threaded use
}
```

`RefCell` is used because Tauri commands receive `&State<WordRepository>` (shared reference), but SQLite operations require `&mut`. This is safe for single-threaded Tauri invocations.

#### Word CRUD

```rust
pub fn upsert(&self, record: &WordRecord) -> Result<UpsertResult, DbError>
pub fn get_by_word_key(&self, word_key: &str) -> Result<Option<WordRecord>, DbError>
pub fn delete_by_word_key(&self, word_key: &str) -> Result<(), DbError>
```

**Upsert flow:**
1. Validate the record.
2. Look up existing record by `word_key`.
3. If exists: merge using merge policy functions → UPDATE + replace child rows.
4. If new: INSERT word + INSERT all child rows + initialize `review_schedule`.
5. Commit transaction.

All child rows (definitions, examples, tags, relations) are written within the same transaction.

```rust
pub struct UpsertResult {
    pub inserted: bool,
    pub definitions_count: usize,
    pub examples_count: usize,
    pub tags_count: usize,
}
```

#### Query

```rust
pub fn query(&self, options: &QueryOptions) -> Result<Vec<WordRecord>, DbError>
```

Builds a dynamic SQL query from `QueryOptions`. Joins `word_tags`/`tags` when `tag_filter` is set; joins `word_topics`/`topics` when `topic_id` or `collection_id` is set. Uses `DISTINCT` to avoid duplicates from joins.

```rust
pub struct QueryOptions {
    pub sort: Option<SortBy>,
    pub limit: Option<usize>,
    pub tag_filter: Option<String>,
    pub collection_id: Option<i64>,
    pub topic_id: Option<i64>,
}

pub enum SortBy {
    Word,           // ORDER BY words.word ASC
    CreatedAt,      // ORDER BY words.created_at DESC
    ReviewCount,    // ORDER BY words.review_count DESC
}
```

#### Learning / SRS Queries

```rust
// Pick a random word (optionally scoped to a topic)
pub fn pick_random_word(&self) -> Result<Option<WordRecord>, DbError>
pub fn pick_random_word_scoped(&self, topic_id: Option<i64>) -> Result<Option<WordRecord>, DbError>

// Pick the overdue word with the earliest due_at
pub fn pick_due_word(&self, now_utc: &str) -> Result<Option<WordRecord>, DbError>
pub fn pick_due_word_scoped(&self, now_utc: &str, topic_id: Option<i64>) -> Result<Option<WordRecord>, DbError>

// SRS-aware: return due word if available, else random
pub fn pick_next_word(&self, srs_enabled: bool, now_utc: &str) -> Result<Option<WordRecord>, DbError>
pub fn pick_next_word_scoped(&self, srs_enabled: bool, now_utc: &str, topic_id: Option<i64>) -> Result<Option<WordRecord>, DbError>
```

#### Review Tracking

```rust
pub fn increment_review_count_by_word_key(&self, word_key: &str) -> Result<(), DbError>
pub fn record_review_event(&self, word_key: &str, was_correct: bool, reviewed_at: &str) -> Result<(), DbError>
pub fn review_history_count_by_word_key(&self, word_key: &str) -> Result<u32, DbError>
pub fn set_due_at_by_word_key(&self, word_key: &str, due_at: &str) -> Result<(), DbError>
pub fn get_due_at_by_word_key(&self, word_key: &str) -> Result<Option<String>, DbError>
```

`set_due_at_by_word_key` uses `INSERT ... ON CONFLICT DO UPDATE` (upsert) so it works for both initial creation and rescheduling.

#### Collections / Topics

```rust
pub fn list_collections(&self) -> Result<Vec<Collection>, DbError>
pub fn create_collection(&self, name: &str, description: Option<&str>) -> Result<Collection, DbError>
pub fn update_collection(&self, id: i64, name: &str, description: Option<&str>) -> Result<(), DbError>
pub fn delete_collection(&self, id: i64) -> Result<(), DbError>
pub fn find_or_create_collection(&self, name: &str) -> Result<Collection, DbError>

pub fn list_topics(&self, collection_id: i64) -> Result<Vec<Topic>, DbError>
pub fn create_topic(&self, collection_id: i64, name: &str, description: Option<&str>) -> Result<Topic, DbError>
pub fn update_topic(&self, id: i64, name: &str, description: Option<&str>) -> Result<(), DbError>
pub fn delete_topic(&self, id: i64) -> Result<(), DbError>
pub fn find_or_create_topic(&self, collection_id: i64, name: &str) -> Result<Topic, DbError>

pub fn assign_word_to_topic(&self, word_key: &str, topic_id: i64) -> Result<(), DbError>
pub fn get_word_topics(&self, word_key: &str) -> Result<Vec<Topic>, DbError>
```

Deleting a collection cascades to its topics via `ON DELETE CASCADE`. Deleting a topic cascades to `word_topics`.

---

## 7. Import / Export Layer

**Files:** `src/import/mod.rs`, `src/export/mod.rs`

### ImportService

```rust
pub struct ImportReport {
    pub inserted_count: usize,
    pub updated_count: usize,
    pub skipped_count: usize,
    pub csv_errors: Vec<CsvRowError>,
}
```

```rust
// Import JSON (flat array format)
pub fn import_from_json_string(repo: &WordRepository, json: &str) -> Result<ImportReport, ImportError>

// Import JSON and assign all words to a collection/topic
pub fn import_from_json_string_scoped(
    repo: &WordRepository,
    json: &str,
    collection_name: &str,
    topic_name: &str,
) -> Result<ImportReport, ImportError>

// Import CSV (collects per-row errors instead of aborting)
pub fn import_from_csv_string(repo: &WordRepository, csv: &str) -> Result<ImportReport, ImportError>
```

For each parsed record, `upsert()` is called. `UpsertResult.inserted` determines whether the record counts toward `inserted_count` or `updated_count`. Parse errors that produce no record increment `skipped_count`.

### ExportService

```rust
// Full round-trip JSON backup (canonical format)
pub fn export_to_json_string(repo: &WordRepository) -> Result<String, ExportError>

// Extended CSV export
pub fn export_to_csv_string(repo: &WordRepository) -> Result<String, ExportError>
```

`export_to_json_string` calls `repo.query(&QueryOptions::default())` to fetch all records, then serializes via `serde_json::to_string_pretty`. The output is valid input for `import_from_json_string`.

`export_to_csv_string` writes the extended header row then one row per word. Multi-value fields are joined with `|`. Fields containing commas, quotes, or newlines are wrapped in double-quotes with internal quotes escaped.

---

## 8. Testing Engine

**File:** `src/testing/mod.rs`

### Types

```rust
pub enum TestMode {
    EnVi,    // English word → Vietnamese meaning
    ViEn,    // Vietnamese meaning → English word
    Hybrid,  // Random selection per question
}

pub struct Question {
    pub word_key: String,
    pub prompt: String,              // What is shown to the user
    pub examples: Vec<String>,       // With target word masked
    pub accepted_answers: Vec<String>,
}

pub struct AnswerResult {
    pub correct: bool,
    pub normalized_answer: String,
    pub accepted_answers: Vec<String>,
}
```

### TestingEngine

```rust
pub struct TestingEngine;

impl TestingEngine {
    pub fn generate_question(
        &self,
        repo: &WordRepository,
        mode: &TestMode,
        srs_enabled: bool,
        now_utc: &str,
    ) -> Result<Option<Question>, DbError>

    pub fn submit_answer(
        &self,
        repo: &WordRepository,
        word_key: &str,
        user_answer: &str,
        accepted_answers: &[String],
        srs_enabled: bool,
        now_utc: &str,
    ) -> Result<AnswerResult, DbError>
}
```

**`generate_question` flow:**
1. Call `repo.pick_next_word(srs_enabled, now_utc)`.
2. Determine direction from `TestMode` (Hybrid picks randomly).
3. Build `prompt` from the word (EN→VI) or first definition meaning (VI→EN).
4. Build `accepted_answers` from all definition meanings.
5. Mask the target word in example sentences (replace with `___`).

**`submit_answer` flow:**
1. Normalize both the user answer and each accepted answer: `trim().to_lowercase()`, collapse consecutive spaces.
2. Compare normalized user answer against normalized accepted answers.
3. Call `repo.increment_review_count_by_word_key()`.
4. Call `repo.record_review_event()`.
5. If SRS enabled: compute `next_due = srs::compute_next_due(now, was_correct, review_count)`, call `repo.set_due_at_by_word_key()`.
6. Return `AnswerResult`.

---

## 9. SRS Module

**File:** `src/srs/mod.rs`

```rust
pub fn compute_next_due(now: DateTime<Utc>, was_correct: bool, review_count: u32) -> String
```

Returns an RFC3339 timestamp for the next review.

**Interval schedule:**

| Condition | Next review |
|-----------|-------------|
| Wrong answer | 10 minutes |
| Correct, `review_count` 0–1 | 1 day |
| Correct, `review_count` 2–3 | 3 days |
| Correct, `review_count` 4–6 | 7 days |
| Correct, `review_count` ≥ 7 | 14 days |

The `review_count` value used is the count *before* incrementing in the current session. The interval grows as the user demonstrates consistent recall.

---

## 10. Service Layer

**File:** `src/service/mod.rs`

`VocabularyService` is a facade that owns a `WordRepository` and exposes the operations needed by Tauri commands in a single type:

```rust
pub struct VocabularyService {
    repo: WordRepository,
    engine: TestingEngine,
}
```

It re-exports import, export, testing, and SRS operations without the caller needing to know about the underlying modules. The Tauri `State` holds one instance of this type per application lifetime.

---

## 11. Tauri Desktop Layer

**Files:** `src-tauri/src/main.rs`, `src-tauri/src/commands.rs`

### Application Setup (`main.rs`)

```rust
tauri::Builder::default()
    .plugin(tauri_plugin_dialog::init())
    .manage(/* VocabularyService state */)
    .invoke_handler(tauri::generate_handler![
        next_question, submit_answer,
        list_words, delete_word,
        list_collections, create_collection, update_collection, delete_collection,
        list_topics, create_topic, update_topic, delete_topic, assign_word_to_topic,
        import_vocabulary, save_export,
    ])
    .run(tauri::generate_context!())
```

Window configuration (`tauri.conf.json`): title `"Reemember"`, size 1000×750, resizable.

### IPC Commands (`commands.rs`)

All commands are `#[tauri::command]` async functions that receive `State<VocabularyService>`.

| Command | Parameters | Returns |
|---------|-----------|---------|
| `next_question` | `srs_enabled: bool` | `Option<QuestionDto>` |
| `submit_answer` | `word_key, answer, accepted_answers, srs_enabled` | `AnswerResultDto` |
| `list_words` | `tag?, collection_id?, topic_id?` | `Vec<WordSummaryDto>` |
| `delete_word` | `word_key` | `()` |
| `list_collections` | — | `Vec<CollectionDto>` |
| `create_collection` | `name, description?` | `CollectionDto` |
| `update_collection` | `id, name, description?` | `()` |
| `delete_collection` | `id` | `()` |
| `list_topics` | `collection_id` | `Vec<TopicDto>` |
| `create_topic` | `collection_id, name, description?` | `TopicDto` |
| `update_topic` | `id, name, description?` | `()` |
| `delete_topic` | `id` | `()` |
| `assign_word_to_topic` | `word_key, topic_id` | `()` |
| `import_vocabulary` | `content: String, format: String` | `ImportReportDto` |
| `save_export` | `format: String` | `()` (opens file dialog) |

### DTOs

DTOs are `#[derive(Serialize, Deserialize)]` structs that decouple the Tauri JSON wire format from internal types.

```rust
pub struct QuestionDto {
    pub word_key: String,
    pub prompt: String,
    pub examples: Vec<String>,
    pub accepted_answers: Vec<String>,
}

pub struct AnswerResultDto {
    pub correct: bool,
    pub normalized_answer: String,
    pub accepted_answers: Vec<String>,
}

pub struct WordSummaryDto {
    pub word: String,
    pub word_key: String,
    pub phonetic: Option<String>,
    pub first_meaning: String,
    pub review_count: u32,
    pub tags: Vec<String>,
}

pub struct CollectionDto { pub id: i64, pub name: String, pub description: Option<String> }
pub struct TopicDto { pub id: i64, pub collection_id: i64, pub name: String, pub description: Option<String> }

pub struct ImportReportDto {
    pub inserted_count: usize,
    pub updated_count: usize,
    pub skipped_count: usize,
    pub error_count: usize,
}
```

---

## 12. Error Handling

The project uses `thiserror` for all error types. Each layer defines its own error type.

### DbError (`src/db/mod.rs`)

```rust
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Merge error: {0}")]
    Merge(String),

    #[error("Validation error: {0}")]
    Validation(String),
}
```

### ParseError (`src/parser/mod.rs`)

```rust
pub enum ParseError {
    Json(serde_json::Error),
    Csv(csv::Error),
    InvalidData(String),
}
```

CSV parsing uses `CsvImportReport` (collects per-row errors) instead of a `Result`. `ParseError` is only returned for fatal structural errors.

### Tauri command errors

Tauri commands return `Result<T, String>`. All `DbError` / `ImportError` values are converted to `String` via `.to_string()` before returning to the frontend. The frontend displays the error string in a notification.

---

## 13. Test Coverage

Integration tests live in `tests/`. Unit tests live inline in source files (`#[cfg(test)]` modules).

### Integration Tests

| File | Phase | Coverage |
|------|-------|----------|
| `parser_phase1.rs` | 1 | JSON canonical parsing, CSV minimal/extended, missing fields, `|` splitting, blank row skip, review_count validation |
| `repository_phase2.rs` | 2 | Upsert insert/update, merge for phonetic/definitions/examples/tags/created_at/review_count/synonyms, query sort, Collection CRUD, Topic CRUD, word-topic assignment |
| `import_export_phase3.rs` | 3 | Import report counts, CSV error collection, JSON round-trip export |
| `testing_engine_phase4.rs` | 4 | Answer normalization, accepted_answers extraction, EN→VI/VI→EN question generation, grading |
| `srs_phase5.rs` | 5 | Wrong-answer interval (10 min), correct-answer interval growth by review_count |

### Unit Tests (inline)

| File | Tests |
|------|-------|
| `src/db/merge.rs` | 6 tests covering all merge functions |
| `src/repository/mod.rs` | 7 tests: upsert, merge phonetic/synonyms, get, Collection CRUD, Topic CRUD, word-topic |
| `src/srs/mod.rs` | 2 tests: wrong-answer timing, interval growth |

### Running tests

```bash
# All tests
cargo test

# Specific integration test file
cargo test --test repository_phase2

# Specific test by name
cargo test test_upsert_merge_phonetic
```

---

## 14. Dependencies

### Library crate (`Cargo.toml`)

| Crate | Version | Purpose |
|-------|---------|---------|
| `serde` | 1.0 | Serialization/deserialization framework |
| `serde_json` | 1.0 | JSON support |
| `csv` | 1.0 | CSV parsing |
| `rusqlite` | 0.30 | SQLite bindings; features: `bundled`, `chrono` |
| `chrono` | 0.4 | DateTime handling; features: `serde` |
| `thiserror` | 1.0 | Error derive macro |

The `bundled` feature on `rusqlite` statically links SQLite — no external SQLite installation required on the user's machine.

### Tauri crate (`src-tauri/Cargo.toml`)

| Crate | Version | Purpose |
|-------|---------|---------|
| `tauri` | 2.0 | Desktop application framework |
| `tauri-plugin-dialog` | 2.0 | Native file open/save dialogs |
| `reemember` | path | The library crate above |

### Rust edition

Both crates use **Rust edition 2021**.
