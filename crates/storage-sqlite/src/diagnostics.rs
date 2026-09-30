//! SQLite implementation of the diagnostics persistence port.
//!
//! Every query returns counts or metadata only; no artifact content, candidate
//! text or secret is ever selected.

use crate::store::SqliteStore;
use application::diagnostics::{
    AssessmentDiagnosticRow, DiagnosticsCounts, DiagnosticsError, DiagnosticsMetrics,
    DiagnosticsStore, Distribution, JobDiagnosticRow, LossMetrics, NoiseMetrics,
    ReceiptDiagnosticRow,
};
use std::collections::BTreeMap;

/// Converts a query failure into a storage error.
fn storage_error(error: rusqlite::Error) -> DiagnosticsError {
    DiagnosticsError::Storage(error.to_string())
}

/// Counts rows in a table.
fn count(connection: &rusqlite::Connection, table: &str) -> Result<i64, DiagnosticsError> {
    connection
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .map_err(storage_error)
}

/// Groups a one-column counts query into a map.
fn grouped(
    connection: &rusqlite::Connection,
    sql: &str,
) -> Result<BTreeMap<String, i64>, DiagnosticsError> {
    let mut statement = connection.prepare(sql).map_err(storage_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)?;
    Ok(rows.into_iter().collect())
}

/// Builds a percentile distribution over the integer `delta_ms` column of
/// `deltas_sql`.
///
/// `deltas_sql` must be a `SELECT … AS delta_ms` statement with no `ORDER BY`;
/// timestamps that SQLite cannot parse are already excluded by its `WHERE`. The
/// percentile is the value at `ROUND((n-1) * p)` in ascending order.
fn distribution(
    connection: &rusqlite::Connection,
    deltas_sql: &str,
) -> Result<Distribution, DiagnosticsError> {
    let samples: i64 = connection
        .query_row(&format!("SELECT COUNT(*) FROM ({deltas_sql})"), [], |row| {
            row.get(0)
        })
        .map_err(storage_error)?;
    if samples == 0 {
        return Ok(Distribution {
            samples: 0,
            p50: None,
            p95: None,
        });
    }
    let percentile = |fraction: f64| -> Result<Option<i64>, DiagnosticsError> {
        let offset = (((samples - 1) as f64) * fraction).round() as i64;
        let value: i64 = connection
            .query_row(
                &format!("SELECT delta_ms FROM ({deltas_sql}) ORDER BY delta_ms LIMIT 1 OFFSET ?1"),
                [offset],
                |row| row.get(0),
            )
            .map_err(storage_error)?;
        Ok(Some(value))
    };
    Ok(Distribution {
        samples,
        p50: percentile(0.5)?,
        p95: percentile(0.95)?,
    })
}

/// Candidate creation minus its receipt, in milliseconds, for parseable rows.
const LATENCY_SQL: &str = "SELECT CAST(strftime('%s', dc.created_at) AS INTEGER) * 1000 \
     - CAST(strftime('%s', r.received_at) AS INTEGER) * 1000 AS delta_ms \
     FROM decision_candidates dc \
     JOIN capture_receipts r ON r.capture_id = dc.capture_id \
     WHERE strftime('%s', dc.created_at) IS NOT NULL \
       AND strftime('%s', r.received_at) IS NOT NULL";

/// Decision confirmation minus its candidate creation, in milliseconds.
const REVIEW_SQL: &str = "SELECT CAST(strftime('%s', d.confirmed_at) AS INTEGER) * 1000 \
     - CAST(strftime('%s', dc.created_at) AS INTEGER) * 1000 AS delta_ms \
     FROM engineering_decisions d \
     JOIN decision_candidates dc ON dc.id = d.candidate_id \
     WHERE strftime('%s', d.confirmed_at) IS NOT NULL \
       AND strftime('%s', dc.created_at) IS NOT NULL";

impl DiagnosticsStore for SqliteStore {
    fn migrations_version(&self) -> Result<i64, DiagnosticsError> {
        self.lock()
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
                [],
                |row| row.get(0),
            )
            .map_err(storage_error)
    }

    fn counts(&self) -> Result<DiagnosticsCounts, DiagnosticsError> {
        let connection = self.lock();
        Ok(DiagnosticsCounts {
            projects: count(&connection, "projects")?,
            captures: count(&connection, "capture_receipts")?,
            artifacts: count(&connection, "capture_artifacts")?,
            candidates_by_status: grouped(
                &connection,
                "SELECT status, COUNT(*) FROM decision_candidates GROUP BY status",
            )?,
            decisions: count(&connection, "engineering_decisions")?,
            revisions: count(&connection, "decision_revisions")?,
            assessments_by_outcome: grouped(
                &connection,
                "SELECT outcome, COUNT(*) FROM assessments GROUP BY outcome",
            )?,
            jobs_by_state: grouped(
                &connection,
                "SELECT state, COUNT(*) FROM jobs GROUP BY state",
            )?,
        })
    }

    fn recent_jobs(&self, limit: usize) -> Result<Vec<JobDiagnosticRow>, DiagnosticsError> {
        let connection = self.lock();
        let mut statement = connection
            .prepare(
                "SELECT id, kind, state, attempts, updated_at, last_error FROM jobs \
                 ORDER BY created_at DESC, id DESC LIMIT ?1",
            )
            .map_err(storage_error)?;
        let rows = statement
            .query_map([limit as i64], |row| {
                Ok(JobDiagnosticRow {
                    id: row.get(0)?,
                    kind: row.get(1)?,
                    state: row.get(2)?,
                    attempts: row.get(3)?,
                    updated_at: row.get(4)?,
                    last_error: row.get(5)?,
                })
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        Ok(rows)
    }

    fn recent_receipts(&self, limit: usize) -> Result<Vec<ReceiptDiagnosticRow>, DiagnosticsError> {
        let connection = self.lock();
        // Resolve the project id from the receipt location; the path itself is
        // never selected, so it cannot leak into the document.
        let mut statement = connection
            .prepare(
                "SELECT r.capture_id, r.artifact_count, r.received_at, p.id \
                 FROM capture_receipts r \
                 LEFT JOIN projects p ON p.location = r.canonical_path \
                 ORDER BY r.received_at DESC, r.capture_id DESC LIMIT ?1",
            )
            .map_err(storage_error)?;
        let rows = statement
            .query_map([limit as i64], |row| {
                Ok(ReceiptDiagnosticRow {
                    capture_id: row.get(0)?,
                    artifact_count: row.get(1)?,
                    received_at: row.get(2)?,
                    project_id: row.get(3)?,
                })
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        Ok(rows)
    }

    fn recent_assessments(
        &self,
        limit: usize,
    ) -> Result<Vec<AssessmentDiagnosticRow>, DiagnosticsError> {
        let connection = self.lock();
        let mut statement = connection
            .prepare(
                "SELECT outcome, error_code FROM assessments \
                 ORDER BY finished_at DESC, id DESC LIMIT ?1",
            )
            .map_err(storage_error)?;
        let rows = statement
            .query_map([limit as i64], |row| {
                Ok(AssessmentDiagnosticRow {
                    outcome: row.get(0)?,
                    error_code: row.get(1)?,
                })
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        Ok(rows)
    }

    fn metrics(&self) -> Result<DiagnosticsMetrics, DiagnosticsError> {
        let connection = self.lock();

        let latency = distribution(&connection, LATENCY_SQL)?;
        let review = distribution(&connection, REVIEW_SQL)?;

        let decided = grouped(
            &connection,
            "SELECT status, COUNT(*) FROM decision_candidates \
             WHERE status IN ('dismissed', 'accepted', 'edited_and_accepted') GROUP BY status",
        )?;
        let decided_total: i64 = decided.values().sum();
        let dismissed = decided.get("dismissed").copied().unwrap_or(0);
        let dismissed_ratio = if decided_total > 0 {
            Some(round4(dismissed as f64 / decided_total as f64))
        } else {
            None
        };

        let (assessments_failed, assessments_skipped, jobs_failed): (i64, i64, i64) = connection
            .query_row(
                "SELECT \
                 (SELECT COUNT(*) FROM assessments WHERE outcome = 'failed'), \
                 (SELECT COUNT(*) FROM assessments WHERE outcome = 'skipped'), \
                 (SELECT COUNT(*) FROM jobs WHERE state = 'failed')",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(storage_error)?;

        Ok(DiagnosticsMetrics {
            latency_capture_to_candidate_ms: latency,
            review_time_ms: review,
            noise: NoiseMetrics {
                decided_total,
                dismissed_ratio,
            },
            losses: LossMetrics {
                assessments_failed,
                assessments_skipped,
                jobs_failed,
                outbox_rejected: 0,
            },
        })
    }
}

/// Rounds a ratio to four decimals, as promised by the document shape.
fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}
