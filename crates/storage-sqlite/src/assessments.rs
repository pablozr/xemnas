//! SQLite implementation of the assessment provenance port.

use application::extract::{AssessmentRecord, AssessmentStore, ExtractError};
use rusqlite::{params, OptionalExtension};

use crate::store::SqliteStore;

/// Converts a query failure into a storage error carried by the use-case error.
fn storage_error(error: rusqlite::Error) -> ExtractError {
    ExtractError::Storage(error.to_string())
}

impl AssessmentStore for SqliteStore {
    fn assessment_attempt(&self, job: Option<&str>) -> Result<Option<i64>, ExtractError> {
        self.lock()
            .query_row("SELECT attempts FROM jobs WHERE id=?1", [job], |r| r.get(0))
            .optional()
            .map_err(storage_error)
    }
    fn record_assessment(&self, row: &AssessmentRecord) -> Result<(), ExtractError> {
        self.lock()
            .execute(
                "INSERT INTO assessments \
                 (id, capture_id, job_id, profile_id, adapter, model, policy, \
                  consent_preview_hash, input_hash, started_at, finished_at, outcome, \
                  candidates, inserted, error_code, attempt, reason, durable_count, detail_count, \
                  error_detail) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
                    ?16, ?17, ?18, ?19, ?20)",
                params![
                    row.id,
                    row.capture_id,
                    row.job_id,
                    row.profile_id,
                    row.adapter,
                    row.model,
                    row.policy,
                    row.consent_preview_hash,
                    row.input_hash,
                    row.started_at,
                    row.finished_at,
                    row.outcome.as_str(),
                    row.candidates,
                    row.inserted,
                    row.error_code,
                    row.attempt,
                    row.reason,
                    row.durable_count as i64,
                    row.detail_count as i64,
                    row.error_detail,
                ],
            )
            .map(|_| ())
            .map_err(storage_error)
    }
}
