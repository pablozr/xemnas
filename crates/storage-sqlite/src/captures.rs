//! SQLite implementation of the Capture persistence port.

use application::captures::{CaptureError, CaptureReceiptRecord, CaptureRepository, CaptureWrite};
use rusqlite::{params, OptionalExtension, Row};

use crate::store::SqliteStore;

/// Column list shared by receipt queries, in record order.
const RECEIPT_COLUMNS: &str =
    "capture_id, idempotency_key, canonical_path, received_at, artifact_count";

/// Converts a query failure into a storage error carried by the use-case error.
fn storage_error(error: rusqlite::Error) -> CaptureError {
    CaptureError::Storage(error.to_string())
}

/// Maps a receipt row into a [`CaptureReceiptRecord`].
fn map_receipt(row: &Row<'_>) -> rusqlite::Result<CaptureReceiptRecord> {
    Ok(CaptureReceiptRecord {
        capture_id: row.get(0)?,
        idempotency_key: row.get(1)?,
        canonical_path: row.get(2)?,
        received_at: row.get(3)?,
        artifact_count: row.get(4)?,
    })
}

/// Classifies a constraint failure into the capture error it represents.
fn constraint_kind(error: &rusqlite::Error) -> Option<CaptureError> {
    if let rusqlite::Error::SqliteFailure(failure, message) = error {
        if failure.code != rusqlite::ErrorCode::ConstraintViolation {
            return None;
        }
        let text = message.as_deref().unwrap_or_default();
        if text.contains("capture_receipts.idempotency_key") {
            return Some(CaptureError::DuplicateIdempotencyKey);
        }
        if text.contains("capture_artifacts") {
            return Some(CaptureError::DuplicateArtifact);
        }
    }
    None
}

impl CaptureRepository for SqliteStore {
    fn insert_capture(&self, write: &CaptureWrite) -> Result<(), CaptureError> {
        self.write_capture_episode(write, None)
    }

    fn insert_capture_with_provenance(
        &self,
        write: &CaptureWrite,
        provenance: &application::capture_episode::CaptureProvenance,
    ) -> Result<(), CaptureError> {
        self.write_capture_episode(write, Some(provenance))
    }

    fn find_receipt(&self, capture_id: &str) -> Result<Option<CaptureReceiptRecord>, CaptureError> {
        self.lock()
            .query_row(
                &format!("SELECT {RECEIPT_COLUMNS} FROM capture_receipts WHERE capture_id = ?1"),
                [capture_id],
                map_receipt,
            )
            .optional()
            .map_err(storage_error)
    }

    fn find_receipt_by_idempotency_key(
        &self,
        idempotency_key: &str,
    ) -> Result<Option<CaptureReceiptRecord>, CaptureError> {
        self.lock()
            .query_row(
                &format!(
                    "SELECT {RECEIPT_COLUMNS} FROM capture_receipts WHERE idempotency_key = ?1"
                ),
                [idempotency_key],
                map_receipt,
            )
            .optional()
            .map_err(storage_error)
    }
}

impl SqliteStore {
    fn write_capture_episode(
        &self,
        write: &CaptureWrite,
        provenance: Option<&application::capture_episode::CaptureProvenance>,
    ) -> Result<(), CaptureError> {
        let mut connection = self.lock();
        let transaction = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;

        let registered: i64 = transaction
            .query_row(
                "SELECT COUNT(*) FROM projects WHERE location = ?1",
                [&write.receipt.canonical_path],
                |row| row.get(0),
            )
            .map_err(storage_error)?;
        if registered == 0 {
            return Err(CaptureError::ProjectNotRegistered);
        }

        transaction
            .execute(
                "INSERT INTO capture_receipts \
                 (capture_id, idempotency_key, canonical_path, received_at, artifact_count) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    write.receipt.capture_id,
                    write.receipt.idempotency_key,
                    write.receipt.canonical_path,
                    write.receipt.received_at,
                    write.receipt.artifact_count,
                ],
            )
            .map_err(|error| constraint_kind(&error).unwrap_or_else(|| storage_error(error)))?;

        if let Some(provenance) = provenance {
            let json = serde_json::to_string(provenance)
                .map_err(|_| CaptureError::Storage("invalid capture provenance".into()))?;
            transaction
                .execute(
                    "INSERT INTO capture_episode_sources(capture_id,provenance) VALUES (?1,?2)",
                    params![write.receipt.capture_id, json],
                )
                .map_err(storage_error)?;
        }
        for artifact in &write.artifacts {
            transaction
                .execute(
                    "INSERT INTO capture_artifacts \
                     (capture_id, artifact_id, kind, content, metadata, fingerprint) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        artifact.capture_id,
                        artifact.artifact_id,
                        artifact.kind,
                        artifact.content,
                        artifact.metadata,
                        artifact.fingerprint,
                    ],
                )
                .map_err(|error| constraint_kind(&error).unwrap_or_else(|| storage_error(error)))?;
        }

        transaction
            .execute(
                "INSERT INTO jobs \
                 (id, kind, payload, state, idempotent, attempts, last_error, created_at, updated_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    write.job.id,
                    write.job.kind,
                    write.job.payload,
                    write.job.state.as_str(),
                    write.job.idempotent,
                    write.job.attempts,
                    write.job.last_error,
                    write.job.created_at,
                    write.job.updated_at,
                ],
            )
            .map_err(storage_error)?;

        transaction
            .execute(
                "INSERT INTO adapter_checkpoints \
                 (adapter, session_id, message_id, capture_id, observed_at, updated_at, \
                  adapter_version) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) \
                 ON CONFLICT(adapter, session_id) DO UPDATE SET \
                     message_id = excluded.message_id, \
                     adapter_version = excluded.adapter_version, \
                     capture_id = excluded.capture_id, \
                     observed_at = excluded.observed_at, \
                     updated_at = excluded.updated_at",
                params![
                    write.checkpoint.adapter,
                    write.checkpoint.session_id,
                    write.checkpoint.message_id,
                    write.checkpoint.capture_id,
                    write.checkpoint.observed_at,
                    write.checkpoint.updated_at,
                    write.checkpoint.adapter_version,
                ],
            )
            .map_err(storage_error)?;

        let project: String = transaction
            .query_row(
                "SELECT id FROM projects WHERE location=?1",
                [&write.receipt.canonical_path],
                |row| row.get(0),
            )
            .map_err(storage_error)?;
        crate::observations::schedule(
            &transaction,
            &project,
            "capture",
            &write.receipt.received_at,
            true,
        )
        .map_err(storage_error)?;
        transaction.commit().map_err(storage_error)?;
        Ok(())
    }
}
