//! SQLite implementation of the relation suggestion port.

use application::decisions::DecisionsError;
use application::relation_suggestions::{RelationSuggestionRecord, RelationSuggestionStore};
use domain::relations::RelationKind;
use rusqlite::{params, OptionalExtension, Row};

use crate::store::SqliteStore;

const COLUMNS: &str = "suggestion_id, project_id, from_id, to_id, kind, quote, reason, created_at";

fn row(row: &Row<'_>) -> rusqlite::Result<(RelationSuggestionRecord, String)> {
    let kind: String = row.get(4)?;
    Ok((
        RelationSuggestionRecord {
            suggestion_id: row.get(0)?,
            project_id: row.get(1)?,
            from_id: row.get(2)?,
            to_id: row.get(3)?,
            kind: RelationKind::DependsOn,
            quote: row.get(5)?,
            reason: row.get(6)?,
            created_at: row.get(7)?,
        },
        kind,
    ))
}

fn typed(pair: (RelationSuggestionRecord, String)) -> Option<RelationSuggestionRecord> {
    let (mut record, kind) = pair;
    record.kind = RelationKind::parse(&kind)?;
    Some(record)
}

impl RelationSuggestionStore for SqliteStore {
    fn insert_relation_suggestion(
        &self,
        record: &RelationSuggestionRecord,
    ) -> Result<bool, DecisionsError> {
        let changed = self
            .lock()
            .execute(
                "INSERT OR IGNORE INTO relation_suggestions \
                 (suggestion_id, project_id, from_id, to_id, kind, quote, reason, created_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    record.suggestion_id,
                    record.project_id,
                    record.from_id,
                    record.to_id,
                    record.kind.as_str(),
                    record.quote,
                    record.reason,
                    record.created_at,
                ],
            )
            .map_err(storage_error)?;
        Ok(changed == 1)
    }

    fn pending_relation_suggestions(
        &self,
        project_id: &str,
    ) -> Result<Vec<RelationSuggestionRecord>, DecisionsError> {
        let connection = self.lock();
        let mut statement = connection
            .prepare(&format!(
                "SELECT {COLUMNS} FROM relation_suggestions \
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

    fn pending_relation_suggestion(
        &self,
        suggestion_id: &str,
    ) -> Result<Option<RelationSuggestionRecord>, DecisionsError> {
        Ok(self
            .lock()
            .query_row(
                &format!(
                    "SELECT {COLUMNS} FROM relation_suggestions \
                     WHERE suggestion_id = ?1 AND outcome IS NULL"
                ),
                [suggestion_id],
                row,
            )
            .optional()
            .map_err(storage_error)?
            .and_then(typed))
    }

    fn resolve_relation_suggestion(
        &self,
        suggestion_id: &str,
        outcome: &str,
        at: &str,
    ) -> Result<Option<RelationSuggestionRecord>, DecisionsError> {
        let record = self.pending_relation_suggestion(suggestion_id)?;
        if record.is_none() {
            return Ok(None);
        }
        let changed = self
            .lock()
            .execute(
                "UPDATE relation_suggestions SET outcome = ?2, resolved_at = ?3 \
                 WHERE suggestion_id = ?1 AND outcome IS NULL",
                params![suggestion_id, outcome, at],
            )
            .map_err(storage_error)?;
        Ok(if changed == 1 { record } else { None })
    }
}

fn storage_error(error: rusqlite::Error) -> DecisionsError {
    DecisionsError::Storage(error.to_string())
}
