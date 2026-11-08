//! SQLite FTS5 ranking for Context Packs.

use std::collections::BTreeMap;

use application::context::{ContextError, ContextStore, DocumentOrigin};
use rusqlite::params;

use crate::store::SqliteStore;

/// The first `document` file of candidate `c`'s capture, as a SQL column.
macro_rules! document_file_of_candidate {
    () => {
        "(SELECT min(json_extract(a.metadata, '$.file')) FROM capture_artifacts a \
         WHERE a.capture_id = c.capture_id AND a.kind = 'document')"
    };
}

/// Whether candidate `c`'s capture carries a `document` artifact.
macro_rules! candidate_has_document {
    () => {
        "EXISTS (SELECT 1 FROM capture_artifacts a \
         WHERE a.capture_id = c.capture_id AND a.kind = 'document')"
    };
}

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

    fn document_origin(&self, project_id: &str) -> Result<DocumentOrigin, ContextError> {
        let connection = self.lock();
        let mut origin = DocumentOrigin::default();
        let decisions = concat!(
            "SELECT d.decision_id, ",
            document_file_of_candidate!(),
            " FROM engineering_decisions d",
            " JOIN decision_candidates c ON c.id = d.candidate_id",
            " WHERE d.project_id = ?1 AND ",
            candidate_has_document!(),
        );
        let claims = concat!(
            "SELECT k.claim_id, ",
            document_file_of_candidate!(),
            " FROM context_claims k",
            " JOIN decision_candidates c ON c.id = COALESCE(k.source_candidate_id,",
            " (SELECT d.candidate_id FROM engineering_decisions d",
            " WHERE d.decision_id = k.source_decision_id))",
            " WHERE k.project_id = ?1 AND ",
            candidate_has_document!(),
        );
        collect_document_origin(&connection, decisions, project_id, &mut origin, |origin| {
            &mut origin.decisions
        })?;
        collect_document_origin(&connection, claims, project_id, &mut origin, |origin| {
            &mut origin.claims
        })?;
        Ok(origin)
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

/// Runs `sql` (`id, file` rows), adding each id to the set `ids` picks and its
/// file to `path_of`.
fn collect_document_origin(
    connection: &rusqlite::Connection,
    sql: &str,
    project_id: &str,
    origin: &mut DocumentOrigin,
    ids: fn(&mut DocumentOrigin) -> &mut std::collections::HashSet<String>,
) -> Result<(), ContextError> {
    let mut statement = connection.prepare_cached(sql).map_err(storage_error)?;
    let rows = statement
        .query_map([project_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
        })
        .map_err(storage_error)?;
    for row in rows {
        let (id, path) = row.map_err(storage_error)?;
        ids(origin).insert(id.clone());
        if let Some(path) = path {
            origin.path_of.insert(id, path);
        }
    }
    Ok(())
}
