# Reemember

A personal vocabulary learning desktop application built with Rust and Tauri. Designed for English learners who want full control over their word data — easy import/export, simple backup/restore, and an integrated spaced repetition system (SRS).

## Features

- Import vocabulary from JSON or CSV files
- Export your full library to JSON or CSV for backup
- Quiz yourself in English→Vietnamese, Vietnamese→English, or Hybrid mode
- Spaced repetition scheduling to surface due words first
- Organize words into Collections and Topics
- Merge-safe upsert: re-importing never loses existing data

## Installation

### Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) (for Tauri frontend tooling)
- [Tauri CLI](https://v2.tauri.app/start/prerequisites/)

```bash
cargo install tauri-cli
```

### Build & Run

```bash
git clone <repo-url>
cd reemember

# Run in development mode
cargo tauri dev

# Build for production
cargo tauri build
```

The application window (1000×750) opens automatically.

### Library-only (no UI)

```bash
# Run all tests
cargo test

# Use as a Rust library
cargo build
```

---

## User Manual

### Study Mode

1. Click **Study** in the navigation bar.
2. Choose a quiz mode:
   - **EN→VI** — see the English word, type the Vietnamese meaning
   - **VI→EN** — see the Vietnamese meaning, type the English word
   - **Hybrid** — alternates between both directions randomly
3. Toggle **SRS** on to prioritize words that are due for review.
4. Type your answer and press **Submit** (or Enter).
5. A green card means correct; red means incorrect — the correct answer is shown.
6. Press **Next** to continue.

Answer grading is case-insensitive and ignores extra whitespace.

### Library

1. Click **Library** in the navigation bar to browse all saved words.
2. Use the sidebar to filter by Collection or Topic.
3. Click a word row to see its full details.
4. Use the **Delete** button to remove a word permanently.

### Import

1. Click **Import** in the navigation bar.
2. Click **Choose File** and select a `.json` or `.csv` file.
3. The import report shows:
   - Words **inserted** (new)
   - Words **updated** (merged with existing data)
   - Words **skipped** (no usable data)
   - Per-row errors for CSV files
4. Re-importing a file is always safe — existing data is merged, never overwritten blindly.

### Export

1. From the **Library** page, click **Export**.
2. Choose **JSON** (recommended, full fidelity) or **CSV** (spreadsheet-compatible).
3. A save dialog opens — choose a destination and filename.
4. The exported file can be re-imported at any time without data loss.

### Collections & Topics

1. Click **Collections** in the navigation bar.
2. Create a **Collection** as a top-level grouping (e.g., "IELTS Vocabulary").
3. Inside a collection, create **Topics** for finer categories (e.g., "Environment", "Technology").
4. Assign words to topics from the Library view.

---

## Data Formats

### JSON Format

```json
[
  {
    "word": "ephemeral",
    "phonetic": "/ɪˈfem.ər.əl/",
    "definitions": [
      { "pos": "adjective", "meaning_vi": "phù du, ngắn ngủi" }
    ],
    "examples": ["Fame is ephemeral."],
    "synonyms": ["transient", "fleeting"],
    "antonyms": ["permanent"],
    "family_words": ["ephemerality"],
    "metadata": {
      "tags": ["advanced", "ielts"],
      "created_at": "2024-01-15T10:00:00Z",
      "review_count": 3
    }
  }
]
```

Only `word` and at least one `definitions` entry are required. All other fields are optional.

### CSV Format

**Minimal (2 columns):**
```csv
word,meaning_vi
ephemeral,phù du
```

**Extended:**
```csv
word,phonetic,meaning_vi,pos,examples,synonyms,antonyms,family_words,tags,review_count
ephemeral,/ɪˈfem.ər.əl/,phù du,adjective,Fame is ephemeral.,transient|fleeting,permanent,ephemerality,advanced|ielts,3
```

Multi-value fields (examples, synonyms, antonyms, family_words, tags) use `|` as the delimiter.

---

## Merge Policy

When a word already exists in the database, Reemember merges the incoming data rather than replacing it:

| Field | Rule |
|-------|------|
| `phonetic` | Use new value if non-empty; otherwise keep existing |
| `definitions` | Combine both lists; deduplicate by (pos, meaning) |
| `examples` | Combine both lists; deduplicate by exact string |
| `tags` | Combine, deduplicate, sort alphabetically |
| `created_at` | Keep the earliest timestamp |
| `review_count` | Keep the highest count |
| `synonyms` / `antonyms` / `family_words` | Combine and deduplicate |

---

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Core library | Rust (stable) |
| Database | SQLite via `rusqlite` (bundled) |
| Desktop shell | Tauri 2 |
| UI | HTML / CSS / JavaScript (single-page) |
| Serialization | `serde` + `serde_json` |
| CSV parsing | `csv` crate |
| Date/time | `chrono` |
| Error handling | `thiserror` |

---

## License

MIT
