//! Search terms of decisions and their column in the full-text index.

use application::search_terms::{SearchTermStore, TermSubject};
use rusqlite::params;

use crate::store::SqliteStore;

impl SearchTermStore for SqliteStore {
    fn decisions_without_terms(
        &self,
        project_id: &str,
        limit: usize,
    ) -> Result<Vec<TermSubject>, String> {
        let connection = self.lock();
        let mut statement = connection
            .prepare(
                "SELECT d.decision_id, d.question, d.choice, d.rationale \
                 FROM engineering_decisions d \
                 WHERE d.project_id = ?1 AND d.status = 'accepted' \
                 AND NOT EXISTS \
                   (SELECT 1 FROM decision_search_terms t WHERE t.decision_id = d.decision_id) \
                 ORDER BY d.confirmed_at, d.decision_id LIMIT ?2",
            )
            .map_err(|error| error.to_string())?;
        let subjects = statement
            .query_map(params![project_id, limit as i64], |row| {
                Ok(TermSubject {
                    decision_id: row.get(0)?,
                    question: row.get(1)?,
                    choice: row.get(2)?,
                    rationale: row.get(3)?,
                })
            })
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
        Ok(subjects)
    }

    fn set_search_terms(
        &self,
        decision_id: &str,
        terms: &[String],
        model: &str,
    ) -> Result<(), String> {
        let encoded = serde_json::to_string(terms).map_err(|error| error.to_string())?;
        let mut connection = self.lock();
        let transaction = connection
            .transaction()
            .map_err(|error| error.to_string())?;
        transaction
            .execute(
                "INSERT INTO decision_search_terms (decision_id, terms, model, created_at) \
                 VALUES (?1, ?2, ?3, strftime('%Y-%m-%dT%H:%M:%SZ', 'now')) \
                 ON CONFLICT(decision_id) DO UPDATE SET \
                 terms = excluded.terms, model = excluded.model, created_at = excluded.created_at",
                params![decision_id, encoded, model],
            )
            .map_err(|error| error.to_string())?;
        transaction
            .execute(
                "UPDATE decisions_fts SET search_terms = ?2 WHERE decision_id = ?1",
                params![decision_id, terms.join(" ; ")],
            )
            .map_err(|error| error.to_string())?;
        transaction.commit().map_err(|error| error.to_string())
    }
}
