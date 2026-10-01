//! SQLite implementation of the injection audit port.

use std::collections::BTreeSet;

use application::context::ContextError;
use application::injection::{
    DeliveredItem, InjectionHistory, InjectionMode, InjectionRecord, InjectionStore, ItemKind,
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

impl InjectionHistory for SqliteStore {
    fn recent_injections(
        &self,
        project_id: &str,
        limit: usize,
    ) -> Result<Vec<InjectionRecord>, ContextError> {
        let connection = self.lock();
        let mut statement = connection
            .prepare(
                "SELECT injection_id, session_id, mode, tokens, omitted, created_at \
                 FROM context_injections WHERE project_id = ?1 \
                 ORDER BY created_at DESC, injection_id DESC LIMIT ?2",
            )
            .map_err(storage_error)?;
        let heads = statement
            .query_map(
                params![project_id, i64::try_from(limit).unwrap_or(i64::MAX)],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, i64>(4)?,
                        row.get::<_, String>(5)?,
                    ))
                },
            )
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        let mut items = connection
            .prepare(
                "SELECT item_kind, item_id, item_version FROM context_injection_items \
                 WHERE injection_id = ?1 ORDER BY position",
            )
            .map_err(storage_error)?;
        let mut records = Vec::with_capacity(heads.len());
        for (injection_id, session_id, mode, tokens, omitted, created_at) in heads {
            let delivered = items
                .query_map([&injection_id], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                    ))
                })
                .map_err(storage_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(storage_error)?
                .into_iter()
                .filter_map(|(kind, id, version)| {
                    let kind = match kind.as_str() {
                        "decision" => ItemKind::Decision,
                        "claim" => ItemKind::Claim,
                        _ => return None,
                    };
                    Some(DeliveredItem { kind, id, version })
                })
                .collect();
            records.push(InjectionRecord {
                injection_id,
                session_id,
                project_id: project_id.to_string(),
                mode: if mode == "inject" {
                    InjectionMode::Inject
                } else {
                    InjectionMode::Shadow
                },
                tokens: usize::try_from(tokens).unwrap_or_default(),
                omitted: usize::try_from(omitted).unwrap_or_default(),
                created_at,
                items: delivered,
            });
        }
        Ok(records)
    }
}

fn storage_error(error: rusqlite::Error) -> ContextError {
    ContextError::Storage(error.to_string())
}
