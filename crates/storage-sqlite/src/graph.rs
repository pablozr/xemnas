//! SQLite implementation of the knowledge graph port (ADR-0005).

use application::graph::{
    summary_files, DecisionNode, EdgeRecord, EntityRecord, GraphError, GraphStore, RuleSource,
};
use domain::entities::{EdgeKind, EdgeOrigin, EntityKind, NodeKind};
use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::store::SqliteStore;

pub(crate) const ENTITY_COLUMNS: &str =
    "entity_id, project_id, kind, name, key, description, created_at, retired_at";

pub(crate) const EDGE_COLUMNS: &str =
    "edge_id, project_id, kind, source_kind, source_id, entity_id, origin, \
     reason, created_at, confirmed_at, invalidated_at";

impl GraphStore for SqliteStore {
    fn project_entities(&self, project_id: &str) -> Result<Vec<EntityRecord>, GraphError> {
        let connection = self.lock();
        let mut statement = connection
            .prepare(&format!(
                "SELECT {ENTITY_COLUMNS} FROM entities WHERE project_id = ?1 \
                 ORDER BY kind, key, entity_id"
            ))
            .map_err(storage_error)?;
        let mut entities = statement
            .query_map([project_id], map_entity)
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        for entity in &mut entities {
            load_lists(&connection, entity)?;
        }
        Ok(entities)
    }

    fn get_entity(&self, entity_id: &str) -> Result<Option<EntityRecord>, GraphError> {
        let connection = self.lock();
        let entity = connection
            .query_row(
                &format!("SELECT {ENTITY_COLUMNS} FROM entities WHERE entity_id = ?1"),
                [entity_id],
                map_entity,
            )
            .optional()
            .map_err(storage_error)?;
        match entity {
            Some(mut entity) => {
                load_lists(&connection, &mut entity)?;
                Ok(Some(entity))
            }
            None => Ok(None),
        }
    }

    fn insert_entity(&self, record: &EntityRecord) -> Result<(), GraphError> {
        let mut connection = self.lock();
        let transaction = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        transaction
            .execute(
                &format!(
                    "INSERT INTO entities ({ENTITY_COLUMNS}) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"
                ),
                params![
                    record.entity_id,
                    record.project_id,
                    record.kind.as_str(),
                    record.name,
                    record.key,
                    record.description,
                    record.created_at,
                    record.retired_at,
                ],
            )
            .map_err(storage_error)?;
        write_lists(&transaction, record)?;
        transaction.commit().map_err(storage_error)
    }

    fn update_entity(&self, record: &EntityRecord) -> Result<bool, GraphError> {
        let mut connection = self.lock();
        let transaction = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        let changed = transaction
            .execute(
                "UPDATE entities SET name = ?2, key = ?3, description = ?4 \
                 WHERE entity_id = ?1 AND retired_at IS NULL",
                params![
                    record.entity_id,
                    record.name,
                    record.key,
                    record.description
                ],
            )
            .map_err(storage_error)?;
        if changed == 0 {
            return Ok(false);
        }
        for table in ["entity_patterns", "entity_aliases"] {
            transaction
                .execute(
                    &format!("DELETE FROM {table} WHERE entity_id = ?1"),
                    [&record.entity_id],
                )
                .map_err(storage_error)?;
        }
        write_lists(&transaction, record)?;
        transaction.commit().map_err(storage_error)?;
        Ok(true)
    }

    fn retire_entity(&self, entity_id: &str, at: &str) -> Result<bool, GraphError> {
        let changed = self
            .lock()
            .execute(
                "UPDATE entities SET retired_at = ?2 WHERE entity_id = ?1 AND retired_at IS NULL",
                params![entity_id, at],
            )
            .map_err(storage_error)?;
        Ok(changed == 1)
    }

    fn project_edges(&self, project_id: &str) -> Result<Vec<EdgeRecord>, GraphError> {
        let connection = self.lock();
        let mut statement = connection
            .prepare(&format!(
                "SELECT {EDGE_COLUMNS} FROM entity_edges WHERE project_id = ?1 \
                 ORDER BY created_at, edge_id"
            ))
            .map_err(storage_error)?;
        let rows = statement
            .query_map([project_id], map_edge)
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        Ok(rows)
    }

    fn get_edge(&self, edge_id: &str) -> Result<Option<EdgeRecord>, GraphError> {
        self.lock()
            .query_row(
                &format!("SELECT {EDGE_COLUMNS} FROM entity_edges WHERE edge_id = ?1"),
                [edge_id],
                map_edge,
            )
            .optional()
            .map_err(storage_error)
    }

    fn insert_edge(&self, record: &EdgeRecord) -> Result<(), GraphError> {
        let inserted = self.lock().execute(
            &format!(
                "INSERT INTO entity_edges ({EDGE_COLUMNS}) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)"
            ),
            params![
                record.edge_id,
                record.project_id,
                record.kind.as_str(),
                record.source_kind.as_str(),
                record.source_id,
                record.entity_id,
                record.origin.as_str(),
                record.reason,
                record.created_at,
                record.confirmed_at,
                record.invalidated_at,
            ],
        );
        match inserted {
            Ok(_) => Ok(()),
            Err(rusqlite::Error::SqliteFailure(error, _))
                if error.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                Err(GraphError::DuplicateEdge)
            }
            Err(error) => Err(storage_error(error)),
        }
    }

    fn confirm_edge(&self, edge_id: &str, at: &str) -> Result<bool, GraphError> {
        let changed = self
            .lock()
            .execute(
                "UPDATE entity_edges SET confirmed_at = ?2 \
                 WHERE edge_id = ?1 AND confirmed_at IS NULL AND invalidated_at IS NULL",
                params![edge_id, at],
            )
            .map_err(storage_error)?;
        Ok(changed == 1)
    }

    fn invalidate_edge(&self, edge_id: &str, at: &str) -> Result<bool, GraphError> {
        let changed = self
            .lock()
            .execute(
                "UPDATE entity_edges SET invalidated_at = ?2 \
                 WHERE edge_id = ?1 AND invalidated_at IS NULL",
                params![edge_id, at],
            )
            .map_err(storage_error)?;
        Ok(changed == 1)
    }

    fn project_decisions(&self, project_id: &str) -> Result<Vec<DecisionNode>, GraphError> {
        let connection = self.lock();
        let mut statement = connection
            .prepare(
                "SELECT d.decision_id, d.question, d.choice, d.confirmed_at, \
                        COALESCE(c.diff_summary, '{}'), d.capture_id \
                 FROM engineering_decisions d \
                 LEFT JOIN decision_candidates c ON c.id = d.candidate_id \
                 WHERE d.project_id = ?1 \
                 ORDER BY d.confirmed_at, d.decision_id",
            )
            .map_err(storage_error)?;
        let rows = statement
            .query_map([project_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                ))
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        // Only the hunks the decision cites: two decisions of one capture do
        // not share each other's dependencies.
        let mut hunks = connection
            .prepare(
                "SELECT a.content FROM evidence_links l \
                 JOIN capture_artifacts a \
                   ON a.artifact_id = l.artifact_id AND a.capture_id = ?2 \
                 WHERE l.decision_id = ?1 AND a.kind = 'diff_hunk' \
                 ORDER BY l.position",
            )
            .map_err(storage_error)?;
        let mut decisions = Vec::with_capacity(rows.len());
        for (decision_id, question, choice, confirmed_at, summary, capture_id) in rows {
            let diffs = match &capture_id {
                Some(capture_id) => hunks
                    .query_map(params![decision_id, capture_id], |row| {
                        row.get::<_, String>(0)
                    })
                    .map_err(storage_error)?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(storage_error)?,
                None => Vec::new(),
            };
            decisions.push(DecisionNode {
                decision_id,
                question,
                choice,
                confirmed_at,
                files: summary_files(&summary),
                diffs,
            });
        }
        Ok(decisions)
    }

    fn rule_sources(&self, project_id: &str) -> Result<Vec<RuleSource>, GraphError> {
        self.rule_sources_of(project_id)
    }
}

impl SqliteStore {
    pub(crate) fn rule_sources_of(&self, project_id: &str) -> Result<Vec<RuleSource>, GraphError> {
        let connection = self.lock();
        let mut statement = connection
            .prepare(
                "SELECT c.claim_id, COALESCE(d.diff_summary, '{}') \
                 FROM context_claims c \
                 JOIN decision_candidates d ON d.id = c.source_candidate_id \
                 WHERE c.project_id = ?1 ORDER BY c.claim_id",
            )
            .map_err(storage_error)?;
        let rows = statement
            .query_map([project_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        Ok(rows
            .into_iter()
            .map(|(claim_id, summary)| RuleSource {
                claim_id,
                files: summary_files(&summary),
            })
            .collect())
    }
}

pub(crate) fn load_lists(
    connection: &Connection,
    entity: &mut EntityRecord,
) -> Result<(), GraphError> {
    let list = |table: &str, column: &str| -> Result<Vec<String>, GraphError> {
        let mut statement = connection
            .prepare(&format!(
                "SELECT {column} FROM {table} WHERE entity_id = ?1 ORDER BY position"
            ))
            .map_err(storage_error)?;
        let values = statement
            .query_map([&entity.entity_id], |row| row.get::<_, String>(0))
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        Ok(values)
    };
    entity.patterns = list("entity_patterns", "pattern")?;
    entity.aliases = list("entity_aliases", "alias")?;
    Ok(())
}

fn write_lists(connection: &Connection, record: &EntityRecord) -> Result<(), GraphError> {
    for (position, pattern) in record.patterns.iter().enumerate() {
        connection
            .execute(
                "INSERT INTO entity_patterns (entity_id, position, pattern) VALUES (?1, ?2, ?3)",
                params![record.entity_id, position as i64, pattern],
            )
            .map_err(storage_error)?;
    }
    for (position, alias) in record.aliases.iter().enumerate() {
        connection
            .execute(
                "INSERT INTO entity_aliases (entity_id, position, alias) VALUES (?1, ?2, ?3)",
                params![record.entity_id, position as i64, alias],
            )
            .map_err(storage_error)?;
    }
    Ok(())
}

fn invalid_column(index: usize, what: &str, value: &str) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        index,
        rusqlite::types::Type::Text,
        format!("{what} desconhecido: {value}").into(),
    )
}

pub(crate) fn map_entity(row: &Row<'_>) -> rusqlite::Result<EntityRecord> {
    let kind: String = row.get(2)?;
    Ok(EntityRecord {
        entity_id: row.get(0)?,
        project_id: row.get(1)?,
        kind: EntityKind::parse(&kind).ok_or_else(|| invalid_column(2, "tipo de item", &kind))?,
        name: row.get(3)?,
        key: row.get(4)?,
        description: row.get(5)?,
        patterns: Vec::new(),
        aliases: Vec::new(),
        created_at: row.get(6)?,
        retired_at: row.get(7)?,
    })
}

pub(crate) fn map_edge(row: &Row<'_>) -> rusqlite::Result<EdgeRecord> {
    let kind: String = row.get(2)?;
    let source_kind: String = row.get(3)?;
    let origin: String = row.get(6)?;
    Ok(EdgeRecord {
        edge_id: row.get(0)?,
        project_id: row.get(1)?,
        kind: EdgeKind::parse(&kind).ok_or_else(|| invalid_column(2, "tipo de vínculo", &kind))?,
        source_kind: NodeKind::parse(&source_kind)
            .ok_or_else(|| invalid_column(3, "tipo de origem", &source_kind))?,
        source_id: row.get(4)?,
        entity_id: row.get(5)?,
        origin: EdgeOrigin::parse(&origin)
            .ok_or_else(|| invalid_column(6, "procedência", &origin))?,
        reason: row.get(7)?,
        created_at: row.get(8)?,
        confirmed_at: row.get(9)?,
        invalidated_at: row.get(10)?,
    })
}

fn storage_error(error: rusqlite::Error) -> GraphError {
    GraphError::Storage(error.to_string())
}
