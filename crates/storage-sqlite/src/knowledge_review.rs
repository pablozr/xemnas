//! Atomic read-only snapshot for the ephemeral knowledge reviewer.
use crate::{claims, decisions, graph, store::SqliteStore};
use application::{
    claim_suggestions::ClaimSuggestionRecord,
    knowledge_review::{ReviewError, ReviewSnapshot, ReviewStore},
    relation_suggestions::RelationSuggestionRecord,
    relations::RelationRow,
};
use domain::{claims::ClaimKind, relations::RelationKind};
use rusqlite::{Connection, OptionalExtension, Row};

fn error(e: rusqlite::Error) -> ReviewError {
    match e {
        rusqlite::Error::FromSqlConversionFailure(..) | rusqlite::Error::InvalidColumnType(..) => {
            ReviewError::InvalidData("coluna inválida no inventário".into())
        }
        _ => ReviewError::Storage("falha ao ler inventário".into()),
    }
}
fn rows<T>(
    c: &Connection,
    sql: &str,
    id: &str,
    map: fn(&Row<'_>) -> rusqlite::Result<T>,
) -> Result<Vec<T>, ReviewError> {
    c.prepare(sql)
        .map_err(error)?
        .query_map([id], map)
        .map_err(error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(error)
}
fn invalid_kind() -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Text,
        "tipo desconhecido".into(),
    )
}
impl ReviewStore for SqliteStore {
    fn review_snapshot(&self, id: &str) -> Result<Option<ReviewSnapshot>, ReviewError> {
        let mut connection = self.lock();
        let tx = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Deferred)
            .map_err(error)?;
        if tx
            .query_row("SELECT id FROM projects WHERE id=?1", [id], |r| {
                r.get::<_, String>(0)
            })
            .optional()
            .map_err(error)?
            .is_none()
        {
            return Ok(None);
        }
        let decisions = rows(
            &tx,
            &format!(
                "SELECT {} FROM engineering_decisions d \
            JOIN projects p ON p.id=d.project_id WHERE d.project_id=?1 ORDER BY d.decision_id",
                decisions::DECISION_COLUMNS
            ),
            id,
            decisions::map_row,
        )?;
        let claims = rows(
            &tx,
            &format!(
                "SELECT {} FROM context_claims WHERE project_id=?1 \
            ORDER BY claim_id",
                claims::CLAIM_COLUMNS
            ),
            id,
            claims::map_row,
        )?;
        // Either endpoint: cross-project corruption must not disappear behind a source-only join.
        let relations = rows(&tx,"SELECT r.from_decision_id,r.to_decision_id,r.kind,r.created_at \
            FROM decision_relations r WHERE r.from_decision_id IN \
            (SELECT decision_id FROM engineering_decisions WHERE project_id=?1) OR \
            r.to_decision_id IN (SELECT decision_id FROM engineering_decisions WHERE project_id=?1) \
            ORDER BY r.from_decision_id,r.to_decision_id,r.kind",id,|r| Ok(RelationRow {
                from:r.get(0)?,to:r.get(1)?,kind:r.get(2)?,created_at:r.get(3)? }))?;
        let mut entities = rows(
            &tx,
            &format!(
                "SELECT {} FROM entities WHERE project_id=?1 \
            ORDER BY entity_id",
                graph::ENTITY_COLUMNS
            ),
            id,
            graph::map_entity,
        )?;
        for entity in &mut entities {
            graph::load_lists(&tx, entity)
                .map_err(|_| ReviewError::Storage("listas de entidade".into()))?;
        }
        let edges = rows(
            &tx,
            &format!(
                "SELECT {} FROM entity_edges WHERE project_id=?1 \
            ORDER BY edge_id",
                graph::EDGE_COLUMNS
            ),
            id,
            graph::map_edge,
        )?;
        let relation_suggestions = rows(&tx,"SELECT suggestion_id,project_id,from_id,to_id,kind, \
            quote,reason,created_at FROM relation_suggestions WHERE project_id=?1 AND outcome IS NULL \
            ORDER BY suggestion_id",id,|r| {
            let kind: String = r.get(4)?;
            Ok(RelationSuggestionRecord { suggestion_id:r.get(0)?,project_id:r.get(1)?,
                from_id:r.get(2)?,to_id:r.get(3)?,kind:RelationKind::parse(&kind).ok_or_else(invalid_kind)?,
                quote:r.get(5)?,reason:r.get(6)?,created_at:r.get(7)? })
        })?;
        let claim_suggestions = rows(
            &tx,
            "SELECT suggestion_id,project_id,decision_id,kind,statement, \
            quote,created_at FROM claim_suggestions WHERE project_id=?1 AND outcome IS NULL \
            ORDER BY suggestion_id",
            id,
            |r| {
                let kind: String = r.get(3)?;
                Ok(ClaimSuggestionRecord {
                    suggestion_id: r.get(0)?,
                    project_id: r.get(1)?,
                    decision_id: r.get(2)?,
                    kind: ClaimKind::parse(&kind).ok_or_else(invalid_kind)?,
                    statement: r.get(4)?,
                    quote: r.get(5)?,
                    created_at: r.get(6)?,
                })
            },
        )?;
        tx.commit().map_err(error)?;
        Ok(Some(ReviewSnapshot {
            project_id: id.into(),
            decisions,
            claims,
            relations,
            entities,
            edges,
            relation_suggestions,
            claim_suggestions,
        }))
    }
    fn search_review_decisions(
        &self,
        id: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<String>, ReviewError> {
        let c = self.lock();
        let mut stmt = c
            .prepare(
                "SELECT f.decision_id FROM decisions_fts f \
            JOIN engineering_decisions d ON d.decision_id=f.decision_id \
            WHERE decisions_fts MATCH ?1 AND d.project_id=?2 AND d.status='accepted' \
            ORDER BY bm25(decisions_fts),f.decision_id LIMIT ?3",
            )
            .map_err(error)?;
        let result = stmt
            .query_map(rusqlite::params![query, id, limit.min(100) as i64], |r| {
                r.get(0)
            })
            .map_err(error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(error);
        result
    }
}
