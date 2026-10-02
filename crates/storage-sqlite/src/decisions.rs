//! SQLite implementation of the Decisions persistence port.

use application::decisions::{
    DecisionContent, DecisionQuery, DecisionRevisionRow, DecisionSearchRow, DecisionStatus,
    DecisionStore, DecisionsError, EvidenceLinkRow, StoredDecision,
};
use rusqlite::types::Value;
use rusqlite::{params, params_from_iter, OptionalExtension, Row};

use crate::store::SqliteStore;

/// Column list shared by `list` and `get`, in [`StoredDecision`] order.
pub(crate) const DECISION_COLUMNS: &str =
    "d.decision_id, d.candidate_id, d.project_id, p.location, \
     d.capture_id, d.status, d.question, d.choice, d.rationale, d.assumptions, d.reconsider_when, \
     d.scope, d.consequences, d.version, d.created_at, d.confirmed_at, d.updated_at";

/// Join that attaches the project location.
const DECISION_FROM: &str = "FROM engineering_decisions d JOIN projects p ON p.id = d.project_id";

/// Converts a query failure into a storage error carried by the use-case error.
fn storage_error(error: rusqlite::Error) -> DecisionsError {
    DecisionsError::Storage(error.to_string())
}

/// Maps a joined decision row into a [`StoredDecision`].
pub(crate) fn map_row(row: &Row<'_>) -> rusqlite::Result<StoredDecision> {
    let status_text: String = row.get(5)?;
    let status = DecisionStatus::parse(&status_text).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            5,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "estado de decisão desconhecido",
            )),
        )
    })?;
    Ok(StoredDecision {
        decision_id: row.get(0)?,
        candidate_id: row.get(1)?,
        project_id: row.get(2)?,
        project_location: row.get(3)?,
        capture_id: row.get(4)?,
        status,
        question: row.get(6)?,
        choice: row.get(7)?,
        rationale: row.get(8)?,
        assumptions: row.get(9)?,
        reconsider_when: row.get(10)?,
        scope: row.get(11)?,
        consequences: row.get(12)?,
        version: row.get(13)?,
        created_at: row.get(14)?,
        confirmed_at: row.get(15)?,
        updated_at: row.get(16)?,
    })
}

impl DecisionStore for SqliteStore {
    fn list(&self, query: &DecisionQuery) -> Result<Vec<StoredDecision>, DecisionsError> {
        if query.statuses.is_empty() {
            return Ok(Vec::new());
        }
        let connection = self.lock();
        let mut sql = format!("SELECT {DECISION_COLUMNS} {DECISION_FROM} WHERE d.status IN (");
        let mut binds: Vec<Value> = Vec::new();
        push_placeholders(&mut sql, query.statuses.len());
        for status in &query.statuses {
            binds.push(Value::Text(status.as_str().to_string()));
        }
        sql.push(')');
        if let Some(project_id) = &query.project_id {
            sql.push_str(" AND d.project_id = ?");
            binds.push(Value::Text(project_id.clone()));
        }
        if let Some(before) = &query.before {
            sql.push_str(" AND (d.confirmed_at < ? OR (d.confirmed_at = ? AND d.decision_id < ?))");
            binds.push(Value::Text(before.confirmed_at.clone()));
            binds.push(Value::Text(before.confirmed_at.clone()));
            binds.push(Value::Text(before.decision_id.clone()));
        }
        sql.push_str(" ORDER BY d.confirmed_at DESC, d.decision_id DESC LIMIT ?");
        binds.push(Value::Integer(query.limit as i64));

        let mut statement = connection.prepare(&sql).map_err(storage_error)?;
        let rows = statement
            .query_map(params_from_iter(binds), map_row)
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        Ok(rows)
    }

    fn get(&self, id: &str) -> Result<Option<StoredDecision>, DecisionsError> {
        self.lock()
            .query_row(
                &format!("SELECT {DECISION_COLUMNS} {DECISION_FROM} WHERE d.decision_id = ?1"),
                [id],
                map_row,
            )
            .optional()
            .map_err(storage_error)
    }

    fn revisions(&self, id: &str) -> Result<Vec<DecisionRevisionRow>, DecisionsError> {
        let connection = self.lock();
        let mut statement = connection
            .prepare(
                "SELECT version, created_at, question, choice, rationale, assumptions, \
                 reconsider_when, scope, consequences FROM decision_revisions \
                 WHERE decision_id = ?1 ORDER BY version DESC",
            )
            .map_err(storage_error)?;
        let rows = statement
            .query_map([id], |row| {
                Ok(DecisionRevisionRow {
                    version: row.get(0)?,
                    created_at: row.get(1)?,
                    question: row.get(2)?,
                    choice: row.get(3)?,
                    rationale: row.get(4)?,
                    assumptions: row.get(5)?,
                    reconsider_when: row.get(6)?,
                    scope: row.get(7)?,
                    consequences: row.get(8)?,
                })
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        Ok(rows)
    }

    fn evidence(&self, id: &str) -> Result<Vec<EvidenceLinkRow>, DecisionsError> {
        let connection = self.lock();
        let mut statement = connection
            .prepare(
                "SELECT e.artifact_id, a.kind, e.position \
                 FROM evidence_links e \
                 JOIN engineering_decisions d ON d.decision_id = e.decision_id \
                 LEFT JOIN capture_artifacts a \
                   ON a.artifact_id = e.artifact_id AND a.capture_id = d.capture_id \
                 WHERE e.decision_id = ?1 \
                 ORDER BY e.position ASC, e.artifact_id ASC",
            )
            .map_err(storage_error)?;
        let rows = statement
            .query_map([id], |row| {
                Ok(EvidenceLinkRow {
                    artifact_id: row.get(0)?,
                    kind: row.get(1)?,
                    position: row.get(2)?,
                })
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        Ok(rows)
    }

    fn search(
        &self,
        match_query: &str,
        project_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<DecisionSearchRow>, DecisionsError> {
        let connection = self.lock();
        let mut sql = String::from(
            "SELECT decisions_fts.decision_id, d.project_id, d.question, \
             snippet(decisions_fts, 1, '[', ']', '…', 12) \
             FROM decisions_fts JOIN engineering_decisions d \
               ON d.decision_id = decisions_fts.decision_id \
             WHERE decisions_fts MATCH ?1",
        );
        let mut binds: Vec<Value> = vec![Value::Text(match_query.to_string())];
        if let Some(project_id) = project_id {
            sql.push_str(" AND d.project_id = ?");
            binds.push(Value::Text(project_id.to_string()));
        }
        sql.push_str(" LIMIT ?");
        binds.push(Value::Integer(limit as i64));

        let mut statement = connection.prepare(&sql).map_err(storage_error)?;
        let rows = statement
            .query_map(params_from_iter(binds), |row| {
                Ok(DecisionSearchRow {
                    decision_id: row.get(0)?,
                    project_id: row.get(1)?,
                    question: row.get(2)?,
                    snippet: row.get(3)?,
                })
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        Ok(rows)
    }

    fn revise(
        &self,
        id: &str,
        content: &DecisionContent,
        version: i64,
        updated_at: &str,
    ) -> Result<bool, DecisionsError> {
        let mut connection = self.lock();
        let transaction = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;

        let changed = transaction
            .execute(
                "UPDATE engineering_decisions \
                 SET question = ?2, choice = ?3, rationale = ?4, assumptions = ?5, \
                     reconsider_when = ?6, scope = ?7, consequences = ?8, version = ?9, \
                     updated_at = ?10 \
                 WHERE decision_id = ?1 AND version = ?11",
                params![
                    id,
                    content.question,
                    content.choice,
                    content.rationale,
                    content.assumptions,
                    content.reconsider_when,
                    content.scope,
                    content.consequences,
                    version,
                    updated_at,
                    version - 1,
                ],
            )
            .map_err(storage_error)?;
        if changed == 0 {
            return Ok(false);
        }

        transaction
            .execute(
                "INSERT INTO decision_revisions \
                 (decision_id, version, question, choice, rationale, assumptions, reconsider_when, \
                  scope, consequences, created_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    id,
                    version,
                    content.question,
                    content.choice,
                    content.rationale,
                    content.assumptions,
                    content.reconsider_when,
                    content.scope,
                    content.consequences,
                    updated_at,
                ],
            )
            .map_err(storage_error)?;

        transaction
            .execute("DELETE FROM decisions_fts WHERE decision_id = ?1", [id])
            .map_err(storage_error)?;
        transaction
            .execute(
                "INSERT INTO decisions_fts (decision_id, question, choice, rationale) \
                 VALUES (?1, ?2, ?3, ?4)",
                params![id, content.question, content.choice, content.rationale],
            )
            .map_err(storage_error)?;

        transaction.commit().map_err(storage_error)?;
        Ok(true)
    }
}

/// Appends `count` comma-separated `?` placeholders to `sql`.
fn push_placeholders(sql: &mut String, count: usize) {
    for index in 0..count {
        if index > 0 {
            sql.push_str(", ");
        }
        sql.push('?');
    }
}
