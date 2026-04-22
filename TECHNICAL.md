# Reemember - Technical Documentation

This document describes the current implementation in `reemember/`.

## 1) Product Scope

- Desktop vocabulary app built with Tauri
- Local persistence with SQLite
- Study engine with optional SRS scheduling
- JSON import and JSON export

Primary run path:

```bash
cd src-tauri
cargo run
```

## 2) Workspace Structure

```text
reemember/
|- src/                    # Core Rust library
|  |- lib.rs
|  |- model.rs
|  |- db/
|  |  |- mod.rs
|  |  |- schema.rs
|  |  `- merge.rs
|  |- parser/
|  |  |- mod.rs
|  |  `- json.rs
|  |- repository/
|  |  |- mod.rs
|  |  `- query.rs
|  |- import/mod.rs
|  |- export/mod.rs
|  |- testing/mod.rs
|  |- srs/mod.rs
|  `- service/mod.rs
|- src-tauri/
|  |- Cargo.toml
|  |- src/
|  |  |- main.rs
|  |  `- commands.rs
|  `- tauri.conf.json
|- tests/
`- ui/index.html
```

## 3) Architecture Overview

Layered flow:

1. `ui/index.html` (single-page UI)
2. `src-tauri/src/commands.rs` (IPC commands)
3. `src/service/mod.rs` (`VocabularyService` facade)
4. Domain modules (`import`, `export`, `testing`, `srs`)
5. `src/repository/mod.rs` (`WordRepository`)
6. `src/db/*` (schema + merge + DB init)
7. `src/model.rs` (core models)

Runtime detail:

- Tauri commands initialize DB per command via `init_db("reemember.db")` in `src-tauri/src/commands.rs`.

## 4) Core Data Models

Defined in `src/model.rs`.

### `WordRecord`

```rust
pub struct WordRecord {
    pub word: String,
    pub phonetic: Option<String>,
    pub definitions: Vec<Definition>,
    pub examples: Vec<String>,
    pub synonyms: Vec<String>,
    pub antonyms: Vec<String>,
    pub family_words: Vec<String>,
    pub metadata: Metadata,
}
```

Rules:

- Stable identity key: `word_key()` (`trim().to_lowercase()`)
- Validation requires non-empty `word`
- Validation requires non-empty `definitions`
- Validation requires at least one non-empty `definitions[].meaning`

### Related structs

- `Definition { pos: Option<String>, meaning: String }`
- `Metadata { tags, created_at, review_count }`
- `Collection`
- `Topic`

## 5) Database Layer

Files: `src/db/mod.rs`, `src/db/schema.rs`, `src/db/merge.rs`

### Initialization

- `init_db(path)` opens file DB, enables foreign keys, applies schema
- `init_memory_db()` sets up in-memory DB for tests

### Main tables

- `words`
- `definitions`
- `examples`
- `tags`, `word_tags`
- `review_schedule`
- `review_history`
- `word_relations`
- `collections`, `topics`, `word_topics`

### Merge policy helpers

In `src/db/merge.rs`:

- `merge_phonetic`
- `merge_definitions`
- `merge_examples`
- `merge_tags`
- `merge_created_at`
- `merge_review_count`
- `merge_string_vec`

## 6) Parser Layer

Files: `src/parser/mod.rs`, `src/parser/json.rs`

Public API:

- `parse_json_canonical(input) -> Result<Vec<WordRecord>, ParseError>`
- `parse_json_bundle(input) -> Result<BundleImport, ParseError>`

`BundleImport` structure:

```json
{
  "collection": "optional",
  "topic": "optional",
  "words": [ ... ]
}
```

Accepted payloads:

- Flat array of `WordRecord`
- Bundle object with `words`

`ParseError` variants:

- `Json(serde_json::Error)`
- `InvalidData(String)`

## 7) Repository Layer

Files: `src/repository/mod.rs`, `src/repository/query.rs`

`WordRepository` contains `RefCell<Connection>` for interior mutability.

### Main operations

- CRUD: `upsert`, `get_by_word_key`, `delete_by_word_key`
- Query: `query(&QueryOptions)`
- Review/SRS:
  - `pick_random_word_scoped`
  - `pick_due_word_scoped`
  - `pick_next_word_scoped`
  - `increment_review_count_by_word_key`
  - `record_review_event`
  - `set_due_at_by_word_key`
  - `get_due_at_by_word_key`
- Organization:
  - collections/topics CRUD
  - `assign_word_to_topic`
  - `get_word_topics`

### Query options

From `src/repository/query.rs`:

- `SortBy::{Word, CreatedAt, ReviewCount}`
- `QueryOptions { sort, limit, tag_filter, collection_id, topic_id }`

## 8) Import / Export Layer

### Import (`src/import/mod.rs`)

- `ImportService::import_from_json_string`
- `ImportService::import_from_json_string_scoped`

Behavior:

- Parse JSON payload
- Resolve optional collection/topic scope
- Upsert records
- Return `ImportReport` with inserted/updated/skipped counters and row-level results

### Export (`src/export/mod.rs`)

- `ExportService::export_to_json_string`

Behavior:

- Query records from repository
- Serialize pretty JSON via `serde_json`

## 9) Testing Engine + SRS

### Testing (`src/testing/mod.rs`)

- Modes: `TestMode::{EnVi, ViEn, Hybrid}`
- Selection uses `pick_next_word_scoped` with `TestingOptions`
- Answer normalization: whitespace collapse + lowercase

Current grading/write behavior:

- Correctness is checked against normalized `expected_answers`
- `review_count` increments on incorrect answers
- A review event is recorded for each submission
- With SRS enabled, next due timestamp is recomputed and stored

### SRS (`src/srs/mod.rs`)

`compute_next_due(now, was_correct, review_count)` intervals:

- Wrong -> +10 minutes
- Correct, `0..=1` -> +1 day
- Correct, `2..=3` -> +3 days
- Correct, `4..=6` -> +7 days
- Correct, `>=7` -> +14 days

## 10) Tauri Desktop Layer

Files: `src-tauri/src/main.rs`, `src-tauri/src/commands.rs`

### Entrypoint

`src-tauri/src/main.rs`:

- Registers command handlers
- Creates main window for `index.html`
- Sets window title and dimensions

### Command surface

- Study: `next_question`, `submit_answer`
- Library: `list_words`, `delete_word`
- Collections: `list_collections`, `create_collection`, `update_collection`, `delete_collection`
- Topics: `list_topics`, `create_topic`, `update_topic`, `delete_topic`, `assign_word_to_topic`
- Data: `import_vocabulary`, `save_export`

## 11) Error Handling

Core error type: `DbError` in `src/db/mod.rs`:

- `Sqlite(rusqlite::Error)`
- `Parse(crate::parser::ParseError)`
- `Merge(String)`
- `Validation(String)`

Tauri commands convert internal errors to `String` for frontend responses.

## 12) Tests

Integration tests in `tests/`:

- `parser_phase1.rs`
- `repository_phase2.rs`
- `import_export_phase3.rs`
- `testing_engine_phase4.rs`
- `srs_phase5.rs`

Run all tests:

```bash
cargo test
```

## 13) Dependencies and Editions

### Root crate (`Cargo.toml`)

- `serde`
- `serde_json`
- `thiserror`
- `rusqlite` (`bundled`, `chrono`)
- `chrono` (`serde`)

### Tauri crate (`src-tauri/Cargo.toml`)

- `tauri`
- `tauri-build` (build-dependency)
- `tauri-plugin-dialog`
- local dependency: `reemember`

### Rust edition

- Root crate: `edition = "2024"`
- Tauri crate: `edition = "2024"`

## 14) Documentation Maintenance

When updating docs, verify against source files:

- parser behavior: `src/parser/`
- import/export command behavior: `src-tauri/src/commands.rs`
- runtime entrypoint: `src-tauri/src/main.rs`
- dependencies and edition: `Cargo.toml`, `src-tauri/Cargo.toml`
