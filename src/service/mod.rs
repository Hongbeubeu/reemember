pub use crate::import::{ImportService, ImportReport, ImportStatus, ImportRowResult};
pub use crate::export::ExportService;
pub use crate::testing::{AnswerResult, Question, TestMode, TestingEngine, TestingOptions};

pub struct VocabularyService;

impl VocabularyService {
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

    pub fn export_json(repo: &crate::repository::WordRepository) -> Result<String, crate::db::DbError> {
        ExportService::export_to_json_string(repo)
    }

    pub fn next_question(
        repo: &crate::repository::WordRepository,
        mode: TestMode,
    ) -> Result<Option<Question>, crate::db::DbError> {
        TestingEngine::generate_question(repo, mode)
    }

    pub fn next_question_with_srs(
        repo: &crate::repository::WordRepository,
        mode: TestMode,
        srs_enabled: bool,
    ) -> Result<Option<Question>, crate::db::DbError> {
        TestingEngine::generate_question_with_options(repo, mode, TestingOptions { srs_enabled, topic_id: None })
    }

    pub fn next_question_scoped(
        repo: &crate::repository::WordRepository,
        mode: TestMode,
        srs_enabled: bool,
        topic_id: Option<i64>,
    ) -> Result<Option<Question>, crate::db::DbError> {
        TestingEngine::generate_question_with_options(repo, mode, TestingOptions { srs_enabled, topic_id })
    }

    pub fn submit_answer(
        repo: &crate::repository::WordRepository,
        question: &Question,
        answer: &str,
    ) -> Result<AnswerResult, crate::db::DbError> {
        TestingEngine::submit_answer(repo, question, answer)
    }

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
