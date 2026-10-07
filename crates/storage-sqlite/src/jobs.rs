//! SQLite implementation of the Job persistence port.

use application::jobs::{
    JobError, JobRecord, JobRepository, JobSettingsStore, JobState, RecoveryReport,
    INTERRUPTED_NON_IDEMPOTENT,
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
    fn cancelled_by_project_purge(&self, record: &JobRecord) -> Result<bool, JobError> {
        if !matches!(
            record.kind.as_str(),
            application::observations::refresh::REFRESH_OBSERVATIONS_KIND
                | application::context_routing::CONTEXT_ROUTING_KIND
        ) || record.state != JobState::Running
        {
            return Ok(false);
        }
        self.lock()
            .query_row(
                "SELECT NOT EXISTS (SELECT 1 FROM jobs WHERE id = ?1) \
                 AND NOT EXISTS (SELECT 1 FROM projects WHERE id = ?2)",
                params![record.id, record.payload],
                |row| row.get(0),
            )
            .map_err(storage_error)
    }

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

    fn counts(&self) -> Result<Vec<(String, JobState, usize)>, JobError> {
        let connection = self.lock();
        let mut statement = connection
            .prepare("SELECT kind, state, COUNT(*) FROM jobs GROUP BY kind, state")
            .map_err(storage_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        rows.into_iter()
            .map(|(kind, state, count)| {
                Ok((
                    kind,
                    JobState::from_str(&state)?,
                    usize::try_from(count).unwrap_or(0),
                ))
            })
            .collect()
    }

    fn claim_next(&self, registered_kinds: &[String]) -> Result<Option<JobRecord>, JobError> {
        if registered_kinds.is_empty() {
            return Ok(None);
        }
        // The slice is in priority order: the kind listed first wins, the
        // oldest job of a kind goes first.
        let placeholders = (1..=registered_kinds.len())
            .map(|position| format!("?{position}"))
            .collect::<Vec<_>>()
            .join(", ");
        let rank = (1..=registered_kinds.len())
            .map(|position| format!("WHEN ?{position} THEN {position}"))
            .collect::<Vec<_>>()
            .join(" ");
        let sql = format!(
            "UPDATE jobs \
             SET state = 'running', attempts = attempts + 1, run_after = NULL, \
                 updated_at = {TOUCH_UPDATED_AT} \
             WHERE id = ( \
                 SELECT id FROM jobs \
                 WHERE state = 'queued' AND kind IN ({placeholders}) \
                   AND (run_after IS NULL OR run_after <= {TOUCH_UPDATED_AT}) \
                 ORDER BY CASE kind {rank} END ASC, created_at ASC, id ASC LIMIT 1 \
             ) \
             RETURNING {JOB_COLUMNS}"
        );
        self.lock()
            .query_row(&sql, params_from_iter(registered_kinds.iter()), map_row)
            .optional()
            .map_err(storage_error)
    }

    fn claim_more(&self, kind: &str, limit: usize) -> Result<Vec<JobRecord>, JobError> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let sql = format!(
            "UPDATE jobs \
             SET state = 'running', attempts = attempts + 1, run_after = NULL, \
                 updated_at = {TOUCH_UPDATED_AT} \
             WHERE id IN ( \
                 SELECT id FROM jobs \
                 WHERE state = 'queued' AND kind = ?1 \
                   AND (run_after IS NULL OR run_after <= {TOUCH_UPDATED_AT}) \
                 ORDER BY created_at ASC, id ASC LIMIT ?2 \
             ) \
             RETURNING {JOB_COLUMNS}"
        );
        let limit = i64::try_from(limit).unwrap_or(i64::MAX);
        let connection = self.lock();
        let mut statement = connection.prepare(&sql).map_err(storage_error)?;
        let mut records = statement
            .query_map(params![kind, limit], map_row)
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        // RETURNING has no order guarantee.
        records.sort_by(|left, right| {
            (&left.created_at, &left.id).cmp(&(&right.created_at, &right.id))
        });
        Ok(records)
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

    fn defer(
        &self,
        id: &str,
        delay: std::time::Duration,
        last_error: &str,
    ) -> Result<bool, JobError> {
        let seconds = delay.as_secs().max(1);
        let changed = self
            .lock()
            .execute(
                &format!(
                    "UPDATE jobs SET state = 'queued', last_error = ?1, \
                     run_after = strftime('%Y-%m-%dT%H:%M:%SZ', 'now', ?2), \
                     updated_at = {TOUCH_UPDATED_AT} \
                     WHERE id = ?3 AND state = 'running'"
                ),
                params![last_error, format!("+{seconds} seconds"), id],
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

impl JobSettingsStore for SqliteStore {
    fn parallel_analyses(&self) -> Result<Option<u8>, JobError> {
        let value: Option<i64> = self
            .lock()
            .query_row(
                "SELECT parallel_analyses FROM job_settings WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error)?;
        Ok(value.and_then(|value| u8::try_from(value).ok()))
    }

    fn set_parallel_analyses(&self, value: u8) -> Result<(), JobError> {
        self.lock()
            .execute(
                &format!(
                    "INSERT INTO job_settings (id, parallel_analyses, updated_at) \
                     VALUES (1, ?1, {TOUCH_UPDATED_AT}) \
                     ON CONFLICT(id) DO UPDATE SET \
                     parallel_analyses = excluded.parallel_analyses, \
                     updated_at = excluded.updated_at"
                ),
                [i64::from(value)],
            )
            .map(|_| ())
            .map_err(|error| match error {
                rusqlite::Error::SqliteFailure(failure, _)
                    if failure.code == rusqlite::ErrorCode::ConstraintViolation =>
                {
                    JobError::InvalidSetting
                }
                other => storage_error(other),
            })
    }
}
