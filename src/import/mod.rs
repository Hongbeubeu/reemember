use crate::parser;
use crate::repository::WordRepository;
use crate::db::DbError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportStatus {
    Inserted,
    Updated,
    Skipped(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportRowResult {
    pub word: Option<String>,
    pub status: ImportStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ImportReport {
    pub inserted_count: usize,
    pub updated_count: usize,
    pub skipped_count: usize,
    pub results: Vec<ImportRowResult>,
}

impl ImportReport {
    pub fn total_processed(&self) -> usize {
        self.inserted_count + self.updated_count + self.skipped_count
    }

    pub fn total_errors(&self) -> usize {
        self.skipped_count
    }
}

pub struct ImportService;

impl ImportService {
    pub fn import_from_json_string(
        repo: &WordRepository,
        json_str: &str,
    ) -> Result<ImportReport, DbError> {
        Self::import_from_json_string_scoped(repo, json_str, None, None)
    }

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

        assert_eq!(report.total_errors(), 2);
    }
}
