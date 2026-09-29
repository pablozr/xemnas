//! SQLite implementation of the Project persistence port.
//!
//! The schema is created by the embedded migrations in [`crate::store`]. The
//! `location` column is `UNIQUE`: it is the single source of truth for the
//! "already registered" rule, which keeps the check correct across restarts and
//! shared store clones.

use application::projects::{ProjectError, ProjectRecord, ProjectRepository};
use rusqlite::{params, OptionalExtension};

use crate::store::SqliteStore;

/// Converts a query failure into a storage error carried by the use-case error.
fn storage_error(error: rusqlite::Error) -> ProjectError {
    ProjectError::Storage(error.to_string())
}

/// Returns `true` when the failure is a constraint violation.
///
/// The only constraints on `projects` are the primary key and the unique
/// location, so both map to the same "already registered" outcome.
fn is_conflict(error: &rusqlite::Error) -> bool {
    matches!(
        error,
        rusqlite::Error::SqliteFailure(failure, _)
            if failure.code == rusqlite::ErrorCode::ConstraintViolation
    )
}

impl ProjectRepository for SqliteStore {
    fn insert(&self, record: &ProjectRecord) -> Result<(), ProjectError> {
        let result = self.lock().execute(
            "INSERT INTO projects (id, location, registered_at) VALUES (?1, ?2, ?3)",
            params![record.id, record.location, record.registered_at],
        );
        match result {
            Ok(_) => Ok(()),
            Err(error) if is_conflict(&error) => Err(ProjectError::AlreadyRegistered),
            Err(error) => Err(storage_error(error)),
        }
    }

    fn list(&self) -> Result<Vec<ProjectRecord>, ProjectError> {
        let connection = self.lock();
        let mut statement = connection
            .prepare("SELECT id, location, registered_at FROM projects ORDER BY registered_at, id")
            .map_err(storage_error)?;
        let records = statement
            .query_map([], |row| {
                Ok(ProjectRecord::new(row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        Ok(records)
    }

    fn get(&self, id: &str) -> Result<Option<ProjectRecord>, ProjectError> {
        self.lock()
            .query_row(
                "SELECT id, location, registered_at FROM projects WHERE id = ?1",
                [id],
                |row| Ok(ProjectRecord::new(row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(storage_error)
    }

    fn find_by_location(&self, location: &str) -> Result<Option<ProjectRecord>, ProjectError> {
        self.lock()
            .query_row(
                "SELECT id, location, registered_at FROM projects WHERE location = ?1",
                [location],
                |row| Ok(ProjectRecord::new(row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(storage_error)
    }

    fn remove(&self, id: &str) -> Result<bool, ProjectError> {
        self.lock()
            .execute("DELETE FROM projects WHERE id = ?1", [id])
            .map(|removed| removed > 0)
            .map_err(storage_error)
    }
}

#[cfg(test)]
mod tests {
    use super::SqliteStore;
    use crate::store::MIGRATIONS;
    use application::projects::ProjectRepository;

    fn temporary_directory(tag: &str) -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};

        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let directory = std::env::temp_dir().join(format!(
            "xemnas-projects-{tag}-{}-{nanos}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&directory).expect("create temporary directory");
        directory
    }

    #[test]
    fn reopening_an_already_migrated_database_is_idempotent() {
        let root = temporary_directory("migrations");
        let database = root.join("xemnas.sqlite3");
        let first = SqliteStore::open(&database).expect("first open");
        let second = SqliteStore::open(&database).expect("second open");

        let connection = second.lock();
        let versions: i64 = connection
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .expect("count migrations");
        assert_eq!(
            versions,
            MIGRATIONS.len() as i64,
            "migration must not be recorded twice"
        );
        let projects: i64 = connection
            .query_row("SELECT COUNT(*) FROM projects", [], |row| row.get(0))
            .expect("count projects");
        assert_eq!(projects, 0);
        drop(connection);
        drop(first);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn insert_rejects_duplicate_location() {
        let root = temporary_directory("duplicate");
        let store = SqliteStore::open(root.join("xemnas.sqlite3")).expect("open store");
        let record = application::projects::ProjectRecord::new(
            "id-1".to_string(),
            "C:\\work\\xemnas".to_string(),
            "2026-01-01T00:00:00Z".to_string(),
        );
        store.insert(&record).expect("first insert");
        let duplicate = application::projects::ProjectRecord::new(
            "id-2".to_string(),
            "C:\\work\\xemnas".to_string(),
            "2026-01-02T00:00:00Z".to_string(),
        );
        assert_eq!(
            store.insert(&duplicate),
            Err(application::projects::ProjectError::AlreadyRegistered)
        );
        assert_eq!(store.list().expect("list").len(), 1);
        let _ = std::fs::remove_dir_all(&root);
    }
}
