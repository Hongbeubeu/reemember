pub mod query;

use crate::db::DbError;
use crate::db::merge;
use crate::model::{Collection, Definition, Metadata, Topic, WordRecord};
use chrono::Utc;
use rusqlite::{Connection, OptionalExtension};
use std::cell::RefCell;

pub use query::{QueryOptions, SortBy};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpsertResult {
    pub inserted: bool,
    pub definitions_count: usize,
    pub examples_count: usize,
    pub tags_count: usize,
}

pub struct WordRepository {
    conn: RefCell<Connection>,
}

impl WordRepository {
    pub fn new(conn: Connection) -> Self {
        WordRepository { conn: RefCell::new(conn) }
    }

    // ===== Word CRUD =====

    pub fn upsert(&self, record: &WordRecord) -> Result<UpsertResult, DbError> {
        record.validate().map_err(|e| DbError::Validation(e.to_string()))?;

        let word_key = record.word_key();
        let now = Utc::now().to_rfc3339();
        let existing = self.get_by_word_key(&word_key)?;

        let (inserted, merged) = if let Some(old) = existing {
            (false, self.merge_records(&old, record)?)
        } else {
            (true, record.clone())
        };

        let mut conn = self.conn.borrow_mut();
        let tx = conn.transaction().map_err(DbError::Sqlite)?;

        if inserted {
            let word_id = Self::insert_word_record_tx(&tx, &merged, &now)?;
            Self::initialize_schedule_tx(&tx, word_id, &now)?;
            Self::insert_definitions_tx(&tx, word_id, &merged.definitions)?;
            Self::insert_examples_tx(&tx, word_id, &merged.examples)?;
            Self::insert_tags_tx(&tx, word_id, &merged.metadata.tags)?;
            Self::insert_relations_tx(&tx, word_id, &merged.synonyms, "synonym")?;
            Self::insert_relations_tx(&tx, word_id, &merged.antonyms, "antonym")?;
            Self::insert_relations_tx(&tx, word_id, &merged.family_words, "family")?;
        } else {
            Self::update_word_record_tx(&tx, &word_key, &merged, &now)?;
            Self::replace_definitions_tx(&tx, &word_key, &merged.definitions)?;
            Self::replace_examples_tx(&tx, &word_key, &merged.examples)?;
            Self::replace_tags_tx(&tx, &word_key, &merged.metadata.tags)?;
            Self::replace_relations_tx(&tx, &word_key, &merged.synonyms, "synonym")?;
            Self::replace_relations_tx(&tx, &word_key, &merged.antonyms, "antonym")?;
            Self::replace_relations_tx(&tx, &word_key, &merged.family_words, "family")?;
        }

        tx.commit().map_err(DbError::Sqlite)?;

        Ok(UpsertResult {
            inserted,
            definitions_count: merged.definitions.len(),
            examples_count: merged.examples.len(),
            tags_count: merged.metadata.tags.len(),
        })
    }

    pub fn get_by_word_key(&self, word_key: &str) -> Result<Option<WordRecord>, DbError> {
        let conn = self.conn.borrow();
        let mut stmt = conn
            .prepare("SELECT id, word, phonetic, created_at, review_count FROM words WHERE word_key = ?")
            .map_err(DbError::Sqlite)?;

        let mut rows = stmt.query([word_key]).map_err(DbError::Sqlite)?;
        let row = match rows.next().map_err(DbError::Sqlite)? {
            Some(r) => r,
            None => return Ok(None),
        };

        let id: i64 = row.get(0).map_err(DbError::Sqlite)?;
        let word: String = row.get(1).map_err(DbError::Sqlite)?;
        let phonetic: Option<String> = row.get(2).map_err(DbError::Sqlite)?;
        let created_at: Option<String> = row.get(3).map_err(DbError::Sqlite)?;
        let review_count: u32 = row.get(4).map_err(DbError::Sqlite)?;
        drop(rows);
        drop(stmt);
        drop(conn);

        let definitions = self.load_definitions(id)?;
        let examples = self.load_examples(id)?;
        let tags = self.load_tags(id)?;
        let synonyms = self.load_relations(id, "synonym")?;
        let antonyms = self.load_relations(id, "antonym")?;
        let family_words = self.load_relations(id, "family")?;

        Ok(Some(WordRecord {
            word, phonetic, definitions, examples, synonyms, antonyms, family_words,
            metadata: Metadata { tags, created_at, review_count },
        }))
    }

    pub fn query(&self, options: &QueryOptions) -> Result<Vec<WordRecord>, DbError> {
        let mut query = "SELECT DISTINCT words.id, words.word, words.phonetic, words.created_at, words.review_count FROM words".to_string();

        let mut joins = vec![];
        let mut conditions = vec![];

        if options.tag_filter.is_some() {
            joins.push("JOIN word_tags ON words.id = word_tags.word_id JOIN tags ON word_tags.tag_id = tags.id");
        }
        if options.topic_id.is_some() || options.collection_id.is_some() {
            joins.push("JOIN word_topics ON words.id = word_topics.word_id JOIN topics ON word_topics.topic_id = topics.id");
        }

        for j in &joins {
            query.push(' ');
            query.push_str(j);
        }

        if let Some(ref tag) = options.tag_filter {
            conditions.push(format!("tags.name = '{}'", tag.replace("'", "''")));
        }
        if let Some(tid) = options.topic_id {
            conditions.push(format!("word_topics.topic_id = {}", tid));
        }
        if let Some(cid) = options.collection_id {
            conditions.push(format!("topics.collection_id = {}", cid));
        }

        if !conditions.is_empty() {
            query.push_str(" WHERE ");
            query.push_str(&conditions.join(" AND "));
        }

        if let Some(ref sort) = options.sort {
            query.push_str(&format!(" ORDER BY {}", sort.to_sql()));
        }
        if let Some(limit) = options.limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }

        let conn = self.conn.borrow();
        let mut stmt = conn.prepare(&query).map_err(DbError::Sqlite)?;
        let rows_result = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, u32>(4)?,
            ))
        }).map_err(DbError::Sqlite)?;

        let mut ids_and_data = vec![];
        for r in rows_result {
            ids_and_data.push(r.map_err(DbError::Sqlite)?);
        }
        drop(stmt);
        drop(conn);

        let mut records = Vec::new();
        for (id, word, phonetic, created_at, review_count) in ids_and_data {
            let definitions = self.load_definitions(id)?;
            let examples = self.load_examples(id)?;
            let tags = self.load_tags(id)?;
            let synonyms = self.load_relations(id, "synonym")?;
            let antonyms = self.load_relations(id, "antonym")?;
            let family_words = self.load_relations(id, "family")?;
            records.push(WordRecord {
                word, phonetic, definitions, examples, synonyms, antonyms, family_words,
                metadata: Metadata { tags, created_at, review_count },
            });
        }

        Ok(records)
    }

    pub fn pick_random_word(&self) -> Result<Option<WordRecord>, DbError> {
        self.pick_random_word_scoped(None)
    }

    pub fn pick_random_word_scoped(&self, topic_id: Option<i64>) -> Result<Option<WordRecord>, DbError> {
        let sql = if let Some(tid) = topic_id {
            format!(
                "SELECT words.id, words.word, words.phonetic, words.created_at, words.review_count \
                 FROM words JOIN word_topics ON words.id = word_topics.word_id \
                 WHERE word_topics.topic_id = {} ORDER BY RANDOM() LIMIT 1",
                tid
            )
        } else {
            "SELECT id, word, phonetic, created_at, review_count FROM words ORDER BY RANDOM() LIMIT 1".to_string()
        };

        self.load_one_word_from_sql(&sql)
    }

    pub fn increment_review_count_by_word_key(&self, word_key: &str) -> Result<(), DbError> {
        let conn = self.conn.borrow();
        conn.execute(
            "UPDATE words SET review_count = review_count + 1 WHERE word_key = ?",
            [word_key],
        ).map_err(DbError::Sqlite)?;
        Ok(())
    }

    pub fn pick_due_word(&self, now_utc: &str) -> Result<Option<WordRecord>, DbError> {
        self.pick_due_word_scoped(now_utc, None)
    }

    pub fn pick_due_word_scoped(&self, now_utc: &str, topic_id: Option<i64>) -> Result<Option<WordRecord>, DbError> {
        let sql = if let Some(tid) = topic_id {
            format!(
                "SELECT words.id, words.word, words.phonetic, words.created_at, words.review_count \
                 FROM words \
                 JOIN review_schedule ON words.id = review_schedule.word_id \
                 JOIN word_topics ON words.id = word_topics.word_id \
                 WHERE review_schedule.due_at <= '{}' AND word_topics.topic_id = {} \
                 ORDER BY review_schedule.due_at ASC LIMIT 1",
                now_utc.replace("'", "''"), tid
            )
        } else {
            format!(
                "SELECT words.id, words.word, words.phonetic, words.created_at, words.review_count \
                 FROM words JOIN review_schedule ON words.id = review_schedule.word_id \
                 WHERE review_schedule.due_at <= '{}' ORDER BY review_schedule.due_at ASC LIMIT 1",
                now_utc.replace("'", "''")
            )
        };

        self.load_one_word_from_sql(&sql)
    }

    pub fn pick_next_word(&self, srs_enabled: bool, now_utc: &str) -> Result<Option<WordRecord>, DbError> {
        self.pick_next_word_scoped(srs_enabled, now_utc, None)
    }

    pub fn pick_next_word_scoped(&self, srs_enabled: bool, now_utc: &str, topic_id: Option<i64>) -> Result<Option<WordRecord>, DbError> {
        if srs_enabled {
            if let Some(record) = self.pick_due_word_scoped(now_utc, topic_id)? {
                return Ok(Some(record));
            }
        }
        self.pick_random_word_scoped(topic_id)
    }

    pub fn set_due_at_by_word_key(&self, word_key: &str, due_at: &str) -> Result<(), DbError> {
        let conn = self.conn.borrow();
        conn.execute(
            "INSERT INTO review_schedule (word_id, due_at) \
             VALUES ((SELECT id FROM words WHERE word_key = ?), ?) \
             ON CONFLICT(word_id) DO UPDATE SET due_at = excluded.due_at",
            rusqlite::params![word_key, due_at],
        ).map_err(DbError::Sqlite)?;
        Ok(())
    }

    pub fn get_due_at_by_word_key(&self, word_key: &str) -> Result<Option<String>, DbError> {
        let conn = self.conn.borrow();
        let mut stmt = conn
            .prepare("SELECT review_schedule.due_at FROM review_schedule \
                       JOIN words ON words.id = review_schedule.word_id WHERE words.word_key = ?")
            .map_err(DbError::Sqlite)?;
        let mut rows = stmt.query([word_key]).map_err(DbError::Sqlite)?;
        let row = match rows.next().map_err(DbError::Sqlite)? {
            Some(r) => r,
            None => return Ok(None),
        };
        let due_at: String = row.get(0).map_err(DbError::Sqlite)?;
        Ok(Some(due_at))
    }

    pub fn record_review_event(&self, word_key: &str, was_correct: bool, reviewed_at: &str) -> Result<(), DbError> {
        let conn = self.conn.borrow();
        conn.execute(
            "INSERT INTO review_history (word_id, reviewed_at, was_correct) \
             VALUES ((SELECT id FROM words WHERE word_key = ?), ?, ?)",
            rusqlite::params![word_key, reviewed_at, if was_correct { 1 } else { 0 }],
        ).map_err(DbError::Sqlite)?;
        Ok(())
    }

    pub fn delete_by_word_key(&self, word_key: &str) -> Result<(), DbError> {
        let conn = self.conn.borrow();
        conn.execute("DELETE FROM words WHERE word_key = ?", [word_key]).map_err(DbError::Sqlite)?;
        Ok(())
    }

    pub fn review_history_count_by_word_key(&self, word_key: &str) -> Result<u32, DbError> {
        let conn = self.conn.borrow();
        let count: u32 = conn.query_row(
            "SELECT COUNT(*) FROM review_history WHERE word_id = (SELECT id FROM words WHERE word_key = ?)",
            [word_key],
            |row| row.get(0),
        ).map_err(DbError::Sqlite)?;
        Ok(count)
    }

    // ===== Collection CRUD =====

    pub fn list_collections(&self) -> Result<Vec<Collection>, DbError> {
        let conn = self.conn.borrow();
        let mut stmt = conn
            .prepare("SELECT id, name, description, created_at FROM collections ORDER BY name")
            .map_err(DbError::Sqlite)?;
        let rows = stmt.query_map([], |row| {
            Ok(Collection {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                created_at: row.get(3)?,
            })
        }).map_err(DbError::Sqlite)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::Sqlite)
    }

    pub fn create_collection(&self, name: &str, description: Option<&str>) -> Result<Collection, DbError> {
        let now = Utc::now().to_rfc3339();
        let conn = self.conn.borrow();
        conn.execute(
            "INSERT INTO collections (name, description, created_at) VALUES (?, ?, ?)",
            rusqlite::params![name, description, now],
        ).map_err(DbError::Sqlite)?;
        let id = conn.last_insert_rowid();
        Ok(Collection {
            id,
            name: name.to_string(),
            description: description.map(|s| s.to_string()),
            created_at: now,
        })
    }

    pub fn update_collection(&self, id: i64, name: &str, description: Option<&str>) -> Result<(), DbError> {
        let conn = self.conn.borrow();
        conn.execute(
            "UPDATE collections SET name = ?, description = ? WHERE id = ?",
            rusqlite::params![name, description, id],
        ).map_err(DbError::Sqlite)?;
        Ok(())
    }

    pub fn delete_collection(&self, id: i64) -> Result<(), DbError> {
        let conn = self.conn.borrow();
        conn.execute("DELETE FROM collections WHERE id = ?", [id]).map_err(DbError::Sqlite)?;
        Ok(())
    }

    pub fn find_or_create_collection(&self, name: &str) -> Result<Collection, DbError> {
        let conn = self.conn.borrow();
        let existing: Option<(i64, String, Option<String>, String)> = conn.query_row(
            "SELECT id, name, description, created_at FROM collections WHERE name = ?",
            [name],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        ).optional().map_err(DbError::Sqlite)?;
        drop(conn);

        if let Some((id, name, description, created_at)) = existing {
            return Ok(Collection { id, name, description, created_at });
        }
        self.create_collection(name, None)
    }

    // ===== Topic CRUD =====

    pub fn list_topics(&self, collection_id: i64) -> Result<Vec<Topic>, DbError> {
        let conn = self.conn.borrow();
        let mut stmt = conn
            .prepare("SELECT id, collection_id, name, description FROM topics WHERE collection_id = ? ORDER BY name")
            .map_err(DbError::Sqlite)?;
        let rows = stmt.query_map([collection_id], |row| {
            Ok(Topic {
                id: row.get(0)?,
                collection_id: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
            })
        }).map_err(DbError::Sqlite)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::Sqlite)
    }

    pub fn create_topic(&self, collection_id: i64, name: &str, description: Option<&str>) -> Result<Topic, DbError> {
        let conn = self.conn.borrow();
        conn.execute(
            "INSERT INTO topics (collection_id, name, description) VALUES (?, ?, ?)",
            rusqlite::params![collection_id, name, description],
        ).map_err(DbError::Sqlite)?;
        let id = conn.last_insert_rowid();
        Ok(Topic {
            id,
            collection_id,
            name: name.to_string(),
            description: description.map(|s| s.to_string()),
        })
    }

    pub fn update_topic(&self, id: i64, name: &str, description: Option<&str>) -> Result<(), DbError> {
        let conn = self.conn.borrow();
        conn.execute(
            "UPDATE topics SET name = ?, description = ? WHERE id = ?",
            rusqlite::params![name, description, id],
        ).map_err(DbError::Sqlite)?;
        Ok(())
    }

    pub fn delete_topic(&self, id: i64) -> Result<(), DbError> {
        let conn = self.conn.borrow();
        conn.execute("DELETE FROM topics WHERE id = ?", [id]).map_err(DbError::Sqlite)?;
        Ok(())
    }

    pub fn find_or_create_topic(&self, collection_id: i64, name: &str) -> Result<Topic, DbError> {
        let conn = self.conn.borrow();
        let existing: Option<(i64, i64, String, Option<String>)> = conn.query_row(
            "SELECT id, collection_id, name, description FROM topics WHERE collection_id = ? AND name = ?",
            rusqlite::params![collection_id, name],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        ).optional().map_err(DbError::Sqlite)?;
        drop(conn);

        if let Some((id, collection_id, name, description)) = existing {
            return Ok(Topic { id, collection_id, name, description });
        }
        self.create_topic(collection_id, name, None)
    }

    // ===== Word-Topic assignment =====

    pub fn assign_word_to_topic(&self, word_key: &str, topic_id: i64) -> Result<(), DbError> {
        let conn = self.conn.borrow();
        conn.execute(
            "INSERT OR IGNORE INTO word_topics (word_id, topic_id) \
             VALUES ((SELECT id FROM words WHERE word_key = ?), ?)",
            rusqlite::params![word_key, topic_id],
        ).map_err(DbError::Sqlite)?;
        Ok(())
    }

    pub fn get_word_topics(&self, word_key: &str) -> Result<Vec<Topic>, DbError> {
        let conn = self.conn.borrow();
        let mut stmt = conn.prepare(
            "SELECT t.id, t.collection_id, t.name, t.description FROM topics t \
             JOIN word_topics wt ON t.id = wt.topic_id \
             JOIN words w ON w.id = wt.word_id WHERE w.word_key = ? ORDER BY t.name",
        ).map_err(DbError::Sqlite)?;
        let rows = stmt.query_map([word_key], |row| {
            Ok(Topic {
                id: row.get(0)?,
                collection_id: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
            })
        }).map_err(DbError::Sqlite)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::Sqlite)
    }

    // ===== Private helpers =====

    fn merge_records(&self, old: &WordRecord, new: &WordRecord) -> Result<WordRecord, DbError> {
        Ok(WordRecord {
            word: new.word.clone(),
            phonetic: merge::merge_phonetic(old.phonetic.clone(), new.phonetic.clone()),
            definitions: merge::merge_definitions(old.definitions.clone(), new.definitions.clone()),
            examples: merge::merge_examples(old.examples.clone(), new.examples.clone()),
            synonyms: merge::merge_string_vec(old.synonyms.clone(), new.synonyms.clone()),
            antonyms: merge::merge_string_vec(old.antonyms.clone(), new.antonyms.clone()),
            family_words: merge::merge_string_vec(old.family_words.clone(), new.family_words.clone()),
            metadata: Metadata {
                tags: merge::merge_tags(old.metadata.tags.clone(), new.metadata.tags.clone()),
                created_at: merge::merge_created_at(old.metadata.created_at.clone(), new.metadata.created_at.clone()),
                review_count: merge::merge_review_count(old.metadata.review_count, new.metadata.review_count),
            },
        })
    }

    fn load_one_word_from_sql(&self, sql: &str) -> Result<Option<WordRecord>, DbError> {
        let conn = self.conn.borrow();
        let mut stmt = conn.prepare(sql).map_err(DbError::Sqlite)?;
        let mut rows = stmt.query([]).map_err(DbError::Sqlite)?;
        let row = match rows.next().map_err(DbError::Sqlite)? {
            Some(r) => r,
            None => return Ok(None),
        };
        let id: i64 = row.get(0).map_err(DbError::Sqlite)?;
        let word: String = row.get(1).map_err(DbError::Sqlite)?;
        let phonetic: Option<String> = row.get(2).map_err(DbError::Sqlite)?;
        let created_at: Option<String> = row.get(3).map_err(DbError::Sqlite)?;
        let review_count: u32 = row.get(4).map_err(DbError::Sqlite)?;
        drop(rows);
        drop(stmt);
        drop(conn);

        let definitions = self.load_definitions(id)?;
        let examples = self.load_examples(id)?;
        let tags = self.load_tags(id)?;
        let synonyms = self.load_relations(id, "synonym")?;
        let antonyms = self.load_relations(id, "antonym")?;
        let family_words = self.load_relations(id, "family")?;

        Ok(Some(WordRecord {
            word, phonetic, definitions, examples, synonyms, antonyms, family_words,
            metadata: Metadata { tags, created_at, review_count },
        }))
    }

    fn load_definitions(&self, word_id: i64) -> Result<Vec<Definition>, DbError> {
        let conn = self.conn.borrow();
        let mut stmt = conn
            .prepare("SELECT pos, meaning FROM definitions WHERE word_id = ?")
            .map_err(DbError::Sqlite)?;
        let defs = stmt.query_map([word_id], |row| {
            Ok(Definition { pos: row.get(0).ok(), meaning: row.get(1)? })
        }).map_err(DbError::Sqlite)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(DbError::Sqlite)?;
        Ok(defs)
    }

    fn load_examples(&self, word_id: i64) -> Result<Vec<String>, DbError> {
        let conn = self.conn.borrow();
        let mut stmt = conn
            .prepare("SELECT example FROM examples WHERE word_id = ?")
            .map_err(DbError::Sqlite)?;
        let examples = stmt.query_map([word_id], |row| row.get(0))
            .map_err(DbError::Sqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(DbError::Sqlite)?;
        Ok(examples)
    }

    fn load_tags(&self, word_id: i64) -> Result<Vec<String>, DbError> {
        let conn = self.conn.borrow();
        let mut stmt = conn
            .prepare("SELECT name FROM tags JOIN word_tags ON tags.id = word_tags.tag_id WHERE word_tags.word_id = ? ORDER BY name")
            .map_err(DbError::Sqlite)?;
        let tags = stmt.query_map([word_id], |row| row.get(0))
            .map_err(DbError::Sqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(DbError::Sqlite)?;
        Ok(tags)
    }

    fn load_relations(&self, word_id: i64, relation_type: &str) -> Result<Vec<String>, DbError> {
        let conn = self.conn.borrow();
        let mut stmt = conn
            .prepare("SELECT related_word FROM word_relations WHERE word_id = ? AND relation_type = ?")
            .map_err(DbError::Sqlite)?;
        let words = stmt.query_map(rusqlite::params![word_id, relation_type], |row| row.get(0))
            .map_err(DbError::Sqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(DbError::Sqlite)?;
        Ok(words)
    }

    fn insert_word_record_tx(tx: &rusqlite::Transaction, record: &WordRecord, now: &str) -> Result<i64, DbError> {
        tx.execute(
            "INSERT INTO words (word_key, word, phonetic, created_at, review_count, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
            rusqlite::params![record.word_key(), &record.word, &record.phonetic, &record.metadata.created_at, record.metadata.review_count, now],
        ).map_err(DbError::Sqlite)?;
        Ok(tx.last_insert_rowid())
    }

    fn initialize_schedule_tx(tx: &rusqlite::Transaction, word_id: i64, due_at: &str) -> Result<(), DbError> {
        tx.execute(
            "INSERT OR IGNORE INTO review_schedule (word_id, due_at) VALUES (?, ?)",
            rusqlite::params![word_id, due_at],
        ).map_err(DbError::Sqlite)?;
        Ok(())
    }

    fn insert_definitions_tx(tx: &rusqlite::Transaction, word_id: i64, definitions: &[Definition]) -> Result<(), DbError> {
        for def in definitions {
            tx.execute(
                "INSERT INTO definitions (word_id, pos, meaning) VALUES (?, ?, ?)",
                rusqlite::params![word_id, &def.pos, &def.meaning],
            ).map_err(DbError::Sqlite)?;
        }
        Ok(())
    }

    fn insert_examples_tx(tx: &rusqlite::Transaction, word_id: i64, examples: &[String]) -> Result<(), DbError> {
        for ex in examples {
            tx.execute(
                "INSERT INTO examples (word_id, example) VALUES (?, ?)",
                rusqlite::params![word_id, ex],
            ).map_err(DbError::Sqlite)?;
        }
        Ok(())
    }

    fn insert_tags_tx(tx: &rusqlite::Transaction, word_id: i64, tags: &[String]) -> Result<(), DbError> {
        for tag in tags {
            tx.execute("INSERT OR IGNORE INTO tags (name) VALUES (?)", rusqlite::params![tag]).map_err(DbError::Sqlite)?;
            let tag_id: i64 = tx.query_row("SELECT id FROM tags WHERE name = ?", [tag], |row| row.get(0)).map_err(DbError::Sqlite)?;
            tx.execute("INSERT OR IGNORE INTO word_tags (word_id, tag_id) VALUES (?, ?)", rusqlite::params![word_id, tag_id]).map_err(DbError::Sqlite)?;
        }
        Ok(())
    }

    fn insert_relations_tx(tx: &rusqlite::Transaction, word_id: i64, words: &[String], relation_type: &str) -> Result<(), DbError> {
        for w in words {
            tx.execute(
                "INSERT OR IGNORE INTO word_relations (word_id, related_word, relation_type) VALUES (?, ?, ?)",
                rusqlite::params![word_id, w, relation_type],
            ).map_err(DbError::Sqlite)?;
        }
        Ok(())
    }

    fn update_word_record_tx(tx: &rusqlite::Transaction, word_key: &str, record: &WordRecord, now: &str) -> Result<(), DbError> {
        tx.execute(
            "UPDATE words SET word = ?, phonetic = ?, created_at = ?, review_count = ?, updated_at = ? WHERE word_key = ?",
            rusqlite::params![&record.word, &record.phonetic, &record.metadata.created_at, record.metadata.review_count, now, word_key],
        ).map_err(DbError::Sqlite)?;
        Ok(())
    }

    fn replace_definitions_tx(tx: &rusqlite::Transaction, word_key: &str, definitions: &[Definition]) -> Result<(), DbError> {
        tx.execute(
            "DELETE FROM definitions WHERE word_id = (SELECT id FROM words WHERE word_key = ?)",
            [word_key],
        ).map_err(DbError::Sqlite)?;
        let word_id: i64 = tx.query_row("SELECT id FROM words WHERE word_key = ?", [word_key], |row| row.get(0)).map_err(DbError::Sqlite)?;
        Self::insert_definitions_tx(tx, word_id, definitions)
    }

    fn replace_examples_tx(tx: &rusqlite::Transaction, word_key: &str, examples: &[String]) -> Result<(), DbError> {
        tx.execute(
            "DELETE FROM examples WHERE word_id = (SELECT id FROM words WHERE word_key = ?)",
            [word_key],
        ).map_err(DbError::Sqlite)?;
        let word_id: i64 = tx.query_row("SELECT id FROM words WHERE word_key = ?", [word_key], |row| row.get(0)).map_err(DbError::Sqlite)?;
        Self::insert_examples_tx(tx, word_id, examples)
    }

    fn replace_tags_tx(tx: &rusqlite::Transaction, word_key: &str, tags: &[String]) -> Result<(), DbError> {
        tx.execute(
            "DELETE FROM word_tags WHERE word_id = (SELECT id FROM words WHERE word_key = ?)",
            [word_key],
        ).map_err(DbError::Sqlite)?;
        let word_id: i64 = tx.query_row("SELECT id FROM words WHERE word_key = ?", [word_key], |row| row.get(0)).map_err(DbError::Sqlite)?;
        Self::insert_tags_tx(tx, word_id, tags)
    }

    fn replace_relations_tx(tx: &rusqlite::Transaction, word_key: &str, words: &[String], relation_type: &str) -> Result<(), DbError> {
        tx.execute(
            "DELETE FROM word_relations WHERE word_id = (SELECT id FROM words WHERE word_key = ?) AND relation_type = ?",
            rusqlite::params![word_key, relation_type],
        ).map_err(DbError::Sqlite)?;
        let word_id: i64 = tx.query_row("SELECT id FROM words WHERE word_key = ?", [word_key], |row| row.get(0)).map_err(DbError::Sqlite)?;
        Self::insert_relations_tx(tx, word_id, words, relation_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_record(word: &str, meaning: &str) -> WordRecord {
        WordRecord {
            word: word.to_string(),
            phonetic: None,
            definitions: vec![Definition { pos: None, meaning: meaning.to_string() }],
            examples: vec![],
            synonyms: vec![],
            antonyms: vec![],
            family_words: vec![],
            metadata: Metadata {
                tags: vec![],
                created_at: Some("2026-04-20T10:00:00Z".to_string()),
                review_count: 0,
            },
        }
    }

    #[test]
    fn test_upsert_new_record() {
        let db = crate::db::init_memory_db().unwrap();
        let repo = WordRepository::new(db);
        let record = create_test_record("test", "a trial");
        let result = repo.upsert(&record).unwrap();
        assert!(result.inserted);
        assert_eq!(result.definitions_count, 1);
    }

    #[test]
    fn test_upsert_merge_phonetic() {
        let db = crate::db::init_memory_db().unwrap();
        let repo = WordRepository::new(db);
        let mut r1 = create_test_record("test", "a trial");
        r1.phonetic = Some("/test/".to_string());
        repo.upsert(&r1).unwrap();
        let mut r2 = create_test_record("test", "an exam");
        r2.phonetic = Some("/updated/".to_string());
        let result = repo.upsert(&r2).unwrap();
        assert!(!result.inserted);
        let loaded = repo.get_by_word_key("test").unwrap().unwrap();
        assert_eq!(loaded.phonetic, Some("/updated/".to_string()));
    }

    #[test]
    fn test_upsert_merges_synonyms() {
        let db = crate::db::init_memory_db().unwrap();
        let repo = WordRepository::new(db);
        let mut r1 = create_test_record("test", "a trial");
        r1.synonyms = vec!["exam".to_string()];
        repo.upsert(&r1).unwrap();
        let mut r2 = create_test_record("test", "a trial");
        r2.synonyms = vec!["quiz".to_string()];
        repo.upsert(&r2).unwrap();
        let loaded = repo.get_by_word_key("test").unwrap().unwrap();
        assert_eq!(loaded.synonyms.len(), 2);
        assert!(loaded.synonyms.contains(&"exam".to_string()));
        assert!(loaded.synonyms.contains(&"quiz".to_string()));
    }

    #[test]
    fn test_get_by_word_key() {
        let db = crate::db::init_memory_db().unwrap();
        let repo = WordRepository::new(db);
        let record = create_test_record("test", "a trial");
        repo.upsert(&record).unwrap();
        let loaded = repo.get_by_word_key("test").unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().word, "test");
    }

    #[test]
    fn test_collection_crud() {
        let db = crate::db::init_memory_db().unwrap();
        let repo = WordRepository::new(db);
        let col = repo.create_collection("Test Collection", Some("A test")).unwrap();
        assert_eq!(col.name, "Test Collection");
        let cols = repo.list_collections().unwrap();
        assert_eq!(cols.len(), 1);
        repo.delete_collection(col.id).unwrap();
        assert!(repo.list_collections().unwrap().is_empty());
    }

    #[test]
    fn test_topic_crud() {
        let db = crate::db::init_memory_db().unwrap();
        let repo = WordRepository::new(db);
        let col = repo.create_collection("My Collection", None).unwrap();
        let topic = repo.create_topic(col.id, "Food", None).unwrap();
        assert_eq!(topic.name, "Food");
        let topics = repo.list_topics(col.id).unwrap();
        assert_eq!(topics.len(), 1);
    }

    #[test]
    fn test_word_topic_assignment() {
        let db = crate::db::init_memory_db().unwrap();
        let repo = WordRepository::new(db);
        let record = create_test_record("apple", "quả táo");
        repo.upsert(&record).unwrap();
        let col = repo.create_collection("Food", None).unwrap();
        let topic = repo.create_topic(col.id, "Fruits", None).unwrap();
        repo.assign_word_to_topic("apple", topic.id).unwrap();
        let word_topics = repo.get_word_topics("apple").unwrap();
        assert_eq!(word_topics.len(), 1);
        assert_eq!(word_topics[0].name, "Fruits");
    }
}
