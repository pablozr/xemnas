//! SQLite implementation of the extraction persistence port.
//!
//! `load_evidence` reads the receipt, its project and the capture artifacts in a
//! deterministic order (by `artifact_id`) with defensive bounds;
//! `insert_candidates` writes every candidate in one transaction and relies on
//! the `dedup_hash` unique constraint so reprocessing never duplicates.

use application::extract::{
    truncate_content, DecisionCandidateRecord, DecisionEvidence, EvidenceArtifact, ExtractError,
    ExtractionStore, MAX_EVIDENCE_ARTIFACTS, MAX_EVIDENCE_CONTENT_BYTES,
};
use rusqlite::{params, OptionalExtension};

use crate::store::SqliteStore;

/// Converts a query failure into a storage error carried by the use-case error.
fn storage_error(error: rusqlite::Error) -> ExtractError {
    ExtractError::Storage(error.to_string())
}

impl ExtractionStore for SqliteStore {
    fn load_evidence(&self, capture_id: &str) -> Result<Option<DecisionEvidence>, ExtractError> {
        let connection = self.lock();

        let receipt = connection
            .query_row(
                "SELECT canonical_path, received_at FROM capture_receipts WHERE capture_id = ?1",
                [capture_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(storage_error)?;
        let Some((canonical_path, received_at)) = receipt else {
            return Ok(None);
        };

        let project_id = connection
            .query_row(
                "SELECT id FROM projects WHERE location = ?1",
                [&canonical_path],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(storage_error)?;
        let Some(project_id) = project_id else {
            return Ok(None);
        };

        let checkpoint = connection
            .query_row(
                "SELECT adapter, session_id, observed_at FROM adapter_checkpoints \
                 WHERE capture_id = ?1 ORDER BY updated_at DESC LIMIT 1",
                [capture_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()
            .map_err(storage_error)?;

        let mut statement = connection
            .prepare(
                "SELECT artifact_id, kind, content, metadata FROM capture_artifacts \
                 WHERE capture_id = ?1 ORDER BY artifact_id LIMIT ?2",
            )
            .map_err(storage_error)?;
        let artifacts = statement
            .query_map(params![capture_id, MAX_EVIDENCE_ARTIFACTS as i64], |row| {
                let content: String = row.get(2)?;
                Ok(EvidenceArtifact {
                    artifact_id: row.get(0)?,
                    kind: row.get(1)?,
                    content: truncate_content(&content, MAX_EVIDENCE_CONTENT_BYTES),
                    metadata: row.get(3)?,
                })
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;

        let (adapter, session_id, observed_at) = match checkpoint {
            Some((adapter, session_id, observed_at)) => {
                (Some(adapter), Some(session_id), Some(observed_at))
            }
            None => (None, None, Some(received_at)),
        };

        Ok(Some(DecisionEvidence {
            capture_id: capture_id.to_string(),
            project_id,
            adapter,
            session_id,
            observed_at,
            artifacts,
        }))
    }

    fn insert_candidates(
        &self,
        records: &[DecisionCandidateRecord],
    ) -> Result<usize, ExtractError> {
        let mut connection = self.lock();
        let transaction = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;

        let mut inserted = 0usize;
        for record in records {
            let changed = transaction
                .execute(
                    "INSERT OR IGNORE INTO decision_candidates \
                     (id, project_id, capture_id, status, question, choice, rationale, signals, \
                      confidence, confidence_reason, evidence_refs, diff_summary, dedup_hash, \
                      created_at, updated_at) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                    params![
                        record.id,
                        record.project_id,
                        record.capture_id,
                        record.status,
                        record.question,
                        record.choice,
                        record.rationale,
                        record.signals,
                        record.confidence,
                        record.confidence_reason,
                        record.evidence_refs,
                        record.diff_summary,
                        record.dedup_hash,
                        record.created_at,
                        record.updated_at,
                    ],
                )
                .map_err(storage_error)?;
            inserted += changed;
        }

        transaction.commit().map_err(storage_error)?;
        Ok(inserted)
    }
}
