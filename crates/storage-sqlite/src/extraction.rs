//! SQLite implementation of the extraction persistence port.

use application::extract::{
    truncate_content, DecisionCandidateRecord, DecisionEvidence, EvidenceArtifact, ExtractError,
    ExtractionBackground, ExtractionStore, MAX_EVIDENCE_ARTIFACTS, MAX_EVIDENCE_CONTENT_BYTES,
};
use application::graph::KnowledgeGraph;
use rusqlite::{params, OptionalExtension};

use crate::store::SqliteStore;

/// Converts a query failure into a storage error carried by the use-case error.
fn storage_error(error: rusqlite::Error) -> ExtractError {
    ExtractError::Storage(error.to_string())
}

/// Recent decisions and rules listed as "already recorded" when the map
/// ties nothing to the capture's files.
const RECENT_KNOWN: usize = 10;
/// Confirmed and rejected candidates shown as the user's taste.
const TASTE_EXAMPLES: usize = 6;

impl ExtractionStore for SqliteStore {
    fn background(
        &self,
        project_id: &str,
        files: &[String],
    ) -> Result<ExtractionBackground, ExtractError> {
        // What the map ties to these files comes first, then recent records.
        let (tied_decisions, tied_claims) = KnowledgeGraph::new(self.clone())
            .context_for_files(project_id, files, None)
            .unwrap_or_default();
        let connection = self.lock();
        let mut known: Vec<String> = Vec::new();
        let mut decision = connection
            .prepare("SELECT question, choice FROM engineering_decisions WHERE decision_id = ?1")
            .map_err(storage_error)?;
        for id in &tied_decisions {
            if let Some((question, choice)) = decision
                .query_row([id], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .optional()
                .map_err(storage_error)?
            {
                known.push(format!("{question} → {choice}"));
            }
        }
        let mut claim = connection
            .prepare("SELECT statement FROM context_claims WHERE claim_id = ?1")
            .map_err(storage_error)?;
        for id in &tied_claims {
            if let Some(statement) = claim
                .query_row([id], |row| row.get::<_, String>(0))
                .optional()
                .map_err(storage_error)?
            {
                known.push(format!("rule: {statement}"));
            }
        }
        let rows = |sql: &str, limit: usize| -> Result<Vec<String>, ExtractError> {
            let mut statement = connection.prepare(sql).map_err(storage_error)?;
            let values = statement
                .query_map(params![project_id, limit as i64], |row| {
                    Ok(format!(
                        "{} → {}",
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?
                    ))
                })
                .map_err(storage_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(storage_error)?;
            Ok(values)
        };
        for line in rows(
            "SELECT question, choice FROM engineering_decisions \
             WHERE project_id = ?1 AND status = 'accepted' \
             ORDER BY confirmed_at DESC LIMIT ?2",
            RECENT_KNOWN,
        )? {
            if !known.contains(&line) {
                known.push(line);
            }
        }
        let mut statements = connection
            .prepare(
                "SELECT statement FROM context_claims WHERE project_id = ?1 \
                 AND valid_until IS NULL ORDER BY valid_from DESC LIMIT ?2",
            )
            .map_err(storage_error)?;
        for statement in statements
            .query_map(params![project_id, RECENT_KNOWN as i64], |row| {
                row.get::<_, String>(0)
            })
            .map_err(storage_error)?
        {
            let line = format!("rule: {}", statement.map_err(storage_error)?);
            if !known.contains(&line) {
                known.push(line);
            }
        }
        let confirmed = rows(
            "SELECT question, choice FROM decision_candidates \
             WHERE project_id = ?1 AND status IN ('accepted', 'edited_and_accepted') \
             ORDER BY updated_at DESC LIMIT ?2",
            TASTE_EXAMPLES,
        )?;
        let rejected = rows(
            "SELECT question, choice FROM decision_candidates \
             WHERE project_id = ?1 AND status = 'dismissed' \
             ORDER BY updated_at DESC LIMIT ?2",
            TASTE_EXAMPLES,
        )?;
        Ok(ExtractionBackground {
            known,
            confirmed,
            rejected,
        })
    }

    fn load_evidence(&self, capture_id: &str) -> Result<Option<DecisionEvidence>, ExtractError> {
        let connection = self.lock();

        let receipt = connection
            .query_row(
                "SELECT canonical_path, received_at FROM capture_receipts WHERE capture_id = ?1",
                [capture_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(storage_error)?;
        let Some((canonical_path, received_at)) = receipt else {
            return Ok(None);
        };

        let project_id = connection
            .query_row(
                "SELECT id FROM projects WHERE location = ?1",
                [&canonical_path],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(storage_error)?;
        let Some(project_id) = project_id else {
            return Ok(None);
        };

        let checkpoint = connection
            .query_row(
                "SELECT adapter, session_id, observed_at FROM adapter_checkpoints \
                 WHERE capture_id = ?1 ORDER BY updated_at DESC LIMIT 1",
                [capture_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()
            .map_err(storage_error)?;

        let mut statement = connection
            .prepare(
                "SELECT artifact_id, kind, content, metadata FROM capture_artifacts \
                 WHERE capture_id = ?1 ORDER BY artifact_id LIMIT ?2",
            )
            .map_err(storage_error)?;
        let artifacts = statement
            .query_map(params![capture_id, MAX_EVIDENCE_ARTIFACTS as i64], |row| {
                let content: String = row.get(2)?;
                Ok(EvidenceArtifact {
                    artifact_id: row.get(0)?,
                    kind: row.get(1)?,
                    content: truncate_content(&content, MAX_EVIDENCE_CONTENT_BYTES),
                    metadata: row.get(3)?,
                })
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;

        let (adapter, session_id, observed_at) = match checkpoint {
            Some((adapter, session_id, observed_at)) => {
                (Some(adapter), Some(session_id), Some(observed_at))
            }
            None => (None, None, Some(received_at)),
        };

        Ok(Some(DecisionEvidence {
            capture_id: capture_id.to_string(),
            project_id,
            adapter,
            session_id,
            observed_at,
            artifacts,
        }))
    }

    fn insert_candidates(
        &self,
        records: &[DecisionCandidateRecord],
    ) -> Result<usize, ExtractError> {
        let mut connection = self.lock();
        let transaction = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;

        let mut inserted = 0usize;
        for record in records {
            let changed = transaction
                .execute(
                    "INSERT OR IGNORE INTO decision_candidates \
                     (id, project_id, capture_id, status, question, choice, rationale, signals, \
                      confidence, confidence_reason, evidence_refs, diff_summary, dedup_hash, \
                      created_at, updated_at, kind, significance, criteria) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, \
                             ?16, ?17, ?18)",
                    params![
                        record.id,
                        record.project_id,
                        record.capture_id,
                        record.status,
                        record.question,
                        record.choice,
                        record.rationale,
                        record.signals,
                        record.confidence,
                        record.confidence_reason,
                        record.evidence_refs,
                        record.diff_summary,
                        record.dedup_hash,
                        record.created_at,
                        record.updated_at,
                        record.kind,
                        record.significance,
                        record.criteria,
                    ],
                )
                .map_err(storage_error)?;
            inserted += changed;
        }

        transaction.commit().map_err(storage_error)?;
        Ok(inserted)
    }
}
