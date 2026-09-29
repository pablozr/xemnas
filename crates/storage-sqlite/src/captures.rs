//! SQLite implementation of the Capture persistence port.
//!
//! One transaction writes the receipt, its artifacts and the analysis job, so a
//! constraint failure (a repeated artifact id, say) leaves no partial state. The
//! `idempotency_key` unique constraint is the deduplication mechanism; the
//! transaction reports it distinctly so the use case can return the stored
//! receipt instead of writing again.

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
///
/// The message names the failed constraint, which is how the receipt unique
/// index is told apart from the artifact primary/unique indexes.
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
        let mut connection = self.lock();
        let transaction = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;

        // §7.3 steps 4-6: the allow-list check is authoritative inside the same
        // IMMEDIATE transaction that writes. A project removed after the
        // use-case pre-check cannot leave a capture for an unregistered project.
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

        // Adapter checkpoint, in the same transaction so a rejected capture
        // leaves no checkpoint behind. `ON CONFLICT` advances the session's
        // high-water mark; a replay never reaches here (the receipt unique
        // conflict rolls the whole transaction back), so a replayed older
        // message cannot move the mark backwards.
        transaction
            .execute(
                "INSERT INTO adapter_checkpoints \
                 (adapter, session_id, message_id, capture_id, observed_at, updated_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
                 ON CONFLICT(adapter, session_id) DO UPDATE SET \
                     message_id = excluded.message_id, \
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
                ],
            )
            .map_err(storage_error)?;

        transaction.commit().map_err(storage_error)?;
        Ok(())
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
