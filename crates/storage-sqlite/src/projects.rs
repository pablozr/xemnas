//! SQLite implementation of the Project persistence port.

use application::projects::{ProjectError, ProjectRecord, ProjectRepository, RemovalImpact};
use rusqlite::{params, OptionalExtension};

use crate::store::SqliteStore;

/// Converts a query failure into a storage error carried by the use-case error.
fn storage_error(error: rusqlite::Error) -> ProjectError {
    ProjectError::Storage(error.to_string())
}

/// Returns `true` when the failure is a constraint violation.
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

    fn removal_impact(&self, id: &str) -> Result<RemovalImpact, ProjectError> {
        let connection = self.lock();
        let count = |sql: &str| -> Result<i64, ProjectError> {
            connection
                .query_row(sql, [id], |row| row.get(0))
                .map_err(storage_error)
        };
        Ok(RemovalImpact {
            decisions: count("SELECT COUNT(*) FROM engineering_decisions WHERE project_id = ?1")?,
            candidates: count("SELECT COUNT(*) FROM decision_candidates WHERE project_id = ?1")?,
            claims: count("SELECT COUNT(*) FROM context_claims WHERE project_id = ?1")?,
            captures: count(
                "SELECT COUNT(*) FROM capture_receipts \
                 WHERE canonical_path = (SELECT location FROM projects WHERE id = ?1)",
            )?,
            injections: count("SELECT COUNT(*) FROM context_injections WHERE project_id = ?1")?,
            entities: count("SELECT COUNT(*) FROM entities WHERE project_id = ?1")?,
        })
    }

    fn purge(&self, id: &str) -> Result<bool, ProjectError> {
        let mut connection = self.lock();
        let transaction = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        for sql in PURGE_STATEMENTS {
            transaction.execute(sql, [id]).map_err(storage_error)?;
        }
        let removed = transaction
            .execute("DELETE FROM projects WHERE id = ?1", [id])
            .map_err(storage_error)?;
        transaction.commit().map_err(storage_error)?;
        Ok(removed > 0)
    }
}

/// Deletes a project's rows child-first; captures cascade to artifacts,
/// checkpoints, assessments and candidates.
const PURGE_STATEMENTS: &[&str] = &[
    "DELETE FROM auto_reviews WHERE project_id = ?1",
    "DELETE FROM project_overviews WHERE project_id = ?1",
    "DELETE FROM project_documents WHERE project_id = ?1",
    "DELETE FROM relation_suggestions WHERE project_id = ?1",
    "DELETE FROM claim_suggestions WHERE project_id = ?1",
    "DELETE FROM entity_edges WHERE project_id = ?1",
    "DELETE FROM entity_patterns WHERE entity_id IN \
     (SELECT entity_id FROM entities WHERE project_id = ?1)",
    "DELETE FROM entity_aliases WHERE entity_id IN \
     (SELECT entity_id FROM entities WHERE project_id = ?1)",
    "DELETE FROM entities WHERE project_id = ?1",
    "DELETE FROM context_injections WHERE project_id = ?1",
    "DELETE FROM claims_fts WHERE claim_id IN \
     (SELECT claim_id FROM context_claims WHERE project_id = ?1)",
    "DELETE FROM context_claims WHERE project_id = ?1",
    "DELETE FROM decision_relations WHERE from_decision_id IN \
     (SELECT decision_id FROM engineering_decisions WHERE project_id = ?1) \
     OR to_decision_id IN (SELECT decision_id FROM engineering_decisions WHERE project_id = ?1)",
    "DELETE FROM decisions_fts WHERE decision_id IN \
     (SELECT decision_id FROM engineering_decisions WHERE project_id = ?1)",
    "DELETE FROM decision_revisions WHERE decision_id IN \
     (SELECT decision_id FROM engineering_decisions WHERE project_id = ?1)",
    "DELETE FROM engineering_decisions WHERE project_id = ?1",
    "DELETE FROM jobs WHERE kind = 'analyze_capture' AND payload IN \
     (SELECT capture_id FROM capture_receipts \
      WHERE canonical_path = (SELECT location FROM projects WHERE id = ?1))",
    "DELETE FROM capture_receipts \
     WHERE canonical_path = (SELECT location FROM projects WHERE id = ?1)",
    "DELETE FROM decision_candidates WHERE project_id = ?1",
];

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
