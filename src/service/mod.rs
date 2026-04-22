/// Service layer coordinating import/export operations.
/// Provides high-level API for multi-format data mobility.

pub use crate::import::{ImportService, ImportReport, ImportStatus, ImportRowResult};
pub use crate::export::{ExportService, ExportFormat};
pub use crate::testing::{AnswerResult, Question, TestMode, TestingEngine, TestingOptions};

/// VocabularyService provides unified interface for import/export operations.
/// Combines parser (Phase 1), repository (Phase 2), and import/export (Phase 3).
pub struct VocabularyService;

impl VocabularyService {
    /// Import records from JSON string with comprehensive reporting.
    ///
    /// # Parameters
    /// - `repo`: Target repository
    /// - `json_str`: JSON array string
    ///
    /// # Returns
    /// ImportReport with per-record status
    ///
    /// # Example
    /// ```ignore
    /// let json = std::fs::read_to_string("words.json")?;
    /// let report = VocabularyService::import_json(&repo, &json)?;
    /// ```
    pub fn import_json(
        repo: &crate::repository::WordRepository,
        json_str: &str,
    ) -> Result<ImportReport, crate::db::DbError> {
        ImportService::import_from_json_string(repo, json_str)
    }

    pub fn import_json_scoped(
        repo: &crate::repository::WordRepository,
        json_str: &str,
        collection_name: Option<&str>,
        topic_name: Option<&str>,
    ) -> Result<ImportReport, crate::db::DbError> {
        ImportService::import_from_json_string_scoped(repo, json_str, collection_name, topic_name)
    }

    /// Import records from CSV string with comprehensive reporting.
    ///
    /// # Parameters
    /// - `repo`: Target repository
    /// - `csv_str`: CSV string (minimal or extended format)
    ///
    /// # Returns
    /// ImportReport with per-row status and errors
    pub fn import_csv(
        repo: &crate::repository::WordRepository,
        csv_str: &str,
    ) -> Result<ImportReport, crate::db::DbError> {
        ImportService::import_from_csv_string(repo, csv_str)
    }

    /// Export all records to JSON string (canonical format).
    ///
    /// # Parameters
    /// - `repo`: Source repository
    ///
    /// # Returns
    /// Pretty-printed JSON array
    pub fn export_json(repo: &crate::repository::WordRepository) -> Result<String, crate::db::DbError> {
        ExportService::export_to_json_string(repo)
    }

    /// Export all records to CSV string (extended format).
    ///
    /// # Parameters
    /// - `repo`: Source repository
    ///
    /// # Returns
    /// CSV string with header and data rows
    pub fn export_csv(repo: &crate::repository::WordRepository) -> Result<String, crate::db::DbError> {
        ExportService::export_to_csv_string(repo)
    }

    /// Generate one testing question for a selected mode.
    pub fn next_question(
        repo: &crate::repository::WordRepository,
        mode: TestMode,
    ) -> Result<Option<Question>, crate::db::DbError> {
        TestingEngine::generate_question(repo, mode)
    }

    /// Generate one question with explicit SRS toggle and optional topic scope.
    pub fn next_question_with_srs(
        repo: &crate::repository::WordRepository,
        mode: TestMode,
        srs_enabled: bool,
    ) -> Result<Option<Question>, crate::db::DbError> {
        TestingEngine::generate_question_with_options(repo, mode, TestingOptions { srs_enabled, topic_id: None })
    }

    /// Generate one question with SRS and topic scope.
    pub fn next_question_scoped(
        repo: &crate::repository::WordRepository,
        mode: TestMode,
        srs_enabled: bool,
        topic_id: Option<i64>,
    ) -> Result<Option<Question>, crate::db::DbError> {
        TestingEngine::generate_question_with_options(repo, mode, TestingOptions { srs_enabled, topic_id })
    }

    /// Submit one answer and return grading result.
    /// Wrong answers automatically increase review_count.
    pub fn submit_answer(
        repo: &crate::repository::WordRepository,
        question: &Question,
        answer: &str,
    ) -> Result<AnswerResult, crate::db::DbError> {
        TestingEngine::submit_answer(repo, question, answer)
    }

    /// Submit one answer with explicit SRS toggle.
    pub fn submit_answer_with_srs(
        repo: &crate::repository::WordRepository,
        question: &Question,
        answer: &str,
        srs_enabled: bool,
    ) -> Result<AnswerResult, crate::db::DbError> {
        TestingEngine::submit_answer_with_options(
            repo,
            question,
            answer,
            TestingOptions { srs_enabled, topic_id: None },
        )
    }
}

