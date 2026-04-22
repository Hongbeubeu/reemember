use crate::repository::{WordRepository, QueryOptions};
use crate::db::DbError;
use serde_json;

pub struct ExportService;

impl ExportService {
    pub fn export_to_json_string(repo: &WordRepository) -> Result<String, DbError> {
        let records = repo.query(&QueryOptions::new())?;
        let json =
            serde_json::to_string_pretty(&records).map_err(|e| DbError::Validation(e.to_string()))?;
        Ok(json)
    }
}
