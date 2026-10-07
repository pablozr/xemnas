//! SQLite implementation of the project documentation port.

use application::documents::{DocumentError, DocumentKind, DocumentStore, ProjectDocument};
use rusqlite::params;

use crate::store::SqliteStore;

impl DocumentStore for SqliteStore {
    fn project_documents(&self, project_id: &str) -> Result<Vec<ProjectDocument>, DocumentError> {
        let connection = self.lock();
        let mut statement = connection
            .prepare(
                "SELECT path, kind, title, headings, excerpt, bytes, fingerprint, indexed_at, source \
                 FROM project_documents WHERE project_id = ?1 ORDER BY path",
            )
            .map_err(storage_error)?;
        let rows = statement
            .query_map([project_id], |row| {
                let kind: String = row.get(1)?;
                let headings: String = row.get(3)?;
                let bytes: i64 = row.get(5)?;
                Ok(ProjectDocument {
                    project_id: project_id.to_string(),
                    path: row.get(0)?,
                    kind: DocumentKind::parse(&kind).unwrap_or(DocumentKind::Guide),
                    title: row.get(2)?,
                    headings: headings
                        .split('\n')
                        .filter(|heading| !heading.is_empty())
                        .map(str::to_string)
                        .collect(),
                    excerpt: row.get(4)?,
                    bytes: u64::try_from(bytes).unwrap_or_default(),
                    fingerprint: row.get(6)?,
                    indexed_at: row.get(7)?,
                    source: row.get(8)?,
                })
            })
            .map_err(storage_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(storage_error)
    }

    fn replace_documents(
        &self,
        project_id: &str,
        documents: &[ProjectDocument],
    ) -> Result<(), DocumentError> {
        let mut connection = self.lock();
        let transaction = connection.transaction().map_err(storage_error)?;
        transaction
            .execute(
                "DELETE FROM project_documents WHERE project_id = ?1",
                [project_id],
            )
            .map_err(storage_error)?;
        {
            let mut insert = transaction
                .prepare(
                    "INSERT INTO project_documents (project_id, path, kind, title, headings, \
                     excerpt, bytes, fingerprint, indexed_at, source) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                )
                .map_err(storage_error)?;
            for document in documents {
                insert
                    .execute(params![
                        project_id,
                        document.path,
                        document.kind.as_str(),
                        document.title,
                        document.headings.join("\n"),
                        document.excerpt,
                        i64::try_from(document.bytes).unwrap_or(i64::MAX),
                        document.fingerprint,
                        document.indexed_at,
                        document.source,
                    ])
                    .map_err(storage_error)?;
            }
        }
        transaction.commit().map_err(storage_error)
    }

    fn requeue_failed_analysis(
        &self,
        idempotency_key: &str,
        max_attempts: Option<u32>,
    ) -> Result<bool, DocumentError> {
        let changed = self
            .lock()
            .execute(
                "UPDATE jobs SET state = 'queued', last_error = NULL, \
                 updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') \
                 WHERE kind = 'analyze_document' AND state = 'failed' \
                 AND payload = (SELECT capture_id FROM capture_receipts WHERE idempotency_key = ?1) \
                 AND (?2 IS NULL OR attempts < ?2)",
                params![idempotency_key, max_attempts],
            )
            .map_err(storage_error)?;
        Ok(changed > 0)
    }
}

fn storage_error(error: rusqlite::Error) -> DocumentError {
    DocumentError::Storage(error.to_string())
}
