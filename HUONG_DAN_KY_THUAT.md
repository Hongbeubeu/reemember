# Hướng Dẫn Kỹ Thuật — Reemember

> Tài liệu này giải thích toàn bộ codebase theo ngôn ngữ của người đọc code lần đầu.
> Code comment viết bằng tiếng Anh. Giải thích trong doc này viết bằng tiếng Việt.

---

## Mục lục

1. [Project là gì?](#1-project-là-gì)
2. [Công nghệ sử dụng](#2-công-nghệ-sử-dụng)
3. [Cấu trúc thư mục](#3-cấu-trúc-thư-mục)
4. [Rust tổ chức code như thế nào?](#4-rust-tổ-chức-code-như-thế-nào)
5. [Sơ đồ kiến trúc tổng thể](#5-sơ-đồ-kiến-trúc-tổng-thể)
6. [Lớp Model — Dữ liệu trông như thế nào](#6-lớp-model--dữ-liệu-trông-như-thế-nào)
7. [Lớp Database — Lưu trữ dữ liệu](#7-lớp-database--lưu-trữ-dữ-liệu)
8. [Lớp Repository — Truy vấn dữ liệu](#8-lớp-repository--truy-vấn-dữ-liệu)
9. [Lớp Parser — Đọc file JSON/MD](#9-lớp-parser--đọc-file-jsonmd)
10. [Lớp Service — Logic nghiệp vụ](#10-lớp-service--logic-nghiệp-vụ)
11. [Lớp Testing & SRS — Engine học từ vựng](#11-lớp-testing--srs--engine-học-từ-vựng)
12. [Module Grammar & Bilingual](#12-module-grammar--bilingual)
13. [Lớp Commands — Cầu nối Backend↔Frontend](#13-lớp-commands--cầu-nối-backendfrontend)
14. [Frontend — Giao diện người dùng](#14-frontend--giao-diện-người-dùng)
15. [Dữ liệu mẫu (Data files)](#15-dữ-liệu-mẫu-data-files)
16. [Cách thêm tính năng mới](#16-cách-thêm-tính-năng-mới)

---

## 1. Project là gì?

**Reemember** là ứng dụng học từ vựng tiếng Anh chạy trên desktop (macOS/Windows/Linux). Người dùng có thể:

- Import từ vựng từ file JSON
- Luyện tập theo kiểu flashcard: nhìn từ tiếng Anh → nhập nghĩa tiếng Việt (và ngược lại)
- Dùng **SRS** (Spaced Repetition System) — hệ thống nhắc ôn từ theo khoảng cách thời gian tối ưu
- Đọc bài song ngữ Anh-Việt
- Xem và luyện bài tập ngữ pháp

---

## 2. Công nghệ sử dụng

| Thành phần | Công nghệ | Vai trò |
|---|---|---|
| Backend | **Rust** | Xử lý logic, lưu trữ dữ liệu |
| Desktop shell | **Tauri** | Đóng gói Rust thành app desktop |
| Database | **SQLite** | Lưu dữ liệu local (file `.db`) |
| Frontend | **HTML/CSS/JS** thuần | Giao diện người dùng (1 file duy nhất) |
| Thư viện Rust chính | `rusqlite`, `serde`, `chrono`, `thiserror` | DB, JSON, thời gian, xử lý lỗi |

**Tại sao Tauri?** Tauri cho phép viết UI bằng web (HTML/JS) nhưng backend là Rust native — nhẹ hơn Electron nhiều lần vì không đóng gói cả trình duyệt Chrome vào app.

---

## 3. Cấu trúc thư mục

```
reemember/
│
├── src/                        ← Thư viện Rust core (logic chính)
│   ├── lib.rs                  ← Khai báo các module
│   ├── model.rs                ← Kiểu dữ liệu (WordRecord, Definition,...)
│   ├── db/                     ← Database: kết nối, schema, migration
│   ├── repository/             ← Truy vấn dữ liệu (CRUD)
│   ├── parser/                 ← Đọc/parse file JSON
│   ├── import/                 ← Logic import từ vựng vào DB
│   ├── export/                 ← Logic export từ vựng ra JSON
│   ├── service/                ← Facade gom các tính năng lại
│   ├── testing/                ← Engine sinh câu hỏi và chấm điểm
│   ├── srs/                    ← Tính toán lịch ôn tập (SRS)
│   ├── grammar/                ← Xử lý bài học ngữ pháp
│   └── bilingual/              ← Xử lý bài đọc song ngữ
│
├── src-tauri/                  ← Ứng dụng Tauri (shell desktop)
│   ├── src/
│   │   ├── main.rs             ← Điểm khởi chạy app, đăng ký commands
│   │   └── commands.rs         ← 30 Tauri commands (API cho frontend)
│   ├── Cargo.toml              ← Dependencies của Tauri app
│   └── reemember.db            ← File SQLite (tạo lúc runtime)
│
├── ui/
│   └── index.html              ← Toàn bộ frontend (1 file HTML+CSS+JS)
│
├── vocabulary_data/            ← File JSON từ vựng mẫu
│   └── daily_life/             ← ~20 file JSON theo chủ đề
├── grammar_data/               ← File Markdown bài học ngữ pháp (~40 file)
├── bilingual_data/             ← File JSON bài đọc song ngữ
│
├── tests/                      ← Integration tests
├── Cargo.toml                  ← Workspace manifest (khai báo 2 crate)
└── HUONG_DAN_KY_THUAT.md      ← File này
```

---

## 4. Rust tổ chức code như thế nào?

Đây là phần quan trọng nhất để hiểu codebase. Rust có một số khái niệm khác với Python/JavaScript.

### 4.1 Crate là gì?

**Crate** = một đơn vị biên dịch trong Rust (tương đương "package" trong Python hay "npm package" trong JS).

Project này có **2 crate** trong một **workspace**:

```toml
# Cargo.toml (file gốc)
[workspace]
members = [".", "src-tauri"]   # 2 crate: thư viện core + app Tauri
```

- **Crate 1**: `reemember` (thư mục gốc `src/`) — thư viện chứa toàn bộ logic
- **Crate 2**: `reemember-tauri` (thư mục `src-tauri/`) — app desktop, *phụ thuộc vào* crate 1

```toml
# src-tauri/Cargo.toml
[dependencies]
reemember = { path = ".." }   # Crate 2 dùng Crate 1 như một thư viện
```

### 4.2 Module là gì?

**Module** = cách Rust tổ chức code trong một crate (tương đương "folder/file" trong Python, hoặc ES module trong JS).

File `src/lib.rs` là **cửa vào** của thư viện:

```rust
// src/lib.rs — declares which modules this crate exposes
pub mod model;       // src/model.rs
pub mod parser;      // src/parser/mod.rs
pub mod db;          // src/db/mod.rs
pub mod repository;  // src/repository/mod.rs
pub mod import;      // src/import/mod.rs
pub mod export;      // src/export/mod.rs
pub mod service;     // src/service/mod.rs
pub mod testing;     // src/testing/mod.rs
pub mod srs;         // src/srs/mod.rs
pub mod grammar;     // src/grammar/mod.rs
pub mod bilingual;   // src/bilingual/mod.rs
```

**Quy tắc tên file:**
- `src/model.rs` → module `model`
- `src/db/mod.rs` → module `db` (khi module có nhiều file con thì dùng thư mục + `mod.rs`)
- `src/db/schema.rs` → sub-module `db::schema`

### 4.3 pub là gì?

`pub` = "public" — cho phép code bên ngoài module dùng. Không có `pub` thì chỉ dùng trong nội bộ module.

```rust
pub struct WordRecord { ... }  // Có thể dùng từ bên ngoài
pub fn init_db(...) { ... }    // Hàm public

fn calculate_streak(...) { ... } // Private, chỉ dùng trong module đó
```

### 4.4 impl là gì?

`impl` = "implementation" — nơi khai báo methods (hàm) cho một struct (tương đương class method trong Python/JS).

```rust
// Define the struct (like a class without methods)
pub struct WordRepository {
    conn: RefCell<Connection>,
}

// Add methods to the struct
impl WordRepository {
    pub fn new(conn: Connection) -> Self { ... }  // Constructor
    pub fn get_by_word_key(&self, key: &str) -> Result<Option<WordRecord>, DbError> { ... }
}
```

### 4.5 Result và ? là gì?

Rust không có `try/catch`. Thay vào đó dùng `Result<T, E>`:

```rust
// Result<T, E> means: either Ok(T) on success, or Err(E) on failure
fn init_db(path: &str) -> Result<Connection, DbError> {
    let conn = Connection::open(path)?;  // ? = if error, return early with Err
    //                                        if ok, unwrap the value
    Ok(conn)
}
```

Dấu `?` là "propagate error" — nếu có lỗi thì trả về lỗi ngay, tương đương:
```rust
// Without ?, you'd write:
let conn = match Connection::open(path) {
    Ok(c) => c,
    Err(e) => return Err(e.into()),
};
```

### 4.6 Trait là gì?

`trait` = interface trong Java/TypeScript — định nghĩa một tập hành vi mà nhiều kiểu có thể implement.

```rust
// serde::Serialize và Deserialize là traits
// Thêm #[derive(Serialize, Deserialize)] để tự động implement
#[derive(Serialize, Deserialize)]
pub struct WordRecord { ... }
// Giờ WordRecord có thể convert sang/từ JSON tự động
```

### 4.7 Tóm tắt mapping khái niệm

| Rust | Python | JavaScript |
|---|---|---|
| `crate` | package | npm package |
| `mod` | module (file) | ES module |
| `struct` | class (data only) | object type |
| `impl` | class methods | class methods |
| `trait` | abstract class / Protocol | interface (TypeScript) |
| `pub` | (public by default) | `export` |
| `Result<T, E>` | try/except | Promise reject |
| `Option<T>` | `Optional` / `None` | `T \| null` |

---

## 5. Sơ đồ kiến trúc tổng thể

```
┌─────────────────────────────────────────────────────┐
│                   ui/index.html                     │
│         (HTML + CSS + JavaScript thuần)             │
│  invoke('command_name', { payload: {...} })         │
└──────────────────────┬──────────────────────────────┘
                       │  Tauri IPC (inter-process call)
                       ▼
┌─────────────────────────────────────────────────────┐
│            src-tauri/src/commands.rs                │
│         (~30 Tauri commands / API endpoints)        │
│  #[tauri::command]                                  │
│  pub fn import_vocabulary(payload) -> Result        │
└──────────────────────┬──────────────────────────────┘
                       │  Gọi thư viện Rust core
                       ▼
┌─────────────────────────────────────────────────────┐
│                  src/ (Core Library)                │
│                                                     │
│  service/ ──► testing/ ──► srs/                    │
│      │                                              │
│      ▼                                              │
│  import/ ──► parser/                               │
│      │                                              │
│      ▼                                              │
│  repository/ ──► db/ ──► SQLite file               │
│                                                     │
│  grammar/    bilingual/                             │
└─────────────────────────────────────────────────────┘
```

**Luồng dữ liệu điển hình (ví dụ: bấm "Next question"):**
1. User bấm nút trên UI
2. JS gọi `invoke('next_question', { payload: { mode: 'hybrid', srsEnabled: true } })`
3. Tauri route đến `commands::next_question()`
4. Command gọi `TestingEngine::generate_question_with_options(repo, mode, options)`
5. Engine dùng `repo.pick_next_word_scoped()` để lấy từ từ SQLite
6. Engine build `Question` struct và trả về
7. Command convert sang `QuestionDto` (JSON-serializable) rồi trả về JS
8. JS nhận JSON và cập nhật UI

---

## 6. Lớp Model — Dữ liệu trông như thế nào

**File:** `src/model.rs`

Đây là các kiểu dữ liệu cốt lõi của toàn bộ app.

### WordRecord — Một từ vựng

```rust
pub struct WordRecord {
    pub word: String,                  // "consistency"
    pub phonetic: Option<String>,      // "/kənˈsɪstənsi/" (có thể không có)
    pub definitions: Vec<Definition>,  // Danh sách định nghĩa
    pub examples: Vec<String>,         // Câu ví dụ
    pub synonyms: Vec<String>,         // Từ đồng nghĩa
    pub antonyms: Vec<String>,         // Từ trái nghĩa
    pub family_words: Vec<String>,     // Từ cùng họ: "consistent", "consistently"
    pub level: Option<String>,         // CEFR: A1/A2/B1/B2/C1/C2
    pub metadata: Metadata,            // Tags, review_count, created_at
}

pub struct Definition {
    pub pos: Option<String>,   // Part of speech: "noun", "verb",...
    pub meaning: String,       // "Tính nhất quán; sự kiên định"
}
```

**Lưu ý quan trọng: `word_key`**

```rust
impl WordRecord {
    pub fn word_key(&self) -> String {
        normalize_key(&self.word)  // trim + lowercase
    }
}

// "Consistency" → word_key = "consistency"
// "CONSISTENCY" → word_key = "consistency"
// Dùng làm unique key trong DB để tránh trùng lặp
```

### Collection và Topic — Tổ chức từ vựng

```
Collection "Daily Life Vocabulary"
    └── Topic "Food & Drink"
    └── Topic "Travel"
Collection "Business English"
    └── Topic "Meetings"
```

```rust
pub struct Collection {
    pub id: i64,
    pub name: String,           // "Daily Life Vocabulary"
    pub description: Option<String>,
    pub created_at: String,     // ISO 8601 timestamp
}

pub struct Topic {
    pub id: i64,
    pub collection_id: i64,     // Foreign key to Collection
    pub name: String,           // "Food & Drink"
    pub description: Option<String>,
}
```

---

## 7. Lớp Database — Lưu trữ dữ liệu

**Files:** `src/db/mod.rs`, `src/db/schema.rs`, `src/db/merge.rs`

### Khởi tạo database

```rust
// src/db/mod.rs
pub fn init_db<P: AsRef<Path>>(db_path: P) -> Result<Connection, DbError> {
    let conn = Connection::open(db_path)?;              // Open or create SQLite file
    conn.execute("PRAGMA foreign_keys = ON", [])?;      // Enable foreign key constraints
    schema::init_schema(&conn)?;                        // Create tables if not exist
    Ok(conn)
}
```

DB file nằm ở: `src-tauri/reemember.db` (tạo lúc app khởi chạy lần đầu).

### Schema — Cấu trúc bảng

App có **14 bảng** SQL. Dưới đây là các bảng quan trọng nhất:

#### Từ vựng (normalized — chuẩn hóa)

Thay vì lưu hết vào 1 bảng khổng lồ, từ vựng được tách ra nhiều bảng liên kết nhau:

```
words (id, word_key, word, phonetic, level, review_count, created_at)
  │
  ├── definitions (id, word_id, pos, meaning)      ← 1 từ có nhiều nghĩa
  ├── examples    (id, word_id, example)           ← 1 từ có nhiều ví dụ
  ├── word_tags   (word_id, tag_id)                ← Many-to-many với tags
  ├── word_relations (word_id, related_word, type) ← synonym/antonym/family
  ├── word_topics (word_id, topic_id)              ← Từ thuộc topic nào
  └── review_schedule (word_id, due_at)            ← SRS: hạn ôn tập tiếp theo
```

**Tại sao lại tách ra nhiều bảng?** Đây là "database normalization":
- Nếu để hết vào 1 bảng: `examples` sẽ là string JSON phức tạp, khó query
- Tách ra: có thể query "tất cả từ chưa có ví dụ nào" bằng SQL đơn giản

#### SRS (Spaced Repetition)

```sql
-- Khi nào ôn từ tiếp theo?
review_schedule (word_id INTEGER, due_at TEXT)

-- Lịch sử ôn tập từng lần
review_history  (id, word_id, reviewed_at, was_correct)
```

#### Ngữ pháp

```
grammar_groups (id, name, sort_order)       ← Nhóm: "Tenses", "Modals",...
grammar_docs   (id, title, category, level, content, group_id)
grammar_exercises (id, doc_id, exercise_type, data)
```

#### Song ngữ

```
bilingual_articles (id, title, book, level, paragraphs JSON, structures JSON)
```

### Migration — Nâng cấp schema

Khi thêm cột mới vào DB đã có dữ liệu cũ, cần "migration". Ví dụ trong code:

```rust
// Check if 'level' column exists (might not in old databases)
let has_level: i64 = conn.query_row(
    "SELECT COUNT(*) FROM pragma_table_info('words') WHERE name = 'level'",
    [], |row| row.get(0),
).unwrap_or(0);

if has_level == 0 {
    conn.execute("ALTER TABLE words ADD COLUMN level TEXT", [])?;
}
```

---

## 8. Lớp Repository — Truy vấn dữ liệu

**Files:** `src/repository/mod.rs`, `src/repository/query.rs`

Repository là "cổng duy nhất" để code khác tương tác với DB. Mọi SQL query đều ở đây.

```rust
pub struct WordRepository {
    conn: RefCell<Connection>,   // RefCell allows interior mutability
}

impl WordRepository {
    pub fn new(conn: Connection) -> Self { ... }

    // Lấy 1 từ theo key
    pub fn get_by_word_key(&self, key: &str) -> Result<Option<WordRecord>, DbError>

    // Lấy danh sách từ với filter
    pub fn query(&self, options: &QueryOptions) -> Result<Vec<WordRecord>, DbError>

    // Thêm hoặc cập nhật từ (upsert)
    pub fn upsert(&self, record: &WordRecord) -> Result<UpsertResult, DbError>

    // Xóa từ
    pub fn delete_by_word_key(&self, key: &str) -> Result<(), DbError>

    // Chọn từ tiếp theo để ôn (có tính SRS và topic filter)
    pub fn pick_next_word_scoped(
        &self,
        srs_enabled: bool,
        now: &str,
        topic_id: Option<i64>,
    ) -> Result<Option<WordRecord>, DbError>

    // Thống kê tổng hợp
    pub fn get_stats(&self) -> Result<Stats, DbError>

    // Import nhiều từ trong 1 transaction
    pub fn import_batch_atomic(
        &self,
        items: &[BatchImportItem],
    ) -> Result<BatchImportReport, DbError>
}
```

### QueryOptions — Bộ lọc linh hoạt

```rust
// src/repository/query.rs
pub struct QueryOptions {
    pub sort: Option<SortBy>,          // Sắp xếp theo gì?
    pub limit: Option<usize>,          // Giới hạn số lượng
    pub tag_filter: Option<String>,    // Lọc theo tag
    pub collection_id: Option<i64>,   // Lọc theo collection
    pub topic_id: Option<i64>,        // Lọc theo topic
}

pub enum SortBy {
    Word,         // A-Z
    ReviewCount,  // Từ ôn nhiều nhất
    DueDate,      // SRS due date
}
```

### Upsert — Insert hoặc Update

Khi import từ, nếu từ đã tồn tại thì update, chưa có thì insert. Đây gọi là "upsert":

```rust
pub struct UpsertResult {
    pub inserted: bool,           // true = từ mới, false = cập nhật từ cũ
    pub definitions_count: usize,
    pub examples_count: usize,
    pub tags_count: usize,
}
```

---

## 9. Lớp Parser — Đọc file JSON/MD

**Files:** `src/parser/json.rs`, `src/grammar/parser.rs`, `src/bilingual/parser.rs`

### JSON Parser cho từ vựng

Hỗ trợ 2 format:

**Format 1 — Array đơn giản:**
```json
[
  {
    "word": "consistency",
    "definitions": [{"pos": "noun", "meaning": "Tính nhất quán"}],
    "examples": ["Consistency is key."],
    "synonyms": ["steadiness"],
    "level": "B2"
  }
]
```

**Format 2 — Bundle (có collection/topic):**
```json
{
  "collection": "Daily Life",
  "topic": "General",
  "words": [ { ...từ vựng... } ]
}
```

```rust
// src/parser/json.rs
// Rust enum for "either format"
#[serde(untagged)]   // Try each variant in order
enum ImportPayload {
    Array(Vec<WordRecord>),   // Format 1
    Bundle(BundleImport),     // Format 2
}

pub fn parse_json_bundle(input: &str) -> Result<BundleImport, ParseError> {
    let payload: ImportPayload = serde_json::from_str(input)?;
    // Normalize both formats to BundleImport
    let bundle = match payload {
        ImportPayload::Array(words) => BundleImport { collection: None, topic: None, words },
        ImportPayload::Bundle(b) => b,
    };
    // Validate each word
    for (idx, record) in bundle.words.iter().enumerate() {
        record.validate().map_err(|e| ParseError::InvalidData(...))?;
    }
    Ok(bundle)
}
```

### Markdown Parser cho ngữ pháp

File `.md` trong `grammar_data/` có format:

```markdown
---
title: Present Simple
category: tenses
level: A1
---

# Present Simple

## Structure
Subject + verb (base form)...

## Exercises
```json
[{"type": "fill_blank", "question": "She ___ (go) to school.", "answer": "goes"}]
```
```

Parser đọc YAML frontmatter (phần giữa `---`) trước, sau đó phần còn lại là content.

---

## 10. Lớp Service — Logic nghiệp vụ

**File:** `src/service/mod.rs`

`VocabularyService` là một **facade** (mặt tiền) — gom tất cả các tính năng lại thành interface đơn giản hơn cho code ở tầng trên (Commands) sử dụng.

```rust
pub struct VocabularyService;  // Không có field, chỉ là namespace cho methods

impl VocabularyService {
    // Wrap ImportService
    pub fn import_json_scoped(repo, json_str, collection_name, topic_name) -> Result<ImportReport, DbError>

    // Wrap ExportService
    pub fn export_json(repo) -> Result<String, DbError>

    // Wrap TestingEngine
    pub fn next_question(repo, mode) -> Result<Option<Question>, DbError>
    pub fn submit_answer_with_srs(repo, question, answer, srs_enabled) -> Result<AnswerResult, DbError>
}
```

---

## 11. Lớp Testing & SRS — Engine học từ vựng

### TestingEngine — Sinh câu hỏi và chấm điểm

**File:** `src/testing/mod.rs`

```rust
pub enum TestMode {
    EnVi,    // Nhìn tiếng Anh → nhập tiếng Việt
    ViEn,    // Nhìn tiếng Việt → nhập tiếng Anh
    Hybrid,  // Random giữa 2 mode trên
}

pub struct TestingEngine;  // Stateless — không lưu state

impl TestingEngine {
    pub fn generate_question_with_options(
        repo: &WordRepository,
        mode: TestMode,
        options: TestingOptions,   // { srs_enabled, topic_id }
    ) -> Result<Option<Question>, DbError> {
        // 1. Pick a word from DB (SRS-aware if enabled)
        let record = repo.pick_next_word_scoped(...)?;
        // 2. Build question based on direction
        Ok(Some(Self::build_question(direction, record)))
    }
}
```

**EN→VI question:**
- Prompt: `"Give the Vietnamese meaning of 'consistency'."`
- Expected answers: `["Tính nhất quán", "sự kiên định"]` (split bởi dấu `;`)
- Examples: từ được mask: `"____ is the key to success."`

**VI→EN question:**
- Prompt: `"Which English word matches: 'Tính nhất quán' ?"`
- Expected answer: `["consistency"]`

**Chấm điểm (normalize trước khi so sánh):**
```rust
pub fn normalize_answer_text(value: &str) -> String {
    value
        .split_whitespace()   // Tách thành các từ (bỏ khoảng trắng thừa)
        .collect::<Vec<_>>()
        .join(" ")            // Ghép lại bằng 1 dấu cách
        .to_lowercase()       // Thành chữ thường
}
// "  Tính  Nhất  Quán " → "tính nhất quán"
```

### SRS — Spaced Repetition System

**File:** `src/srs/mod.rs`

Hệ thống tính toán khi nào nên ôn lại một từ:

```rust
pub fn compute_next_due(now: DateTime<Utc>, was_correct: bool, review_count: u32) -> String {
    let interval = if was_correct {
        match review_count {
            0..=1 => Duration::days(1),   // Ôn lại sau 1 ngày
            2..=3 => Duration::days(3),   // Sau 3 ngày
            4..=6 => Duration::days(7),   // Sau 1 tuần
            _     => Duration::days(14),  // Sau 2 tuần
        }
    } else {
        Duration::minutes(10)  // Trả lời sai → ôn lại sau 10 phút
    };
    (now + interval).to_rfc3339()  // Trả về timestamp ISO 8601
}
```

---

## 12. Module Grammar & Bilingual

### Grammar Module

**Files:** `src/grammar/mod.rs`, `src/grammar/parser.rs`, `src/grammar/repository.rs`

```rust
// Grammar document in DB
pub struct GrammarDoc {
    pub id: i64,
    pub title: String,         // "Present Simple"
    pub category: Option<String>, // "tenses"
    pub level: Option<String>,    // "A1"
    pub content: String,          // Markdown content
    pub examples: Vec<String>,
    pub exercise_count: usize,
    pub group_id: Option<i64>,    // Thuộc group nào
    pub created_at: String,
}

pub struct GrammarRepository {
    conn: Connection,
}

impl GrammarRepository {
    pub fn upsert_doc(&self, doc: &GrammarDocInput, group_id: Option<i64>) -> Result<UpsertResult, ...>
    pub fn list_docs(&self) -> Result<Vec<GrammarDoc>, ...>
    pub fn get_doc_with_exercises(&self, id: i64) -> Result<Option<GrammarDocDetail>, ...>
    pub fn find_or_create_group(&self, name: &str) -> Result<i64, ...>
}
```

### Bilingual Module

**Files:** `src/bilingual/mod.rs`, `src/bilingual/parser.rs`, `src/bilingual/repository.rs`

```rust
pub struct BilingualArticle {
    pub id: i64,
    pub title: String,
    pub book: String,           // Tên sách/nguồn
    pub level: Option<String>,  // B1, B2,...
    pub paragraphs: Vec<Vec<BilingualSegment>>,  // Đoạn văn → câu → song ngữ
    pub structures: Vec<GrammarStructure>,       // Cấu trúc ngữ pháp trong bài
    pub created_at: String,
}

pub struct BilingualSegment {
    pub en: String,   // "The boat was in trouble."
    pub vi: String,   // "Con thuyền đang gặp nguy hiểm."
}
```

---

## 13. Lớp Commands — Cầu nối Backend↔Frontend

**Files:** `src-tauri/src/commands.rs`, `src-tauri/src/main.rs`

Đây là **API layer** của app — giống như REST API endpoints nhưng thay vì HTTP thì dùng Tauri IPC.

### Cách một command hoạt động

```rust
// src-tauri/src/commands.rs

// DTO (Data Transfer Object) — kiểu dữ liệu đi qua IPC phải serializable
#[derive(Deserialize)]   // Nhận từ JS
#[serde(rename_all = "camelCase")]  // JS dùng camelCase, Rust dùng snake_case
pub struct NextQuestionRequest {
    pub mode: String,         // "hybrid", "en-vi", "vi-en"
    pub srs_enabled: bool,
    pub topic_id: Option<i64>,
}

#[derive(Serialize)]    // Gửi về JS
#[serde(rename_all = "camelCase")]
pub struct QuestionDto {
    pub word_key: String,
    pub direction: String,
    pub prompt: String,
    pub word: Option<String>,
    pub expected_answers: Vec<String>,
    // ...
}

#[tauri::command]   // Macro đánh dấu đây là Tauri command
pub fn next_question(payload: NextQuestionRequest) -> Result<Option<QuestionDto>, String> {
    let mode = parse_mode(&payload.mode)?;

    // Every command opens its own DB connection (stateless design)
    let conn = init_db(DB_PATH).map_err(|e| e.to_string())?;
    let repo = WordRepository::new(conn);

    let options = TestingOptions {
        srs_enabled: payload.srs_enabled,
        topic_id: payload.topic_id,
    };

    // Call core library
    let maybe_question = TestingEngine::generate_question_with_options(&repo, mode, options)
        .map_err(|e| e.to_string())?;

    // Convert internal type → DTO for JSON serialization
    Ok(maybe_question.map(from_question))
}
```

### Đăng ký commands trong main.rs

```rust
// src-tauri/src/main.rs
fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            next_question,        // Tất cả commands phải đăng ký ở đây
            submit_answer,
            import_vocabulary,
            sync_local_data,
            // ... 27 commands khác
        ])
        .setup(|app| {
            // Tạo cửa sổ app
            WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("Reemember")
                .inner_size(1000.0, 750.0)
                .build()?;

            // Thông báo nếu có từ cần ôn
            if let Ok(conn) = init_db(DB_PATH) {
                let repo = WordRepository::new(conn);
                if let Ok(stats) = repo.get_stats() {
                    if stats.due_count > 0 {
                        app.notification().builder()
                            .body(format!("Bạn có {} từ đang chờ ôn tập!", stats.due_count))
                            .show();
                    }
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### Danh sách tất cả commands

| Command | Chức năng |
|---|---|
| `next_question` | Lấy câu hỏi tiếp theo |
| `submit_answer` | Nộp câu trả lời, cập nhật SRS |
| `list_words` | Danh sách từ (có filter) |
| `delete_word` | Xóa 1 từ |
| `import_vocabulary` | Import 1 file JSON |
| `import_vocabulary_batch` | Import nhiều file (atomic transaction) |
| `sync_manifest_url` | Sync từ URL manifest |
| `sync_local_data` | Sync từ thư mục data local |
| `save_export` | Export ra file JSON |
| `list_collections` | Danh sách collections |
| `create/update/delete_collection` | CRUD collection |
| `list_topics` | Danh sách topics |
| `create/update/delete_topic` | CRUD topic |
| `assign_word_to_topic` | Gán từ vào topic |
| `list_grammar_docs` | Danh sách bài ngữ pháp |
| `get_grammar_doc` | Chi tiết 1 bài ngữ pháp |
| `import_grammar` | Import bài ngữ pháp |
| `list/create/update/delete_grammar_group` | CRUD nhóm ngữ pháp |
| `move_grammar_doc` | Chuyển bài vào nhóm khác |
| `delete_grammar_doc` | Xóa bài ngữ pháp |
| `list_bilingual_articles` | Danh sách bài song ngữ |
| `get_bilingual_article` | Chi tiết 1 bài song ngữ |
| `import_bilingual` | Import bài song ngữ |
| `get_stats` | Thống kê học tập |
| `set_app_theme` | Đổi theme |

---

## 14. Frontend — Giao diện người dùng

**File:** `ui/index.html` (1 file duy nhất, ~3000 dòng HTML+CSS+JS)

### Cách gọi backend từ JS

```javascript
// Wrapper function (defined once, used everywhere)
function invoke(cmd, args) {
    return window.__TAURI_INTERNALS__.invoke(cmd, args);
}

// Usage examples
const question = await invoke('next_question', {
    payload: { mode: 'hybrid', srsEnabled: true, topicId: null }
});

const result = await invoke('submit_answer', {
    payload: { answer: 'tính nhất quán', srsEnabled: true, question }
});
```

Lưu ý: Tauri yêu cầu args phải wrapped trong `{ payload: ... }` khi command nhận struct có tên `payload`.

### Cấu trúc UI

```
app (flex column)
├── nav.top-nav          ← Navigation bar (desktop)
│   ├── Study
│   ├── Library
│   ├── Import/Export
│   ├── Bilingual Books
│   ├── Grammar
│   └── Stats
│
├── main.app-main
│   ├── #studyPage       ← Tab Study
│   │   ├── .card (Session controls)
│   │   │   ├── Mode select (EN→VI / VI→EN / Hybrid)
│   │   │   ├── Scope select (All / by topic)
│   │   │   ├── SRS toggle
│   │   │   ├── [Next question] [Reset stats]
│   │   │   └── .study-controls (CSS Grid layout)
│   │   ├── .stats-bar   ← Asked / Correct / Accuracy
│   │   └── .question-card
│   │       ├── Prompt text
│   │       ├── Meta (phonetic, level)
│   │       ├── Examples
│   │       ├── Answer input
│   │       ├── Result box (correct/wrong)
│   │       └── Related panel (synonyms, antonyms,...)
│   │
│   ├── #libraryPage     ← Tab Library
│   ├── #importExportPage ← Tab Import/Export
│   ├── #bilingualPage   ← Tab Bilingual Books
│   ├── #grammarPage     ← Tab Grammar
│   └── #statsPage       ← Tab Stats
│
└── nav.bot-nav          ← Navigation (mobile)
```

### Themes

App hỗ trợ 9 themes thông qua CSS custom properties:
`system`, `light`, `dark`, `sepia`, `ocean`, `rose`, `forest`, `midnight`, `contrast`

---

## 15. Dữ liệu mẫu (Data files)

### vocabulary_data/

```
vocabulary_data/
└── daily_life/
    ├── greetings_expressions.json   ← ~30 từ A1
    ├── food_and_drink.json          ← ~30 từ A1-A2
    ├── shopping.json
    ├── health_and_body.json
    └── ... (20 file tổng cộng)
```

Format mỗi file:
```json
{
  "collection": "Daily Life Vocabulary",
  "topic": "Food & Drink",
  "words": [
    {
      "word": "appetite",
      "phonetic": "/ˈæpɪtaɪt/",
      "level": "B1",
      "definitions": [{"pos": "noun", "meaning": "Cảm giác thèm ăn; sự thèm muốn"}],
      "examples": ["I have no appetite today."],
      "synonyms": ["hunger", "desire"],
      "antonyms": ["aversion"],
      "family_words": ["appetizer", "appetizing"]
    }
  ]
}
```

### grammar_data/

40+ file Markdown, tổ chức theo chủ đề ngữ pháp (tenses, conditionals, modals,...). Mỗi file là 1 bài học hoàn chỉnh với lý thuyết + 12 bài tập.

### bilingual_data/

Các đoạn văn từ sách "The Open Boat" (Stephen Crane) và các bài báo mẫu, đã được dịch song ngữ Anh-Việt.

---

## 16. Cách thêm tính năng mới

### Ví dụ: Thêm tính năng "đánh dấu từ yêu thích"

**Bước 1: Thêm cột vào DB** (`src/db/schema.rs`)

```rust
// In init_schema(), add migration:
let has_favorite: i64 = conn.query_row(
    "SELECT COUNT(*) FROM pragma_table_info('words') WHERE name = 'is_favorite'",
    [], |row| row.get(0),
).unwrap_or(0);
if has_favorite == 0 {
    conn.execute("ALTER TABLE words ADD COLUMN is_favorite INTEGER NOT NULL DEFAULT 0", [])?;
}
```

**Bước 2: Cập nhật Model** (`src/model.rs`)

```rust
pub struct WordRecord {
    // ...existing fields...
    #[serde(default)]
    pub is_favorite: bool,   // New field
}
```

**Bước 3: Thêm method Repository** (`src/repository/mod.rs`)

```rust
impl WordRepository {
    pub fn toggle_favorite(&self, word_key: &str) -> Result<bool, DbError> {
        // Toggle is_favorite, return new value
        let conn = self.conn.borrow();
        conn.execute(
            "UPDATE words SET is_favorite = 1 - is_favorite WHERE word_key = ?1",
            [word_key],
        )?;
        let new_val: i64 = conn.query_row(
            "SELECT is_favorite FROM words WHERE word_key = ?1",
            [word_key], |r| r.get(0),
        )?;
        Ok(new_val == 1)
    }
}
```

**Bước 4: Thêm Tauri command** (`src-tauri/src/commands.rs`)

```rust
#[tauri::command]
pub fn toggle_favorite(word_key: String) -> Result<bool, String> {
    let conn = init_db(DB_PATH).map_err(|e| e.to_string())?;
    let repo = WordRepository::new(conn);
    repo.toggle_favorite(&word_key).map_err(|e| e.to_string())
}
```

**Bước 5: Đăng ký command** (`src-tauri/src/main.rs`)

```rust
.invoke_handler(tauri::generate_handler![
    // ...existing commands...
    toggle_favorite,   // Add here
])
```

**Bước 6: Gọi từ JS** (`ui/index.html`)

```javascript
async function toggleFavorite(wordKey) {
    const isFavorite = await invoke('toggle_favorite', { wordKey });
    updateFavoriteIcon(wordKey, isFavorite);
}
```

---

## Phụ lục: Luồng Import từ vựng

Đây là một luồng phức tạp, giải thích để hiểu cách các lớp phối hợp:

```
User drag-drop file.json vào UI
    ↓
JS: FileReader.readAsText(file)
    ↓
JS: invoke('import_vocabulary', { payload: { content, collectionName, topicName } })
    ↓
commands::import_vocabulary(payload)
    ↓
VocabularyService::import_json_scoped(repo, content, collection, topic)
    ↓
ImportService::import_from_json_string_scoped()
    ↓
parser::parse_json_bundle(content)
    ├── Detect format: Array hay Bundle?
    ├── Deserialize JSON → Vec<WordRecord>
    └── Validate mỗi word (word không rỗng, có definition)
    ↓
Resolve collection & topic (create nếu chưa có)
    ↓
repo.upsert(word) cho từng từ
    ├── Nếu word_key chưa có → INSERT
    └── Nếu word_key đã có → UPDATE (merge definitions, examples,...)
    ↓
ImportReport { inserted_count, updated_count, skipped_count }
    ↓
ImportReportDto → JSON → JS
    ↓
UI hiển thị kết quả: "Inserted 23, Updated 5, Skipped 0"
```
