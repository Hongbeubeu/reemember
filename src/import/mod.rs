/// Import module for loading vocabulary records from files.
/// Combines parser (Phase 1) + repository (Phase 2) with comprehensive reporting.

use crate::parser::{self, CsvRowError};
use crate::repository::WordRepository;
use crate::db::DbError;

/// Status of a single record during import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportStatus {
    /// Record successfully inserted (new word)
    Inserted,
    /// Record successfully merged with existing word
    Updated,
    /// Record skipped due to validation error
    Skipped(String),
}

/// Detailed information about one record's import result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportRowResult {
    pub word: Option<String>,  // Word value if available
    pub status: ImportStatus,
}

/// Comprehensive report of an import operation.
/// Tracks statistics and detailed per-row results.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ImportReport {
    /// Number of records successfully inserted
    pub inserted_count: usize,
    /// Number of records successfully updated via merge
    pub updated_count: usize,
    /// Number of records skipped due to validation errors
    pub skipped_count: usize,
    /// Per-row errors from CSV parsing (before validation)
    pub csv_errors: Vec<CsvRowError>,
    /// Detailed results for each record
    pub results: Vec<ImportRowResult>,
}

impl ImportReport {
    /// Total number of processed records (inserted + updated + skipped)
    pub fn total_processed(&self) -> usize {
        self.inserted_count + self.updated_count + self.skipped_count
    }

    /// Total number of errors (CSV parsing + skipped validation)
    pub fn total_errors(&self) -> usize {
        self.csv_errors.len() + self.skipped_count
    }
}

/// Import service for loading records from JSON or CSV format.
/// Combines parsing with repository upsert for atomic operations.
pub struct ImportService;

impl ImportService {
    /// Import records from a JSON string.
    /// Parses JSON canonical format and upserts each record into repository.
    ///
    /// # Parameters
    /// - `repo`: Target repository for upsert
    /// - `json_str`: JSON array string to parse
    ///
    /// # Returns
    /// - `Ok(ImportReport)`: Import completed with detailed results
    /// - `Err(DbError)`: Fatal database error (stops import)
    ///
    /// # Example
    /// ```ignore
    /// let json = r#"[{"word": "test", "definitions": [...]}]"#;
    /// let report = ImportService::import_from_json_string(&repo, json)?;
    /// println!("Inserted: {}, Updated: {}, Errors: {}",
    ///          report.inserted_count, report.updated_count, report.total_errors());
    /// ```
    pub fn import_from_json_string(
        repo: &WordRepository,
        json_str: &str,
    ) -> Result<ImportReport, DbError> {
        Self::import_from_json_string_scoped(repo, json_str, None, None)
    }

    /// Import from JSON, optionally assigning words to a collection/topic.
    /// `collection_name` and `topic_name` override anything embedded in the JSON bundle.
    pub fn import_from_json_string_scoped(
        repo: &WordRepository,
        json_str: &str,
        collection_name: Option<&str>,
        topic_name: Option<&str>,
    ) -> Result<ImportReport, DbError> {
        let bundle = parser::parse_json_bundle(json_str)
            .map_err(|e| DbError::Validation(format!("JSON parse error: {}", e)))?;

        let effective_collection = collection_name
            .or_else(|| bundle.collection.as_deref());
        let effective_topic = topic_name
            .or_else(|| bundle.topic.as_deref());

        let topic_id = if let Some(cname) = effective_collection {
            let collection = repo.find_or_create_collection(cname)?;
            if let Some(tname) = effective_topic {
                Some(repo.find_or_create_topic(collection.id, tname)?.id)
            } else {
                None
            }
        } else {
            None
        };

        let mut report = ImportReport::default();
        for record in bundle.words {
            let word_key = record.word_key();
            let word = record.word.clone();
            match repo.upsert(&record) {
                Ok(result) => {
                    if let Some(tid) = topic_id {
                        let _ = repo.assign_word_to_topic(&word_key, tid);
                    }
                    if result.inserted {
                        report.inserted_count += 1;
                        report.results.push(ImportRowResult { word: Some(word), status: ImportStatus::Inserted });
                    } else {
                        report.updated_count += 1;
                        report.results.push(ImportRowResult { word: Some(word), status: ImportStatus::Updated });
                    }
                }
                Err(e) => {
                    report.skipped_count += 1;
                    report.results.push(ImportRowResult { word: Some(word), status: ImportStatus::Skipped(e.to_string()) });
                }
            }
        }

        Ok(report)
    }

    /// Import records from a CSV string.
    /// Parses CSV (minimal or extended format) and upserts each valid record.
    /// Per-row parsing errors are collected but don't stop processing.
    ///
    /// # Parameters
    /// - `repo`: Target repository for upsert
    /// - `csv_str`: CSV string to parse
    ///
    /// # Returns
    /// - `Ok(ImportReport)`: Import completed with detailed results (including CSV errors)
    /// - `Err(DbError)`: Fatal database error (stops import)
    ///
    /// # CSV Formats Supported
    /// - Minimal: `word,meaning_vi`
    /// - Extended: `word,meaning_vi,phonetic,pos,examples,tags,created_at,review_count`
    ///
    /// # Example
    /// ```ignore
    /// let csv = "word,meaning_vi\ntest,trial\nbad,\n";
    /// let report = ImportService::import_from_csv_string(&repo, csv)?;
    /// println!("CSV errors: {}", report.csv_errors.len());
    /// ```
    pub fn import_from_csv_string(
        repo: &WordRepository,
        csv_str: &str,
    ) -> Result<ImportReport, DbError> {
        let csv_report = parser::parse_csv(csv_str)?;

        let mut report = ImportReport::default();
        report.csv_errors = csv_report.errors;

        for record in csv_report.records {
            let word = record.word.clone();
            match repo.upsert(&record) {
                Ok(result) => {
                    if result.inserted {
                        report.inserted_count += 1;
                        report.results.push(ImportRowResult {
                            word: Some(word),
                            status: ImportStatus::Inserted,
                        });
                    } else {
                        report.updated_count += 1;
                        report.results.push(ImportRowResult {
                            word: Some(word),
                            status: ImportStatus::Updated,
                        });
                    }
                }
                Err(e) => {
                    report.skipped_count += 1;
                    report.results.push(ImportRowResult {
                        word: Some(word),
                        status: ImportStatus::Skipped(e.to_string()),
                    });
                }
            }
        }

        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_report_total_processed() {
        let mut report = ImportReport::default();
        report.inserted_count = 5;
        report.updated_count = 3;
        report.skipped_count = 2;

        assert_eq!(report.total_processed(), 10);
    }

    #[test]
    fn test_import_report_total_errors() {
        let mut report = ImportReport::default();
        report.skipped_count = 2;
        report.csv_errors = vec![CsvRowError {
            row: 3,
            message: "test".to_string(),
        }];

        assert_eq!(report.total_errors(), 3);
    }
}


