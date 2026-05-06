/// Database schema initialization and management.
/// Defines all tables and creates them on first connection if needed.

use rusqlite::Connection;
use crate::db::DbError;

/// Initialize the database schema by creating all required tables.
/// This function is idempotent—calling it multiple times is safe.
///
/// # Schema Overview
/// - `words`: Main table storing core word information (word, phonetic, timestamps, review count)
/// - `definitions`: Normalized table for word meanings (supports multiple definitions per word)
/// - `examples`: Examples of word usage (supports multiple examples per word)
/// - `tags`: Dictionary of available tags
/// - `word_tags`: Junction table linking words to tags (many-to-many relationship)
///
/// # Parameters
/// - `conn`: Active SQLite connection
///
/// # Returns
/// - `Ok(())`: Schema initialized successfully
/// - `Err(DbError)`: Table creation failed
pub fn init_schema(conn: &Connection) -> Result<(), DbError> {
    // Create words table with core vocabulary entry data
    conn.execute(
        "CREATE TABLE IF NOT EXISTS words (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            word_key TEXT NOT NULL UNIQUE,
            word TEXT NOT NULL,
            phonetic TEXT,
            created_at TEXT,
            review_count INTEGER NOT NULL DEFAULT 0,
            updated_at TEXT NOT NULL
        )",
        [],
    )?;

    // Create definitions table (normalized: one row per definition)
    // This allows efficient filtering and grouping by part-of-speech
    conn.execute(
        "CREATE TABLE IF NOT EXISTS definitions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            word_id INTEGER NOT NULL,
            pos TEXT,
            meaning TEXT NOT NULL,
            FOREIGN KEY(word_id) REFERENCES words(id) ON DELETE CASCADE
        )",
        [],
    )?;

    // Create examples table (one row per example)
    // Denormalized for simplicity; could normalize further if needed for Phase 3
    conn.execute(
        "CREATE TABLE IF NOT EXISTS examples (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            word_id INTEGER NOT NULL,
            example TEXT NOT NULL,
            FOREIGN KEY(word_id) REFERENCES words(id) ON DELETE CASCADE
        )",
        [],
    )?;

    // Create tags table (dictionary of tag names)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS tags (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE
        )",
        [],
    )?;

    // Create word_tags junction table (many-to-many: words to tags)
    // Allows efficient grouping queries and supports filtering by tag
    conn.execute(
        "CREATE TABLE IF NOT EXISTS word_tags (
            word_id INTEGER NOT NULL,
            tag_id INTEGER NOT NULL,
            PRIMARY KEY (word_id, tag_id),
            FOREIGN KEY(word_id) REFERENCES words(id) ON DELETE CASCADE,
            FOREIGN KEY(tag_id) REFERENCES tags(id) ON DELETE CASCADE
        )",
        [],
    )?;

    // Create review_schedule table (one due_at timestamp per word)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS review_schedule (
            word_id INTEGER PRIMARY KEY,
            due_at TEXT NOT NULL,
            FOREIGN KEY(word_id) REFERENCES words(id) ON DELETE CASCADE
        )",
        [],
    )?;

    // Create review_history table (append-only review events)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS review_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            word_id INTEGER NOT NULL,
            reviewed_at TEXT NOT NULL,
            was_correct INTEGER NOT NULL,
            FOREIGN KEY(word_id) REFERENCES words(id) ON DELETE CASCADE
        )",
        [],
    )?;

    // Create indexes for common query patterns
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_words_word_key ON words(word_key)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_definitions_word_id ON definitions(word_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_examples_word_id ON examples(word_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_word_tags_word_id ON word_tags(word_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_word_tags_tag_id ON word_tags(tag_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_review_schedule_due_at ON review_schedule(due_at)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_review_history_word_id ON review_history(word_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_review_history_reviewed_at ON review_history(reviewed_at)",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS word_relations (
            word_id INTEGER NOT NULL,
            related_word TEXT NOT NULL,
            relation_type TEXT NOT NULL CHECK(relation_type IN ('synonym','antonym','family')),
            PRIMARY KEY (word_id, related_word, relation_type),
            FOREIGN KEY(word_id) REFERENCES words(id) ON DELETE CASCADE
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS collections (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            description TEXT,
            created_at TEXT NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS topics (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            collection_id INTEGER NOT NULL,
            name TEXT NOT NULL,
            description TEXT,
            UNIQUE(collection_id, name),
            FOREIGN KEY(collection_id) REFERENCES collections(id) ON DELETE CASCADE
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS word_topics (
            word_id INTEGER NOT NULL,
            topic_id INTEGER NOT NULL,
            PRIMARY KEY (word_id, topic_id),
            FOREIGN KEY(word_id) REFERENCES words(id) ON DELETE CASCADE,
            FOREIGN KEY(topic_id) REFERENCES topics(id) ON DELETE CASCADE
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_word_relations_word_id ON word_relations(word_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_topics_collection_id ON topics(collection_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_word_topics_topic_id ON word_topics(topic_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_word_topics_word_id ON word_topics(word_id)",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS grammar_groups (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            description TEXT,
            sort_order INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS grammar_docs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            category TEXT,
            level TEXT,
            content TEXT NOT NULL DEFAULT '',
            examples TEXT NOT NULL DEFAULT '[]',
            group_id INTEGER,
            created_at TEXT NOT NULL,
            FOREIGN KEY(group_id) REFERENCES grammar_groups(id) ON DELETE SET NULL
        )",
        [],
    )?;

    // Migration: add group_id to existing grammar_docs tables that predate it.
    let has_group_id: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('grammar_docs') WHERE name = 'group_id'",
        [],
        |row| row.get(0),
    ).unwrap_or(0);
    if has_group_id == 0 {
        conn.execute("ALTER TABLE grammar_docs ADD COLUMN group_id INTEGER", [])?;
    }

    conn.execute(
        "CREATE TABLE IF NOT EXISTS grammar_exercises (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            doc_id INTEGER NOT NULL,
            order_index INTEGER NOT NULL DEFAULT 0,
            exercise_type TEXT NOT NULL,
            data TEXT NOT NULL,
            FOREIGN KEY(doc_id) REFERENCES grammar_docs(id) ON DELETE CASCADE
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_grammar_exercises_doc_id ON grammar_exercises(doc_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_grammar_docs_group_id ON grammar_docs(group_id)",
        [],
    )?;

    Ok(())
}

