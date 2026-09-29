//! SQLite implementation of the Job persistence port.
//!
//! All state changes use compare-and-set SQL, so a claim or a transition can
//! never resurrect a job from the wrong state. `recover_interrupted` runs the
//! interrupted idempotent/non-idempotent split in a single transaction.

use application::jobs::{
    JobError, JobRecord, JobRepository, JobState, RecoveryReport, INTERRUPTED_NON_IDEMPOTENT,
};
use rusqlite::{params, params_from_iter, OptionalExtension, Row};

use crate::store::SqliteStore;

/// Column list shared by every `JobRecord` query, in record order.
const JOB_COLUMNS: &str =
    "id, kind, payload, state, idempotent, attempts, last_error, created_at, updated_at";

/// Converts a query failure into a storage error carried by the use-case error.
fn storage_error(error: rusqlite::Error) -> JobError {
    JobError::Storage(error.to_string())
}

/// Maps a jobs row into a [`JobRecord`].
fn map_row(row: &Row<'_>) -> rusqlite::Result<JobRecord> {
    let state_text: String = row.get(3)?;
    let state = JobState::from_str(&state_text).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, Box::new(error))
    })?;
    Ok(JobRecord {
        id: row.get(0)?,
        kind: row.get(1)?,
        payload: row.get(2)?,
        state,
        idempotent: row.get::<_, i64>(4)? != 0,
        attempts: row.get(5)?,
        last_error: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

/// `updated_at` expression used by every write.
const TOUCH_UPDATED_AT: &str = "strftime('%Y-%m-%dT%H:%M:%SZ', 'now')";

impl JobRepository for SqliteStore {
    fn insert(&self, record: &JobRecord) -> Result<(), JobError> {
        self.lock()
            .execute(
                "INSERT INTO jobs \
                 (id, kind, payload, state, idempotent, attempts, last_error, created_at, updated_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    record.id,
                    record.kind,
                    record.payload,
                    record.state.as_str(),
                    record.idempotent,
                    record.attempts,
                    record.last_error,
                    record.created_at,
                    record.updated_at,
                ],
            )
            .map(|_| ())
            .map_err(storage_error)
    }

    fn get(&self, id: &str) -> Result<Option<JobRecord>, JobError> {
        self.lock()
            .query_row(
                &format!("SELECT {JOB_COLUMNS} FROM jobs WHERE id = ?1"),
                [id],
                map_row,
            )
            .optional()
            .map_err(storage_error)
    }

    fn list(&self) -> Result<Vec<JobRecord>, JobError> {
        let connection = self.lock();
        let mut statement = connection
            .prepare(&format!(
                "SELECT {JOB_COLUMNS} FROM jobs ORDER BY created_at DESC, id DESC"
            ))
            .map_err(storage_error)?;
        let records = statement
            .query_map([], map_row)
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        Ok(records)
    }

    fn claim_next(&self, registered_kinds: &[String]) -> Result<Option<JobRecord>, JobError> {
        if registered_kinds.is_empty() {
            return Ok(None);
        }
        let placeholders = vec!["?"; registered_kinds.len()].join(", ");
        let sql = format!(
            "UPDATE jobs \
             SET state = 'running', attempts = attempts + 1, updated_at = {TOUCH_UPDATED_AT} \
             WHERE id = ( \
                 SELECT id FROM jobs \
                 WHERE state = 'queued' AND kind IN ({placeholders}) \
                 ORDER BY created_at ASC, id ASC LIMIT 1 \
             ) \
             RETURNING {JOB_COLUMNS}"
        );
        self.lock()
            .query_row(&sql, params_from_iter(registered_kinds.iter()), map_row)
            .optional()
            .map_err(storage_error)
    }

    fn transition(
        &self,
        id: &str,
        from: JobState,
        to: JobState,
        last_error: Option<&str>,
    ) -> Result<bool, JobError> {
        let changed = self
            .lock()
            .execute(
                &format!(
                    "UPDATE jobs SET state = ?1, last_error = ?2, updated_at = {TOUCH_UPDATED_AT} \
                     WHERE id = ?3 AND state = ?4"
                ),
                params![to.as_str(), last_error, id, from.as_str()],
            )
            .map_err(storage_error)?;
        Ok(changed > 0)
    }

    fn recover_interrupted(&self) -> Result<RecoveryReport, JobError> {
        let mut connection = self.lock();
        let transaction = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;

        let requeued = transaction
            .execute(
                &format!(
                    "UPDATE jobs SET state = 'queued', last_error = NULL, updated_at = {TOUCH_UPDATED_AT} \
                     WHERE state = 'running' AND idempotent = 1"
                ),
                [],
            )
            .map_err(storage_error)?;

        let failed = transaction
            .execute(
                &format!(
                    "UPDATE jobs SET state = 'failed', last_error = ?1, updated_at = {TOUCH_UPDATED_AT} \
                     WHERE state = 'running' AND idempotent = 0"
                ),
                [INTERRUPTED_NON_IDEMPOTENT],
            )
            .map_err(storage_error)?;

        transaction.commit().map_err(storage_error)?;
        Ok(RecoveryReport { requeued, failed })
    }
}
