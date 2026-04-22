# Reemember

A personal vocabulary learning desktop app built with Rust + Tauri.

## What it does

- Study vocabulary in `EN -> VI`, `VI -> EN`, or `Hybrid` mode
- Use SRS due-first scheduling (optional toggle)
- Organize words with Collections and Topics
- Import vocabulary from JSON (array or bundle format)
- Export full vocabulary to JSON backup
- Merge re-imported data safely (upsert + dedupe rules)

## Run the app

### Prerequisites

- Rust toolchain (stable)
- Platform prerequisites for Tauri desktop builds

### Development run

```bash
cd src-tauri
cargo run
```

### Run tests

```bash
cargo test
```

## Import / Export format

### Import JSON

Supported inputs:

1. **Flat array**

```json
[
  {
    "word": "ephemeral",
    "definitions": [
      { "pos": "adjective", "meaning": "phu du, ngan ngui" }
    ],
    "examples": ["Fame is ephemeral."],
    "synonyms": ["transient", "fleeting"],
    "antonyms": ["permanent"],
    "family_words": ["ephemerality"],
    "metadata": {
      "tags": ["advanced", "ielts"],
      "created_at": "2026-01-15T10:00:00Z",
      "review_count": 0
    }
  }
]
```

2. **Bundle object**

```json
{
  "collection": "IELTS",
  "topic": "Environment",
  "words": [
    {
      "word": "ephemeral",
      "definitions": [{ "meaning": "phu du" }]
    }
  ]
}
```

Required fields per word:

- `word` (non-empty)
- `definitions` with at least one non-empty `meaning`

### Export JSON

Export saves the entire vocabulary library as pretty-printed JSON and is round-trip compatible with import.

## Architecture snapshot

- Core library: `src/`
- Desktop app: `src-tauri/`
- UI: `ui/index.html`
- Database: SQLite via `rusqlite` (bundled)
- Desktop entrypoint: `src-tauri/src/main.rs`

## License

MIT
