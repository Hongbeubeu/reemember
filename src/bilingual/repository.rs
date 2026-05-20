use super::parser::BilingualArticleInput;
use super::{BilingualArticle, BilingualArticleSummary, BilingualSegment, BilingualStructure};
use crate::db::DbError;
use chrono::Utc;
use rusqlite::{Connection, OptionalExtension};
use std::cell::RefCell;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BilingualArticleUpsertResult {
    pub id: i64,
    pub inserted: bool,
}

pub struct BilingualRepository {
    conn: RefCell<Connection>,
}

impl BilingualRepository {
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: RefCell::new(conn),
        }
    }

    pub fn list_articles(&self) -> Result<Vec<BilingualArticleSummary>, DbError> {
        let conn = self.conn.borrow();
        let mut stmt = conn
            .prepare(
                "SELECT id, title, book, level, paragraphs, structures, created_at \
                 FROM bilingual_articles ORDER BY book, title",
            )
            .map_err(DbError::Sqlite)?;

        let rows = stmt
            .query_map([], |row| {
                let paragraphs_json: String = row.get(4)?;
                let structures_json: String = row.get(5)?;
                let paragraphs: Vec<Vec<BilingualSegment>> =
                    serde_json::from_str(&paragraphs_json).unwrap_or_default();
                let structures: Vec<BilingualStructure> =
                    serde_json::from_str(&structures_json).unwrap_or_default();

                Ok(BilingualArticleSummary {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    book: row.get(2)?,
                    level: row.get(3)?,
                    paragraph_count: paragraphs.len(),
                    structure_count: structures.len(),
                    created_at: row.get(6)?,
                })
            })
            .map_err(DbError::Sqlite)?;

        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::Sqlite)
    }

    pub fn get_article(&self, id: i64) -> Result<Option<BilingualArticle>, DbError> {
        let conn = self.conn.borrow();
        let article = conn
            .query_row(
                "SELECT id, title, book, level, paragraphs, structures, created_at \
                 FROM bilingual_articles WHERE id = ?",
                [id],
                |row| {
                    let paragraphs_json: String = row.get(4)?;
                    let structures_json: String = row.get(5)?;
                    let paragraphs = serde_json::from_str(&paragraphs_json).unwrap_or_default();
                    let structures = serde_json::from_str(&structures_json).unwrap_or_default();

                    Ok(BilingualArticle {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        book: row.get(2)?,
                        level: row.get(3)?,
                        paragraphs,
                        structures,
                        created_at: row.get(6)?,
                    })
                },
            )
            .optional()
            .map_err(DbError::Sqlite)?;
        Ok(article)
    }

    pub fn upsert_article(
        &self,
        input: &BilingualArticleInput,
    ) -> Result<BilingualArticleUpsertResult, DbError> {
        let title = normalize_required(&input.title, "title")?;
        let book = normalize_required(&input.book, "book")?;
        let level = normalize_optional(&input.level);
        let now = Utc::now().to_rfc3339();
        let paragraphs_json = serde_json::to_string(&input.paragraphs)
            .map_err(|e| DbError::Validation(e.to_string()))?;
        let structures_json = serde_json::to_string(&input.structures)
            .map_err(|e| DbError::Validation(e.to_string()))?;

        let conn = self.conn.borrow();
        let existing_id = conn
            .query_row(
                "SELECT id FROM bilingual_articles WHERE book = ? AND title = ?",
                rusqlite::params![&book, &title],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(DbError::Sqlite)?;

        if let Some(id) = existing_id {
            conn.execute(
                "UPDATE bilingual_articles \
                 SET level = ?, paragraphs = ?, structures = ?, updated_at = ? \
                 WHERE id = ?",
                rusqlite::params![&level, &paragraphs_json, &structures_json, &now, id],
            )
            .map_err(DbError::Sqlite)?;
            Ok(BilingualArticleUpsertResult {
                id,
                inserted: false,
            })
        } else {
            conn.execute(
                "INSERT INTO bilingual_articles \
                 (title, book, level, paragraphs, structures, created_at, updated_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
                rusqlite::params![
                    &title,
                    &book,
                    &level,
                    &paragraphs_json,
                    &structures_json,
                    &now,
                    &now
                ],
            )
            .map_err(DbError::Sqlite)?;
            Ok(BilingualArticleUpsertResult {
                id: conn.last_insert_rowid(),
                inserted: true,
            })
        }
    }
}

fn normalize_optional(value: &Option<String>) -> Option<String> {
    value
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

fn normalize_required(value: &str, field_name: &str) -> Result<String, DbError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(DbError::Validation(format!("{} is required", field_name)));
    }
    Ok(trimmed.to_string())
}
