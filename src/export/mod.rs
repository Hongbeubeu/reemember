/// Export module for writing vocabulary records to files.
/// Supports JSON canonical and CSV extended formats with round-trip guarantee.

use crate::repository::{WordRepository, QueryOptions};
use crate::db::DbError;
use serde_json;

/// Supported export formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// JSON array format (canonical for backup)
    Json,
    /// CSV extended format with all fields
    Csv,
}

/// Export service for writing records to files or strings.
/// Ensures data integrity with round-trip support.
pub struct ExportService;

impl ExportService {
    /// Export all records from repository as JSON string.
    /// Uses canonical format for backup/restore reliability.
    ///
    /// # Parameters
    /// - `repo`: Repository to export from
    ///
    /// # Returns
    /// - `Ok(String)`: JSON array string (pretty-printed)
    /// - `Err(DbError)`: Query or serialization failed
    ///
    /// # JSON Format
    /// ```json
    /// [
    ///   {
    ///     "word": "Consistency",
    ///     "phonetic": "/kənˈsɪstənsi/",
    ///     "definitions": [{"pos": "noun", "meaning": "..."}],
    ///     "examples": [...],
    ///     "metadata": {"tags": [...], "created_at": "...", "review_count": 0}
    ///   }
    /// ]
    /// ```
    ///
    /// # Example
    /// ```ignore
    /// let json = ExportService::export_to_json_string(&repo)?;
    /// std::fs::write("backup.json", &json)?;
    /// ```
    pub fn export_to_json_string(repo: &WordRepository) -> Result<String, DbError> {
        let records = repo.query(&QueryOptions::new())?;
        let json =
            serde_json::to_string_pretty(&records).map_err(|e| DbError::Validation(e.to_string()))?;
        Ok(json)
    }

    /// Export all records from repository as CSV string (extended format).
    /// Format includes all fields: word, meaning_vi, phonetic, pos, examples, tags, created_at, review_count.
    ///
    /// # Parameters
    /// - `repo`: Repository to export from
    ///
    /// # Returns
    /// - `Ok(String)`: CSV string with header and data rows
    /// - `Err(DbError)`: Query failed
    ///
    /// # CSV Columns
    /// word, meaning_vi, phonetic, pos, examples, tags, created_at, review_count
    ///
    /// # Delimiters
    /// - Multiple meanings separated by `;`
    /// - Multiple examples separated by `|`
    /// - Multiple tags separated by `|`
    ///
    /// # Example
    /// ```ignore
    /// let csv = ExportService::export_to_csv_string(&repo)?;
    /// std::fs::write("words.csv", &csv)?;
    /// ```
    pub fn export_to_csv_string(repo: &WordRepository) -> Result<String, DbError> {
        let records = repo.query(&QueryOptions::new())?;

        let mut csv = String::new();
        csv.push_str("word,meaning_vi,phonetic,pos,examples,tags,created_at,review_count\n");

        for record in records {
            let word = Self::escape_csv_field(&record.word);
            let meanings = record
                .definitions
                .iter()
                .map(|d| d.meaning.as_str())
                .collect::<Vec<_>>()
                .join("; ");
            let meaning_vi = Self::escape_csv_field(&meanings);
            let phonetic = Self::escape_csv_field(record.phonetic.as_deref().unwrap_or(""));
            let pos = record
                .definitions
                .first()
                .and_then(|d| d.pos.as_ref())
                .map(|p| p.as_str())
                .unwrap_or("");
            let pos = Self::escape_csv_field(pos);
            let examples = Self::escape_csv_field(&record.examples.join(" | "));
            let tags = Self::escape_csv_field(&record.metadata.tags.join(" | "));
            let created_at = Self::escape_csv_field(record.metadata.created_at.as_deref().unwrap_or(""));
            let review_count = record.metadata.review_count;

            csv.push_str(&format!(
                "{},{},{},{},{},{},{},{}\n",
                word, meaning_vi, phonetic, pos, examples, tags, created_at, review_count
            ));
        }

        Ok(csv)
    }

    /// Escape special characters in CSV field value.
    /// Fields containing commas, quotes, or newlines are quoted.
    fn escape_csv_field(field: &str) -> String {
        if field.contains(',') || field.contains('"') || field.contains('\n') {
            format!("\"{}\"", field.replace('"', "\"\""))
        } else {
            field.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_csv_field_no_special_chars() {
        let result = ExportService::escape_csv_field("simple");
        assert_eq!(result, "simple");
    }

    #[test]
    fn test_escape_csv_field_with_comma() {
        let result = ExportService::escape_csv_field("hello, world");
        assert_eq!(result, "\"hello, world\"");
    }

    #[test]
    fn test_escape_csv_field_with_quote() {
        let result = ExportService::escape_csv_field("say \"hello\"");
        assert_eq!(result, "\"say \"\"hello\"\"\"");
    }

    #[test]
    fn test_escape_csv_field_with_newline() {
        let result = ExportService::escape_csv_field("line1\nline2");
        assert_eq!(result, "\"line1\nline2\"");
    }
}


