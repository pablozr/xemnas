//! SQLite implementation of the claims port.

use application::claims::{ClaimRecord, ClaimStore, ClaimsError};
use domain::claims::ClaimKind;
use rusqlite::{params, OptionalExtension, Row};

use crate::store::SqliteStore;

const CLAIM_COLUMNS: &str = "claim_id, project_id, kind, statement, valid_from, valid_until, \
     source_decision_id, created_at, updated_at";

impl ClaimStore for SqliteStore {
    fn insert_claim(&self, record: &ClaimRecord) -> Result<(), ClaimsError> {
        let mut connection = self.lock();
        let transaction = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        transaction
            .execute(
                &format!("INSERT INTO context_claims ({CLAIM_COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"),
                params![
                    record.claim_id,
                    record.project_id,
                    record.kind.as_str(),
                    record.statement,
                    record.valid_from,
                    record.valid_until,
                    record.source_decision_id,
                    record.created_at,
                    record.updated_at,
                ],
            )
            .map_err(storage_error)?;
        transaction
            .execute(
                "INSERT INTO claims_fts (claim_id, statement) VALUES (?1, ?2)",
                params![record.claim_id, record.statement],
            )
            .map_err(storage_error)?;
        transaction.commit().map_err(storage_error)
    }

    fn get_claim(&self, claim_id: &str) -> Result<Option<ClaimRecord>, ClaimsError> {
        self.lock()
            .query_row(
                &format!("SELECT {CLAIM_COLUMNS} FROM context_claims WHERE claim_id = ?1"),
                [claim_id],
                map_row,
            )
            .optional()
            .map_err(storage_error)
    }

    fn project_claims(&self, project_id: &str) -> Result<Vec<ClaimRecord>, ClaimsError> {
        let connection = self.lock();
        let mut statement = connection
            .prepare(&format!(
                "SELECT {CLAIM_COLUMNS} FROM context_claims WHERE project_id = ?1 \
                 ORDER BY valid_from, claim_id"
            ))
            .map_err(storage_error)?;
        let rows = statement
            .query_map([project_id], map_row)
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        Ok(rows)
    }

    fn close_claim(
        &self,
        claim_id: &str,
        valid_until: &str,
        expected_until: Option<&str>,
        updated_at: &str,
    ) -> Result<bool, ClaimsError> {
        let changed = self
            .lock()
            .execute(
                "UPDATE context_claims SET valid_until = ?2, updated_at = ?4 \
                 WHERE claim_id = ?1 AND valid_until IS ?3",
                params![claim_id, valid_until, expected_until, updated_at],
            )
            .map_err(storage_error)?;
        Ok(changed == 1)
    }
}

fn map_row(row: &Row<'_>) -> rusqlite::Result<ClaimRecord> {
    let kind_text: String = row.get(2)?;
    let kind = ClaimKind::parse(&kind_text).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            2,
            rusqlite::types::Type::Text,
            format!("tipo de afirmação desconhecido: {kind_text}").into(),
        )
    })?;
    Ok(ClaimRecord {
        claim_id: row.get(0)?,
        project_id: row.get(1)?,
        kind,
        statement: row.get(3)?,
        valid_from: row.get(4)?,
        valid_until: row.get(5)?,
        source_decision_id: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

fn storage_error(error: rusqlite::Error) -> ClaimsError {
    ClaimsError::Storage(error.to_string())
}
