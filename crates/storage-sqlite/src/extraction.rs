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

fn nature_error(e: rusqlite::Error) -> application::InboxError {
    application::InboxError::Storage(e.to_string())
}

pub(crate) fn nature_destination(
    connection: &rusqlite::Connection,
    id: &str,
) -> Result<application::review_exception::NatureDestination, application::InboxError> {
    use application::review_exception::{
        classify_nature, CandidateNature, NatureDestination, ObservationReference,
    };
    let row: Option<(String, Option<String>, String)> = connection
        .query_row(
            "SELECT n.nature,n.observation_reference,dc.project_id FROM candidate_nature n
         JOIN decision_candidates dc ON dc.id=n.candidate_id WHERE dc.id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()
        .map_err(nature_error)?;
    let Some((nature, reference, project)) = row else {
        return Ok(NatureDestination::ReviewRequired);
    };
    let nature: CandidateNature = serde_json::from_value(serde_json::Value::String(nature))
        .map_err(|e| application::InboxError::InvalidData(e.to_string()))?;
    let reference: Option<ObservationReference> = reference
        .map(|s| serde_json::from_str(&s))
        .transpose()
        .map_err(|e| application::InboxError::InvalidData(e.to_string()))?;
    let Some(reference) = reference else {
        return Ok(classify_nature(nature, &project, None, None, false));
    };
    let json: Option<String> = connection
        .query_row(
            "SELECT record_json FROM observation_records WHERE observation_id=?1 AND version=?2
         AND project_id=?3 AND status='current'",
            params![reference.observation_id, reference.version, project],
            |r| r.get(0),
        )
        .optional()
        .map_err(nature_error)?;
    let record: Option<application::observations::ObservationRecord> = json
        .map(|s| serde_json::from_str(&s))
        .transpose()
        .map_err(|e| application::InboxError::InvalidData(e.to_string()))?;
    let mut eligible: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM observation_sources s JOIN observation_refresh f
         ON f.project_id=s.project_id WHERE s.source_id=?1 AND s.project_id=?2
         AND s.sha256=?3 AND s.parser_policy_version=?4 AND s.last_check_status='verified' AND f.dirty=0)",
        params![reference.source_id,project,reference.source_sha256,reference.parser_policy_version],
        |r| r.get(0)).map_err(nature_error)?;
    for support in &reference.supporting_sources {
        eligible &= connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM observation_sources WHERE source_id=?1
             AND project_id=?2 AND sha256=?3 AND last_check_status='verified')",
                params![support.source_id, project, support.source_sha256],
                |r| r.get::<_, bool>(0),
            )
            .map_err(nature_error)?;
    }
    let literal_supported: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM decision_candidates dc
         JOIN capture_artifacts a ON a.capture_id=dc.capture_id
         JOIN json_each(dc.evidence_refs) refs ON refs.value=a.artifact_id
         WHERE dc.id=?1 AND json_valid(a.metadata)
         AND json_type(a.metadata,'$.observation_reference')='object'
         AND NOT EXISTS(SELECT fullkey,type,atom FROM json_tree(json_extract(a.metadata,'$.observation_reference'))
             EXCEPT SELECT fullkey,type,atom FROM json_tree(?2))
         AND NOT EXISTS(SELECT fullkey,type,atom FROM json_tree(?2)
             EXCEPT SELECT fullkey,type,atom FROM json_tree(json_extract(a.metadata,'$.observation_reference')))
          AND dc.choice=?3 AND a.content=?3)",
            params![
                id,
                serde_json::to_string(&reference)
                     .map_err(|e| application::InboxError::InvalidData(e.to_string()))?,
                reference.canonical_description()
            ],
            |r| r.get(0),
        )
        .map_err(nature_error)?;
    eligible &= literal_supported;
    Ok(classify_nature(
        nature,
        &project,
        Some(&reference),
        record.as_ref(),
        eligible,
    ))
}

impl application::review_exception::NatureTraceStore for SqliteStore {
    fn bind_observation(
        &self,
        candidate_id: &str,
        reference: &application::review_exception::ObservationReference,
    ) -> Result<application::review_exception::NatureDestination, application::InboxError> {
        let mut guard = self.lock();
        let tx = guard
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(nature_error)?;
        let project: String = tx
            .query_row(
                "SELECT project_id FROM decision_candidates WHERE id=?1 AND status='pending'",
                [candidate_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(nature_error)?
            .ok_or(application::InboxError::NotFound)?;
        if project != reference.project_id {
            return Err(application::InboxError::InvalidData(
                "observation project mismatch".into(),
            ));
        }
        let nature: Option<String> = tx
            .query_row(
                "SELECT nature FROM candidate_nature WHERE candidate_id=?1",
                [candidate_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(nature_error)?;
        if nature.as_deref() != Some("description") {
            return Err(application::InboxError::InvalidState);
        }
        let json = serde_json::to_string(reference)
            .map_err(|e| application::InboxError::InvalidData(e.to_string()))?;
        // Only an adapter-provided typed hint on the candidate's own cited
        // artifact may bind prose to a fact. A caller cannot hide arbitrary text
        // by supplying the identity of an unrelated valid observation.
        let supported: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM decision_candidates dc
             JOIN capture_artifacts a ON a.capture_id=dc.capture_id
             JOIN json_each(dc.evidence_refs) refs ON refs.value=a.artifact_id
             WHERE dc.id=?1 AND json_valid(a.metadata)
             AND json_type(a.metadata,'$.observation_reference')='object'
             AND NOT EXISTS(SELECT fullkey,type,atom FROM json_tree(json_extract(a.metadata,'$.observation_reference'))
                 EXCEPT SELECT fullkey,type,atom FROM json_tree(?2))
             AND NOT EXISTS(SELECT fullkey,type,atom FROM json_tree(?2)
                 EXCEPT SELECT fullkey,type,atom FROM json_tree(json_extract(a.metadata,'$.observation_reference')))
              AND dc.choice=?3 AND a.content=?3)",
                params![candidate_id, json, reference.canonical_description()],
                |r| r.get(0),
            )
            .map_err(nature_error)?;
        if !supported {
            // Leave classification and source history intact, but never grant
            // descriptive validity to a noncanonical assertion.
            return Ok(application::review_exception::NatureDestination::ReviewRequired);
        }
        tx.execute(
            "UPDATE candidate_nature SET destination='review_required',observation_reference=?2
             WHERE candidate_id=?1 AND nature='description'",
            params![candidate_id, json],
        )
        .map_err(nature_error)?;
        let destination = nature_destination(&tx, candidate_id)?;
        tx.commit().map_err(nature_error)?;
        Ok(destination)
    }
    fn nature_traces(
        &self,
        project_id: &str,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<application::review_exception::NatureTrace>, application::InboxError> {
        use application::review_exception::NatureTrace;
        if limit == 0 || limit > 100 || offset > i64::MAX as usize {
            return Err(application::InboxError::InvalidFilter(
                "limite inválido".into(),
            ));
        }
        let connection = self.lock();
        let mut statement = connection.prepare(
            "SELECT dc.id, coalesce(n.nature, 'unknown'), coalesce(n.destination, 'review_required')
             FROM decision_candidates dc LEFT JOIN candidate_nature n ON n.candidate_id = dc.id
             WHERE dc.project_id = ?1 ORDER BY dc.created_at DESC, dc.id DESC LIMIT ?2 OFFSET ?3"
        ).map_err(|e| application::InboxError::Storage(e.to_string()))?;
        let rows = statement
            .query_map(params![project_id, limit as i64, offset as i64], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })
            .map_err(|e| application::InboxError::Storage(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| application::InboxError::Storage(e.to_string()))?;
        rows.into_iter()
            .map(|(candidate_id, nature, _destination)| {
                let decode = |s: String| serde_json::Value::String(s);
                let destination = nature_destination(&connection, &candidate_id)?;
                Ok(NatureTrace {
                    candidate_id,
                    nature: serde_json::from_value(decode(nature))
                        .map_err(|e| application::InboxError::InvalidData(e.to_string()))?,
                    destination,
                })
            })
            .collect()
    }
}

impl ExtractionStore for SqliteStore {
    fn record_nature(
        &self,
        dedup_hash: &str,
        nature: application::review_exception::CandidateNature,
    ) -> Result<(), ExtractError> {
        use application::review_exception::{classify_nature, NatureDestination};
        let nature_json =
            serde_json::to_value(nature).map_err(|e| ExtractError::Validation(e.to_string()))?;
        let destination = classify_nature(nature, "", None, None, false);
        let destination = match destination {
            NatureDestination::InferenceStored => "inference_stored",
            _ => "review_required",
        };
        self.lock()
            .execute(
                "INSERT OR IGNORE INTO candidate_nature(candidate_id, nature, destination)
             SELECT id, ?2, ?3 FROM decision_candidates WHERE dedup_hash = ?1",
                params![dedup_hash, nature_json.as_str(), destination],
            )
            .map_err(storage_error)?;
        Ok(())
    }
    fn record_components(
        &self,
        dedup_hash: &str,
        components: &[application::extract::CandidateComponent],
    ) -> Result<(), ExtractError> {
        let connection = self.lock();
        for component in components {
            connection
                .execute(
                    "INSERT OR IGNORE INTO candidate_components(candidate_id, entity_id, quote)
                     SELECT id, ?2, ?3 FROM decision_candidates WHERE dedup_hash = ?1",
                    params![dedup_hash, component.entity_id, component.quote],
                )
                .map_err(storage_error)?;
        }
        Ok(())
    }
    fn background(
        &self,
        project_id: &str,
        files: &[String],
    ) -> Result<ExtractionBackground, ExtractError> {
        // What the map ties to these files comes first, then recent records.
        let (tied_decisions, tied_claims) = KnowledgeGraph::new(self.clone())
            .context_for_files(project_id, files, None)
            .unwrap_or_default();
        let entities = KnowledgeGraph::new(self.clone())
            .entities(project_id)
            .unwrap_or_default();
        let components = application::link_suggestions::candidate_components(&entities)
            .into_iter()
            .map(|entity| application::extract::MapComponent {
                entity_id: entity.entity_id.clone(),
                name: entity.name.clone(),
            })
            .collect();
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
            components,
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
                      created_at, updated_at, kind, significance, criteria, qualifiers) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, \
                             ?16, ?17, ?18, ?19)",
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
                        record.qualifiers,
                    ],
                )
                .map_err(storage_error)?;
            inserted += changed;
        }

        transaction.commit().map_err(storage_error)?;
        Ok(inserted)
    }
}
