//! SQLite implementation of the Settings → OpenCode persistence port.

use crate::store::SqliteStore;
use application::integration::{AdapterActivityRow, IntegrationError, IntegrationStore};

impl IntegrationStore for SqliteStore {
    fn adapter_activity(&self) -> Result<Vec<AdapterActivityRow>, IntegrationError> {
        let connection = self.lock();
        let mut statement = connection
            .prepare(
                "SELECT c.adapter, \
                        (SELECT latest.adapter_version FROM adapter_checkpoints latest \
                          WHERE latest.adapter = c.adapter \
                          ORDER BY latest.updated_at DESC, latest.observed_at DESC LIMIT 1), \
                        COUNT(*), MAX(c.observed_at), MAX(c.updated_at) \
                 FROM adapter_checkpoints c \
                 GROUP BY c.adapter \
                 ORDER BY MAX(c.updated_at) DESC, c.adapter",
            )
            .map_err(storage_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok(AdapterActivityRow {
                    adapter: row.get(0)?,
                    adapter_version: row.get(1)?,
                    sessions: row.get(2)?,
                    last_observed_at: row.get(3)?,
                    last_received_at: row.get(4)?,
                })
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        Ok(rows)
    }
}

fn storage_error(error: rusqlite::Error) -> IntegrationError {
    IntegrationError::Storage(error.to_string())
}
