//! SQLite implementation of the automatic review's persistence port.

use std::collections::BTreeSet;

use application::auto_approval::{
    ApprovalStore, By, Conflict, ConflictKind, Entry, ItemKind, Mode, ReviewError, Verdict,
};
use rusqlite::{params, OptionalExtension};

use crate::store::SqliteStore;

fn storage_error(error: rusqlite::Error) -> ReviewError {
    ReviewError::Storage(error.to_string())
}

impl ApprovalStore for SqliteStore {
    fn approval_mode(&self) -> Result<Mode, ReviewError> {
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

    fn set_approval_mode(&self, mode: Mode, at: &str) -> Result<(), ReviewError> {
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

    fn reviewed(&self, project_id: &str) -> Result<BTreeSet<(ItemKind, String)>, ReviewError> {
        let connection = self.lock();
        let mut statement = connection
            .prepare(
                "SELECT item_kind, item_id FROM auto_reviews \
                 WHERE project_id = ?1 AND undone_at IS NULL",
            )
            .map_err(storage_error)?;
        let rows = statement
            .query_map([project_id], |row| {
                let kind: String = row.get(0)?;
                let id: String = row.get(1)?;
                Ok((kind, id))
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        Ok(rows
            .into_iter()
            .filter_map(|(kind, id)| ItemKind::parse(&kind).map(|kind| (kind, id)))
            .collect())
    }

    fn record_review(&self, entry: &Entry) -> Result<(), ReviewError> {
        self.lock()
            .execute(
                "INSERT OR REPLACE INTO auto_reviews \
                 (item_kind, item_id, project_id, verdict, decided_by, reason, title, \
                  result_id, created_at, undone_at, conflicts_kind, conflicts_id) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    entry.kind.as_str(),
                    entry.item_id,
                    entry.project_id,
                    entry.verdict.as_str(),
                    entry.by.as_str(),
                    entry.reason,
                    entry.title,
                    entry.result_id,
                    entry.created_at,
                    entry.undone_at,
                    entry.conflicts_with.as_ref().map(|side| side.kind.as_str()),
                    entry.conflicts_with.as_ref().map(|side| side.id.as_str()),
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn review_ledger(&self, project_id: &str, limit: usize) -> Result<Vec<Entry>, ReviewError> {
        let connection = self.lock();
        let mut statement = connection
            .prepare(
                "SELECT item_kind, item_id, project_id, verdict, decided_by, reason, title, \
                        result_id, created_at, undone_at, conflicts_kind, conflicts_id \
                 FROM auto_reviews WHERE project_id = ?1 \
                 ORDER BY created_at DESC, item_id LIMIT ?2",
            )
            .map_err(storage_error)?;
        let rows = statement
            .query_map(params![project_id, limit as i64], |row| {
                let kind: String = row.get(0)?;
                let verdict: String = row.get(3)?;
                let by: String = row.get(4)?;
                let conflicts_kind: Option<String> = row.get(10)?;
                let conflicts_id: Option<String> = row.get(11)?;
                let conflicts_with = conflicts_kind
                    .as_deref()
                    .and_then(ConflictKind::parse)
                    .zip(conflicts_id)
                    .map(|(kind, id)| Conflict { kind, id });
                Ok(Entry {
                    kind: ItemKind::parse(&kind).unwrap_or(ItemKind::Candidate),
                    item_id: row.get(1)?,
                    project_id: row.get(2)?,
                    verdict: Verdict::parse(&verdict).unwrap_or(Verdict::NeedsHuman),
                    by: By::parse(&by).unwrap_or(By::Rules),
                    reason: row.get(5)?,
                    title: row.get(6)?,
                    result_id: row.get(7)?,
                    created_at: row.get(8)?,
                    undone_at: row.get(9)?,
                    conflicts_with,
                })
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        Ok(rows)
    }

    fn mark_undone(&self, kind: ItemKind, item_id: &str, at: &str) -> Result<(), ReviewError> {
        self.lock()
            .execute(
                "UPDATE auto_reviews SET undone_at = ?3 WHERE item_kind = ?1 AND item_id = ?2",
                params![kind.as_str(), item_id, at],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn review_calls(&self, since: &str) -> Result<Vec<(String, bool)>, ReviewError> {
        let connection = self.lock();
        let mut statement = connection
            .prepare(
                "SELECT called_at, succeeded FROM auto_review_calls \
                 WHERE called_at >= ?1 ORDER BY called_at, call_id",
            )
            .map_err(storage_error)?;
        let rows = statement
            .query_map([since], |row| Ok((row.get(0)?, row.get::<_, i64>(1)? == 1)))
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        Ok(rows)
    }

    fn record_review_call(&self, at: &str, items: usize, ok: bool) -> Result<(), ReviewError> {
        self.lock()
            .execute(
                "INSERT INTO auto_review_calls (called_at, items, succeeded) VALUES (?1, ?2, ?3)",
                params![at, items as i64, i64::from(ok)],
            )
            .map_err(storage_error)?;
        Ok(())
    }
}
