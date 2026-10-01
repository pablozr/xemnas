//! SQLite implementation of the project overview port.

use application::overview::{OverviewError, OverviewStore, ProjectOverview};
use rusqlite::{params, OptionalExtension};

use crate::store::SqliteStore;

impl OverviewStore for SqliteStore {
    fn overview(&self, project_id: &str) -> Result<Option<ProjectOverview>, OverviewError> {
        let content: Option<String> = self
            .lock()
            .query_row(
                "SELECT content FROM project_overviews WHERE project_id = ?1",
                [project_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error)?;
        content
            .map(|content| ProjectOverview::from_json(&content))
            .transpose()
    }

    fn save_overview(&self, overview: &ProjectOverview) -> Result<(), OverviewError> {
        let content = overview.to_json()?;
        self.lock()
            .execute(
                "INSERT INTO project_overviews (project_id, generated_at, content)                  VALUES (?1, ?2, ?3)                  ON CONFLICT(project_id) DO UPDATE SET                  generated_at = excluded.generated_at, content = excluded.content",
                params![overview.project_id, overview.generated_at, content],
            )
            .map_err(storage_error)?;
        Ok(())
    }
}

fn storage_error(error: rusqlite::Error) -> OverviewError {
    OverviewError::Storage(error.to_string())
}
