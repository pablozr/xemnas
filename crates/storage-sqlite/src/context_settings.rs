//! SQLite implementation of the context settings port.

use application::context::ContextError;
use application::context_settings::{ContextMode, ContextSettingsStore, ProjectContextSettings};
use rusqlite::{params, OptionalExtension};

use crate::store::SqliteStore;

impl ContextSettingsStore for SqliteStore {
    fn context_settings(
        &self,
        project_id: &str,
    ) -> Result<Option<ProjectContextSettings>, ContextError> {
        self.lock()
            .query_row(
                "SELECT mode, budget_tokens, updated_at FROM project_context_settings \
                 WHERE project_id = ?1",
                [project_id],
                |row| {
                    let mode: String = row.get(0)?;
                    let budget: Option<i64> = row.get(1)?;
                    Ok(ProjectContextSettings {
                        project_id: project_id.to_string(),
                        mode: ContextMode::parse(&mode).unwrap_or_default(),
                        budget_tokens: budget.and_then(|value| usize::try_from(value).ok()),
                        updated_at: Some(row.get(2)?),
                    })
                },
            )
            .optional()
            .map_err(storage_error)
    }

    fn save_context_settings(&self, settings: &ProjectContextSettings) -> Result<(), ContextError> {
        self.lock()
            .execute(
                "INSERT INTO project_context_settings (project_id, mode, budget_tokens, updated_at) \
                 VALUES (?1, ?2, ?3, ?4) \
                 ON CONFLICT(project_id) DO UPDATE SET mode = excluded.mode, \
                     budget_tokens = excluded.budget_tokens, updated_at = excluded.updated_at",
                params![
                    settings.project_id,
                    settings.mode.as_str(),
                    settings.budget_tokens.map(|budget| budget as i64),
                    settings.updated_at,
                ],
            )
            .map(|_| ())
            .map_err(storage_error)
    }
}

fn storage_error(error: rusqlite::Error) -> ContextError {
    ContextError::Storage(error.to_string())
}
