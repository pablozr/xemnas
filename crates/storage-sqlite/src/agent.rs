//! SQLite implementation of the agent access port.

use application::agent_access::{AgentAccessError, AgentStore};

use crate::store::SqliteStore;

impl AgentStore for SqliteStore {
    fn project_decision_ids(&self, project_id: &str) -> Result<Vec<String>, AgentAccessError> {
        let connection = self.lock();
        let mut statement = connection
            .prepare("SELECT decision_id FROM engineering_decisions WHERE project_id = ?1")
            .map_err(storage_error)?;
        let ids = statement
            .query_map([project_id], |row| row.get(0))
            .map_err(storage_error)?
            .collect::<Result<Vec<String>, _>>()
            .map_err(storage_error)?;
        Ok(ids)
    }
}

fn storage_error(error: rusqlite::Error) -> AgentAccessError {
    AgentAccessError::Storage(error.to_string())
}
