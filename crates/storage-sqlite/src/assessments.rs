//! SQLite implementation of the assessment provenance port.
//!
//! One row per analysis run, carrying metadata and hashes only — never artifact
//! content or a provider response body (PRIV-001). The foreign key to
//! `capture_receipts` keeps a capture's assessments cascading with it.

use application::extract::{AssessmentRecord, AssessmentStore, ExtractError};
use rusqlite::params;

use crate::store::SqliteStore;

/// Converts a query failure into a storage error carried by the use-case error.
fn storage_error(error: rusqlite::Error) -> ExtractError {
    ExtractError::Storage(error.to_string())
}

impl AssessmentStore for SqliteStore {
    fn record_assessment(&self, row: &AssessmentRecord) -> Result<(), ExtractError> {
        self.lock()
            .execute(
                "INSERT INTO assessments \
                 (id, capture_id, job_id, profile_id, adapter, model, policy, \
                  consent_preview_hash, input_hash, started_at, finished_at, outcome, \
                  candidates, inserted, error_code) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
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
                ],
            )
            .map(|_| ())
            .map_err(storage_error)
    }
}
