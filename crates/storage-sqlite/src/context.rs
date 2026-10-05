//! SQLite FTS5 ranking for Context Packs.

use std::collections::BTreeMap;

use application::context::{ContextError, ContextStore};
use rusqlite::params;

use crate::store::SqliteStore;

impl ContextStore for SqliteStore {
    fn observations_dirty(&self, project_id: &str) -> Result<bool, ContextError> {
        self.lock()
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM observation_refresh WHERE project_id=?1 AND dirty=1)",
                [project_id],
                |row| row.get(0),
            )
            .map_err(storage_error)
    }
    fn rank_decisions(
        &self,
        project_id: &str,
        match_query: &str,
        limit: usize,
    ) -> Result<Vec<String>, ContextError> {
        rank(
            self,
            "SELECT f.decision_id FROM decisions_fts f \
             JOIN engineering_decisions d ON d.decision_id = f.decision_id \
             WHERE decisions_fts MATCH ?1 AND d.project_id = ?2 \
             ORDER BY bm25(decisions_fts, 0.0, 3.0, 2.0, 1.0, 1.0), f.decision_id LIMIT ?3",
            project_id,
            match_query,
            limit,
        )
    }

    fn decision_search_terms(
        &self,
        project_id: &str,
    ) -> Result<BTreeMap<String, String>, ContextError> {
        let connection = self.lock();
        let mut statement = connection
            .prepare(
                "SELECT t.decision_id, group_concat(j.value, ' ') \
                 FROM decision_search_terms t \
                 JOIN engineering_decisions d ON d.decision_id = t.decision_id, \
                 json_each(t.terms) j \
                 WHERE d.project_id = ?1 GROUP BY t.decision_id",
            )
            .map_err(storage_error)?;
        let terms = statement
            .query_map([project_id], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(storage_error)?
            .collect::<Result<BTreeMap<String, String>, _>>()
            .map_err(storage_error)?;
        Ok(terms)
    }

    fn rank_claims(
        &self,
        project_id: &str,
        match_query: &str,
        limit: usize,
    ) -> Result<Vec<String>, ContextError> {
        rank(
            self,
            "SELECT f.claim_id FROM claims_fts f \
             JOIN context_claims c ON c.claim_id = f.claim_id \
             WHERE claims_fts MATCH ?1 AND c.project_id = ?2 \
             ORDER BY bm25(claims_fts), f.claim_id LIMIT ?3",
            project_id,
            match_query,
            limit,
        )
    }
}

fn rank(
    store: &SqliteStore,
    sql: &str,
    project_id: &str,
    match_query: &str,
    limit: usize,
) -> Result<Vec<String>, ContextError> {
    let connection = store.lock();
    let mut statement = connection.prepare(sql).map_err(storage_error)?;
    let ids = statement
        .query_map(params![match_query, project_id, limit as i64], |row| {
            row.get(0)
        })
        .map_err(storage_error)?
        .collect::<Result<Vec<String>, _>>()
        .map_err(storage_error)?;
    Ok(ids)
}

fn storage_error(error: rusqlite::Error) -> ContextError {
    ContextError::Storage(error.to_string())
}
