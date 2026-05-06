use std::cell::RefCell;
use rusqlite::Connection;
use chrono::Utc;
use crate::db::DbError;
use super::{GrammarDoc, GrammarDocDetail, GrammarDocSummary, GrammarExercise, GrammarGroup, GrammarGroupSummary};
use super::parser::GrammarDocInput;

pub struct GrammarRepository {
    conn: RefCell<Connection>,
}

impl GrammarRepository {
    pub fn new(conn: Connection) -> Self {
        GrammarRepository { conn: RefCell::new(conn) }
    }

    // ── Docs ─────────────────────────────────────────────────────────────────

    pub fn list_docs(&self) -> Result<Vec<GrammarDocSummary>, DbError> {
        let conn = self.conn.borrow();
        let mut stmt = conn.prepare(
            "SELECT g.id, g.title, g.category, g.level, g.group_id, g.created_at, \
             COUNT(e.id) as exercise_count \
             FROM grammar_docs g \
             LEFT JOIN grammar_exercises e ON e.doc_id = g.id \
             GROUP BY g.id ORDER BY g.created_at DESC",
        ).map_err(DbError::Sqlite)?;

        let rows = stmt.query_map([], |row| {
            Ok(GrammarDocSummary {
                id: row.get(0)?,
                title: row.get(1)?,
                category: row.get(2)?,
                level: row.get(3)?,
                group_id: row.get(4)?,
                created_at: row.get(5)?,
                exercise_count: row.get::<_, i64>(6)? as usize,
            })
        }).map_err(DbError::Sqlite)?;

        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::Sqlite)
    }

    pub fn get_doc_with_exercises(&self, id: i64) -> Result<Option<GrammarDocDetail>, DbError> {
        let conn = self.conn.borrow();

        let doc = {
            let mut stmt = conn.prepare(
                "SELECT id, title, category, level, content, examples, group_id, created_at \
                 FROM grammar_docs WHERE id = ?",
            ).map_err(DbError::Sqlite)?;
            let mut rows = stmt.query([id]).map_err(DbError::Sqlite)?;
            match rows.next().map_err(DbError::Sqlite)? {
                None => return Ok(None),
                Some(row) => GrammarDoc {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    category: row.get(2)?,
                    level: row.get(3)?,
                    content: row.get(4)?,
                    examples: serde_json::from_str(&row.get::<_, String>(5)?)
                        .unwrap_or_default(),
                    group_id: row.get(6)?,
                    created_at: row.get(7)?,
                },
            }
        };

        let exercises = {
            let mut stmt = conn.prepare(
                "SELECT id, doc_id, order_index, exercise_type, data \
                 FROM grammar_exercises WHERE doc_id = ? ORDER BY order_index",
            ).map_err(DbError::Sqlite)?;
            let rows = stmt.query_map([id], |row| {
                Ok(GrammarExercise {
                    id: row.get(0)?,
                    doc_id: row.get(1)?,
                    order_index: row.get(2)?,
                    exercise_type: row.get(3)?,
                    data: serde_json::from_str(&row.get::<_, String>(4)?)
                        .unwrap_or(serde_json::Value::Null),
                })
            }).map_err(DbError::Sqlite)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(DbError::Sqlite)?
        };

        Ok(Some(GrammarDocDetail { doc, exercises }))
    }

    pub fn insert_doc(&self, input: &GrammarDocInput, group_id: Option<i64>) -> Result<i64, DbError> {
        let now = Utc::now().to_rfc3339();
        let examples_json = serde_json::to_string(&input.examples)
            .map_err(|e| DbError::Validation(e.to_string()))?;

        let mut conn = self.conn.borrow_mut();
        let tx = conn.transaction().map_err(DbError::Sqlite)?;

        tx.execute(
            "INSERT INTO grammar_docs (title, category, level, content, examples, group_id, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            rusqlite::params![
                &input.title, &input.category, &input.level,
                &input.content, &examples_json, &group_id, &now
            ],
        ).map_err(DbError::Sqlite)?;

        let doc_id = tx.last_insert_rowid();

        for (i, exercise) in input.exercises.iter().enumerate() {
            let exercise_type = exercise.get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            let data_json = serde_json::to_string(exercise)
                .map_err(|e| DbError::Validation(e.to_string()))?;
            tx.execute(
                "INSERT INTO grammar_exercises (doc_id, order_index, exercise_type, data) \
                 VALUES (?, ?, ?, ?)",
                rusqlite::params![doc_id, i as i32, &exercise_type, &data_json],
            ).map_err(DbError::Sqlite)?;
        }

        tx.commit().map_err(DbError::Sqlite)?;
        Ok(doc_id)
    }

    pub fn delete_doc(&self, id: i64) -> Result<(), DbError> {
        let conn = self.conn.borrow();
        conn.execute("DELETE FROM grammar_docs WHERE id = ?", [id])
            .map_err(DbError::Sqlite)?;
        Ok(())
    }

    pub fn move_doc(&self, doc_id: i64, group_id: Option<i64>) -> Result<(), DbError> {
        let conn = self.conn.borrow();
        conn.execute(
            "UPDATE grammar_docs SET group_id = ? WHERE id = ?",
            rusqlite::params![group_id, doc_id],
        ).map_err(DbError::Sqlite)?;
        Ok(())
    }

    // ── Groups ───────────────────────────────────────────────────────────────

    pub fn list_groups(&self) -> Result<Vec<GrammarGroupSummary>, DbError> {
        let conn = self.conn.borrow();
        let mut stmt = conn.prepare(
            "SELECT g.id, g.name, g.description, g.sort_order, g.created_at, \
             COUNT(d.id) as doc_count \
             FROM grammar_groups g \
             LEFT JOIN grammar_docs d ON d.group_id = g.id \
             GROUP BY g.id ORDER BY g.sort_order, g.name",
        ).map_err(DbError::Sqlite)?;

        let rows = stmt.query_map([], |row| {
            Ok(GrammarGroupSummary {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                sort_order: row.get(3)?,
                created_at: row.get(4)?,
                doc_count: row.get::<_, i64>(5)? as usize,
            })
        }).map_err(DbError::Sqlite)?;

        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::Sqlite)
    }

    pub fn create_group(&self, name: &str, description: Option<&str>) -> Result<GrammarGroup, DbError> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(DbError::Validation("group name is required".into()));
        }
        let now = Utc::now().to_rfc3339();
        let conn = self.conn.borrow();
        conn.execute(
            "INSERT INTO grammar_groups (name, description, sort_order, created_at) VALUES (?, ?, 0, ?)",
            rusqlite::params![trimmed, description, &now],
        ).map_err(DbError::Sqlite)?;
        let id = conn.last_insert_rowid();
        Ok(GrammarGroup {
            id,
            name: trimmed.to_string(),
            description: description.map(|s| s.to_string()),
            sort_order: 0,
            created_at: now,
        })
    }

    /// Find a group by exact name (case-sensitive); create if absent.
    pub fn find_or_create_group(&self, name: &str) -> Result<i64, DbError> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(DbError::Validation("group name is required".into()));
        }
        let conn = self.conn.borrow();
        let existing: Option<i64> = conn
            .query_row(
                "SELECT id FROM grammar_groups WHERE name = ?",
                rusqlite::params![trimmed],
                |row| row.get(0),
            )
            .ok();
        if let Some(id) = existing {
            return Ok(id);
        }
        drop(conn);
        let group = self.create_group(trimmed, None)?;
        Ok(group.id)
    }

    pub fn update_group(&self, id: i64, name: Option<&str>, description: Option<Option<&str>>) -> Result<(), DbError> {
        let conn = self.conn.borrow();
        if let Some(name) = name {
            let trimmed = name.trim();
            if trimmed.is_empty() {
                return Err(DbError::Validation("group name is required".into()));
            }
            conn.execute(
                "UPDATE grammar_groups SET name = ? WHERE id = ?",
                rusqlite::params![trimmed, id],
            ).map_err(DbError::Sqlite)?;
        }
        if let Some(desc) = description {
            conn.execute(
                "UPDATE grammar_groups SET description = ? WHERE id = ?",
                rusqlite::params![desc, id],
            ).map_err(DbError::Sqlite)?;
        }
        Ok(())
    }

    /// Delete the group; docs in it become Ungrouped (group_id = NULL).
    /// Done explicitly because pre-existing DBs may lack the FK ON DELETE SET NULL.
    pub fn delete_group(&self, id: i64) -> Result<(), DbError> {
        let mut conn = self.conn.borrow_mut();
        let tx = conn.transaction().map_err(DbError::Sqlite)?;
        tx.execute("UPDATE grammar_docs SET group_id = NULL WHERE group_id = ?", [id])
            .map_err(DbError::Sqlite)?;
        tx.execute("DELETE FROM grammar_groups WHERE id = ?", [id])
            .map_err(DbError::Sqlite)?;
        tx.commit().map_err(DbError::Sqlite)?;
        Ok(())
    }
}
