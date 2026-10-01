//! SQLite implementation of the derived-context suggestion port.

use application::claim_suggestions::{ClaimSuggestionRecord, ClaimSuggestionStore};
use application::decisions::DecisionsError;
use domain::claims::ClaimKind;
use rusqlite::{params, OptionalExtension, Row};

use crate::store::SqliteStore;

const COLUMNS: &str = "suggestion_id, project_id, decision_id, kind, statement, quote, created_at";

fn row(row: &Row<'_>) -> rusqlite::Result<(ClaimSuggestionRecord, String)> {
    Ok((
        ClaimSuggestionRecord {
            suggestion_id: row.get(0)?,
            project_id: row.get(1)?,
            decision_id: row.get(2)?,
            kind: ClaimKind::Constraint,
            statement: row.get(4)?,
            quote: row.get(5)?,
            created_at: row.get(6)?,
        },
        row.get(3)?,
    ))
}

fn typed(pair: (ClaimSuggestionRecord, String)) -> Option<ClaimSuggestionRecord> {
    let (mut record, kind) = pair;
    record.kind = ClaimKind::parse(&kind)?;
    Some(record)
}

impl ClaimSuggestionStore for SqliteStore {
    fn insert_claim_suggestion(
        &self,
        record: &ClaimSuggestionRecord,
    ) -> Result<bool, DecisionsError> {
        let changed = self
            .lock()
            .execute(
                "INSERT OR IGNORE INTO claim_suggestions \
                 (suggestion_id, project_id, decision_id, kind, statement, quote, created_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    record.suggestion_id,
                    record.project_id,
                    record.decision_id,
                    record.kind.as_str(),
                    record.statement,
                    record.quote,
                    record.created_at,
                ],
            )
            .map_err(storage_error)?;
        Ok(changed == 1)
    }

    fn pending_claim_suggestions(
        &self,
        project_id: &str,
    ) -> Result<Vec<ClaimSuggestionRecord>, DecisionsError> {
        let connection = self.lock();
        let mut statement = connection
            .prepare(&format!(
                "SELECT {COLUMNS} FROM claim_suggestions \
                 WHERE project_id = ?1 AND outcome IS NULL ORDER BY created_at, suggestion_id"
            ))
            .map_err(storage_error)?;
        let rows = statement
            .query_map([project_id], row)
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        Ok(rows.into_iter().filter_map(typed).collect())
    }

    fn pending_claim_suggestion(
        &self,
        suggestion_id: &str,
    ) -> Result<Option<ClaimSuggestionRecord>, DecisionsError> {
        Ok(self
            .lock()
            .query_row(
                &format!(
                    "SELECT {COLUMNS} FROM claim_suggestions \
                     WHERE suggestion_id = ?1 AND outcome IS NULL"
                ),
                [suggestion_id],
                row,
            )
            .optional()
            .map_err(storage_error)?
            .and_then(typed))
    }

    fn resolve_claim_suggestion(
        &self,
        suggestion_id: &str,
        outcome: &str,
        at: &str,
    ) -> Result<bool, DecisionsError> {
        let changed = self
            .lock()
            .execute(
                "UPDATE claim_suggestions SET outcome = ?2, resolved_at = ?3 \
                 WHERE suggestion_id = ?1 AND outcome IS NULL",
                params![suggestion_id, outcome, at],
            )
            .map_err(storage_error)?;
        Ok(changed == 1)
    }
}

fn storage_error(error: rusqlite::Error) -> DecisionsError {
    DecisionsError::Storage(error.to_string())
}
