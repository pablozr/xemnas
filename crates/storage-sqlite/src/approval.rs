//! SQLite implementation of the approval policy's persistence port.

use application::auto_approval::{ApprovalError, ApprovalStore, Entry, Lane, LedgerRow, Mode};
use application::inbox::CandidateStatus;
use rusqlite::{params, OptionalExtension};

use crate::store::SqliteStore;

fn storage_error(error: rusqlite::Error) -> ApprovalError {
    ApprovalError::Storage(error.to_string())
}

fn entry_from(row: &rusqlite::Row<'_>) -> rusqlite::Result<Entry> {
    let lane: String = row.get(2)?;
    Ok(Entry {
        candidate_id: row.get(0)?,
        project_id: row.get(1)?,
        lane: Lane::parse(&lane).unwrap_or(Lane::Audit),
        created_at: row.get(3)?,
        due_at: row.get(4)?,
        resolved_at: row.get(5)?,
        decision_id: row.get(6)?,
    })
}

const ENTRY_COLUMNS: &str = "candidate_id, project_id, lane, created_at, due_at, resolved_at, \
     decision_id";

impl ApprovalStore for SqliteStore {
    fn approval_mode(&self) -> Result<Mode, ApprovalError> {
        let mode: Option<String> = self
            .lock()
            .query_row(
                "SELECT mode FROM approval_settings WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error)?;
        Ok(mode.as_deref().and_then(Mode::parse).unwrap_or_default())
    }

    fn set_approval_mode(&self, mode: Mode, at: &str) -> Result<(), ApprovalError> {
        self.lock()
            .execute(
                "INSERT INTO approval_settings (id, mode, updated_at) VALUES (1, ?1, ?2) \
                 ON CONFLICT(id) DO UPDATE SET mode = excluded.mode, \
                 updated_at = excluded.updated_at",
                params![mode.as_str(), at],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn auto_entry(&self, candidate_id: &str) -> Result<Option<Entry>, ApprovalError> {
        self.lock()
            .query_row(
                &format!("SELECT {ENTRY_COLUMNS} FROM auto_approvals WHERE candidate_id = ?1"),
                [candidate_id],
                entry_from,
            )
            .optional()
            .map_err(storage_error)
    }

    fn insert_auto_entry(&self, entry: &Entry) -> Result<(), ApprovalError> {
        self.lock()
            .execute(
                &format!(
                    "INSERT OR IGNORE INTO auto_approvals ({ENTRY_COLUMNS}) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
                ),
                params![
                    entry.candidate_id,
                    entry.project_id,
                    entry.lane.as_str(),
                    entry.created_at,
                    entry.due_at,
                    entry.resolved_at,
                    entry.decision_id,
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn due_auto_entries(&self, project_id: &str, now: &str) -> Result<Vec<Entry>, ApprovalError> {
        let connection = self.lock();
        let mut statement = connection
            .prepare(&format!(
                "SELECT {ENTRY_COLUMNS} FROM auto_approvals \
                 WHERE project_id = ?1 AND lane = 'held' AND resolved_at IS NULL \
                 AND due_at IS NOT NULL AND due_at <= ?2 ORDER BY due_at, candidate_id"
            ))
            .map_err(storage_error)?;
        let rows = statement
            .query_map(params![project_id, now], entry_from)
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        Ok(rows)
    }

    fn resolve_auto_entry(
        &self,
        candidate_id: &str,
        decision_id: Option<&str>,
        at: &str,
    ) -> Result<(), ApprovalError> {
        self.lock()
            .execute(
                "UPDATE auto_approvals SET resolved_at = ?2, decision_id = ?3 \
                 WHERE candidate_id = ?1 AND resolved_at IS NULL",
                params![candidate_id, at, decision_id],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn ledger(&self, project_id: &str, limit: usize) -> Result<Vec<LedgerRow>, ApprovalError> {
        let connection = self.lock();
        let mut statement = connection
            .prepare(
                "SELECT a.candidate_id, c.question, a.due_at, \
                        CASE WHEN a.decision_id IS NOT NULL THEN a.resolved_at END, \
                        a.decision_id \
                 FROM auto_approvals a JOIN decision_candidates c ON c.id = a.candidate_id \
                 WHERE a.project_id = ?1 AND a.lane = 'held' \
                 AND ((a.resolved_at IS NULL AND c.status = 'pending') \
                      OR a.decision_id IS NOT NULL) \
                 ORDER BY COALESCE(a.resolved_at, a.due_at) DESC, a.candidate_id \
                 LIMIT ?2",
            )
            .map_err(storage_error)?;
        let rows = statement
            .query_map(params![project_id, limit as i64], |row| {
                Ok(LedgerRow {
                    candidate_id: row.get(0)?,
                    question: row.get(1)?,
                    due_at: row.get(2)?,
                    accepted_at: row.get(3)?,
                    decision_id: row.get(4)?,
                })
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        Ok(rows)
    }

    fn auto_outcomes(
        &self,
        project_id: Option<&str>,
    ) -> Result<Vec<(Lane, CandidateStatus, bool)>, ApprovalError> {
        let connection = self.lock();
        let mut statement = connection
            .prepare(
                "SELECT a.lane, c.status, a.decision_id IS NOT NULL \
                 FROM auto_approvals a JOIN decision_candidates c ON c.id = a.candidate_id \
                 WHERE (?1 IS NULL OR a.project_id = ?1) \
                 ORDER BY a.created_at, a.candidate_id",
            )
            .map_err(storage_error)?;
        let rows = statement
            .query_map(params![project_id], |row| {
                let lane: String = row.get(0)?;
                let status: String = row.get(1)?;
                Ok((
                    Lane::parse(&lane).unwrap_or(Lane::Audit),
                    CandidateStatus::parse(&status).unwrap_or(CandidateStatus::Pending),
                    row.get::<_, bool>(2)?,
                ))
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        Ok(rows)
    }
}
