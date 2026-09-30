//! SQLite implementation of the decision relations port.

use application::decisions::DecisionsError;
use application::relations::{RelationInsert, RelationRow, RelationStore};
use domain::relations::DecisionRelation;
use rusqlite::{params, ErrorCode, Row};

use crate::store::SqliteStore;

impl RelationStore for SqliteStore {
    fn project_relations(&self, project_id: &str) -> Result<Vec<RelationRow>, DecisionsError> {
        query(
            self,
            "SELECT r.from_decision_id, r.to_decision_id, r.kind, r.created_at \
             FROM decision_relations r \
             JOIN engineering_decisions d ON d.decision_id = r.from_decision_id \
             WHERE d.project_id = ?1 \
             ORDER BY r.created_at, r.from_decision_id, r.to_decision_id",
            project_id,
        )
    }

    fn decision_relations(&self, decision_id: &str) -> Result<Vec<RelationRow>, DecisionsError> {
        query(
            self,
            "SELECT from_decision_id, to_decision_id, kind, created_at \
             FROM decision_relations \
             WHERE from_decision_id = ?1 OR to_decision_id = ?1 \
             ORDER BY created_at, from_decision_id, to_decision_id",
            decision_id,
        )
    }

    fn insert_relation(
        &self,
        relation: &DecisionRelation,
        created_at: &str,
        supersede: bool,
    ) -> Result<RelationInsert, DecisionsError> {
        let mut connection = self.lock();
        let transaction = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        if supersede {
            let changed = transaction
                .execute(
                    "UPDATE engineering_decisions SET status = 'superseded', updated_at = ?2 \
                     WHERE decision_id = ?1 AND status = 'accepted'",
                    params![relation.to(), created_at],
                )
                .map_err(storage_error)?;
            if changed == 0 {
                return Ok(RelationInsert::TargetNotAccepted);
            }
        }
        let inserted = transaction.execute(
            "INSERT INTO decision_relations (from_decision_id, to_decision_id, kind, created_at) \
             VALUES (?1, ?2, ?3, ?4)",
            params![
                relation.from(),
                relation.to(),
                relation.kind().as_str(),
                created_at
            ],
        );
        match inserted {
            Ok(_) => {}
            Err(rusqlite::Error::SqliteFailure(error, _))
                if error.code == ErrorCode::ConstraintViolation =>
            {
                return Ok(RelationInsert::Duplicate);
            }
            Err(error) => return Err(storage_error(error)),
        }
        transaction.commit().map_err(storage_error)?;
        Ok(RelationInsert::Inserted)
    }
}

fn query(store: &SqliteStore, sql: &str, id: &str) -> Result<Vec<RelationRow>, DecisionsError> {
    let connection = store.lock();
    let mut statement = connection.prepare(sql).map_err(storage_error)?;
    let rows = statement
        .query_map([id], map_row)
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)?;
    Ok(rows)
}

fn map_row(row: &Row<'_>) -> rusqlite::Result<RelationRow> {
    Ok(RelationRow {
        from: row.get(0)?,
        to: row.get(1)?,
        kind: row.get(2)?,
        created_at: row.get(3)?,
    })
}

fn storage_error(error: rusqlite::Error) -> DecisionsError {
    DecisionsError::Storage(error.to_string())
}
