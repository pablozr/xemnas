//! SQLite implementation of the injection audit port.

use std::collections::BTreeSet;

use application::context::ContextError;
use application::injection::{
    DeliveredItem, InjectionMode, InjectionRecord, InjectionStore, ItemKind,
};
use rusqlite::params;

use crate::store::SqliteStore;

impl InjectionStore for SqliteStore {
    fn delivered(
        &self,
        session_id: &str,
        mode: InjectionMode,
    ) -> Result<BTreeSet<DeliveredItem>, ContextError> {
        let connection = self.lock();
        let mut statement = connection
            .prepare(
                "SELECT DISTINCT i.item_kind, i.item_id, i.item_version \
                 FROM context_injection_items i \
                 JOIN context_injections c ON c.injection_id = i.injection_id \
                 WHERE c.session_id = ?1 AND c.mode = ?2",
            )
            .map_err(storage_error)?;
        let rows = statement
            .query_map(params![session_id, mode.as_str()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        Ok(rows
            .into_iter()
            .filter_map(|(kind, id, version)| {
                let kind = match kind.as_str() {
                    "decision" => ItemKind::Decision,
                    "claim" => ItemKind::Claim,
                    _ => return None,
                };
                Some(DeliveredItem { kind, id, version })
            })
            .collect())
    }

    fn record_injection(&self, record: &InjectionRecord) -> Result<(), ContextError> {
        let mut connection = self.lock();
        let transaction = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        transaction
            .execute(
                "INSERT INTO context_injections \
                 (injection_id, session_id, project_id, mode, tokens, omitted, created_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    record.injection_id,
                    record.session_id,
                    record.project_id,
                    record.mode.as_str(),
                    record.tokens as i64,
                    record.omitted as i64,
                    record.created_at,
                ],
            )
            .map_err(storage_error)?;
        for (position, item) in record.items.iter().enumerate() {
            transaction
                .execute(
                    "INSERT INTO context_injection_items \
                     (injection_id, position, item_kind, item_id, item_version) \
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        record.injection_id,
                        position as i64,
                        item.kind.as_str(),
                        item.id,
                        item.version,
                    ],
                )
                .map_err(storage_error)?;
        }
        transaction.commit().map_err(storage_error)
    }
}

fn storage_error(error: rusqlite::Error) -> ContextError {
    ContextError::Storage(error.to_string())
}
