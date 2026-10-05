//! SQLite implementation of the Decision Inbox persistence port.

use std::collections::HashMap;

use application::briefing::Briefing;
use application::inbox::{
    ArtifactView, CandidateStatus, DecisionSeed, InboxError, InboxQuery, InboxStore,
    StoredCandidate, ValidatedEdits,
};
use rusqlite::types::Value;
use rusqlite::{params, params_from_iter, OptionalExtension, Row};

use crate::store::SqliteStore;

/// Column list shared by `list` and `get`, in [`StoredCandidate`] order.
pub(crate) const CANDIDATE_COLUMNS: &str =
    "dc.id, dc.project_id, p.location, dc.capture_id, dc.status, \
     dc.question, dc.choice, dc.rationale, dc.signals, dc.confidence, dc.confidence_reason, \
     dc.evidence_refs, dc.diff_summary, ck.adapter, ck.session_id, ck.observed_at, r.received_at, \
     dc.created_at, dc.updated_at, dc.kind, dc.significance, dc.criteria, dc.qualifiers";

/// Join that attaches the project, the receipt and the latest checkpoint.
pub(crate) const CANDIDATE_FROM: &str = "FROM decision_candidates dc \
     JOIN projects p ON p.id = dc.project_id \
     JOIN capture_receipts r ON r.capture_id = dc.capture_id \
     LEFT JOIN adapter_checkpoints ck ON ck.rowid = ( \
         SELECT ck2.rowid FROM adapter_checkpoints ck2 \
         WHERE ck2.capture_id = dc.capture_id \
         ORDER BY ck2.updated_at DESC, ck2.rowid DESC LIMIT 1 \
     )";

/// Converts a query failure into a storage error carried by the use-case error.
fn storage_error(error: rusqlite::Error) -> InboxError {
    InboxError::Storage(error.to_string())
}

/// Maps a joined candidate row into a [`StoredCandidate`].
pub(crate) fn map_row(row: &Row<'_>) -> rusqlite::Result<StoredCandidate> {
    let status_text: String = row.get(4)?;
    let status = CandidateStatus::parse(&status_text).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            4,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "estado de candidato desconhecido",
            )),
        )
    })?;
    let received_at: String = row.get(16)?;
    let observed_at: Option<String> = row.get(15)?;
    Ok(StoredCandidate {
        qualifiers: row.get(22)?,
        id: row.get(0)?,
        project_id: row.get(1)?,
        project_location: row.get(2)?,
        capture_id: row.get(3)?,
        status,
        question: row.get(5)?,
        choice: row.get(6)?,
        rationale: row.get(7)?,
        signals: row.get(8)?,
        confidence: row.get(9)?,
        confidence_reason: row.get(10)?,
        evidence_refs: row.get(11)?,
        diff_summary: row.get(12)?,
        adapter: row.get(13)?,
        session_id: row.get(14)?,
        observed_at: observed_at.or_else(|| Some(received_at.clone())),
        received_at,
        created_at: row.get(17)?,
        updated_at: row.get(18)?,
        kind: row.get(19)?,
        significance: row.get(20)?,
        criteria: row.get(21)?,
    })
}

/// Compare-and-set one status change with a fixed `to` and allowed sources.
fn cas(
    store: &SqliteStore,
    id: &str,
    allowed_from: &[CandidateStatus],
    to: CandidateStatus,
    edits: Option<&ValidatedEdits>,
    updated_at: &str,
) -> Result<bool, InboxError> {
    let mut sql = String::from("UPDATE decision_candidates SET status = ?, updated_at = ?");
    let mut binds: Vec<Value> = vec![
        Value::Text(to.as_str().to_string()),
        Value::Text(updated_at.to_string()),
    ];
    if let Some(edits) = edits {
        sql.push_str(", question = ?, choice = ?, rationale = ?");
        binds.push(Value::Text(edits.question().to_string()));
        binds.push(Value::Text(edits.choice().to_string()));
        binds.push(Value::Text(edits.rationale().to_string()));
    }
    sql.push_str(" WHERE id = ? AND status IN (");
    push_placeholders(&mut sql, allowed_from.len());
    sql.push(')');
    binds.push(Value::Text(id.to_string()));
    for status in allowed_from {
        binds.push(Value::Text(status.as_str().to_string()));
    }
    let changed = store
        .lock()
        .execute(&sql, params_from_iter(binds))
        .map_err(storage_error)?;
    Ok(changed > 0)
}

/// Compare-and-set many status changes with a fixed `to` and allowed sources.
fn cas_batch(
    store: &SqliteStore,
    ids: &[String],
    allowed_from: &[CandidateStatus],
    to: CandidateStatus,
    updated_at: &str,
) -> Result<usize, InboxError> {
    if ids.is_empty() || allowed_from.is_empty() {
        return Ok(0);
    }
    let mut sql =
        String::from("UPDATE decision_candidates SET status = ?, updated_at = ? WHERE id IN (");
    push_placeholders(&mut sql, ids.len());
    sql.push_str(") AND status IN (");
    push_placeholders(&mut sql, allowed_from.len());
    sql.push(')');
    let mut binds: Vec<Value> = vec![
        Value::Text(to.as_str().to_string()),
        Value::Text(updated_at.to_string()),
    ];
    for id in ids {
        binds.push(Value::Text(id.clone()));
    }
    for status in allowed_from {
        binds.push(Value::Text(status.as_str().to_string()));
    }
    let changed = store
        .lock()
        .execute(&sql, params_from_iter(binds))
        .map_err(storage_error)?;
    Ok(changed)
}

impl InboxStore for SqliteStore {
    fn eligible_in_review(
        &self,
        project_id: &str,
        candidate_id: &str,
        query: &InboxQuery,
    ) -> Result<bool, InboxError> {
        let mut connection = self.lock();
        let transaction = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        let result = crate::review_exception::eligible_in_review(
            &transaction,
            project_id,
            candidate_id,
            query,
        )?;
        transaction.commit().map_err(storage_error)?;
        Ok(result)
    }
    fn briefing(&self, project_id: &str, since: &str) -> Result<Briefing, InboxError> {
        let (new_candidates, decisions, deliveries, sessions): (i64, i64, i64, i64) = self
            .lock()
            .query_row(
                "SELECT                  (SELECT COUNT(*) FROM decision_candidates                     WHERE project_id = ?1 AND created_at > ?2),                  (SELECT COUNT(*) FROM engineering_decisions                     WHERE project_id = ?1 AND confirmed_at > ?2),                  (SELECT COUNT(*) FROM context_injections                     WHERE project_id = ?1 AND mode = 'inject' AND created_at > ?2),                  (SELECT COUNT(DISTINCT session_id) FROM context_injections                     WHERE project_id = ?1 AND mode = 'inject' AND created_at > ?2)",
                rusqlite::params![project_id, since],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .map_err(storage_error)?;
        let count = |value: i64| usize::try_from(value).unwrap_or(0);
        Ok(Briefing {
            since: since.to_owned(),
            new_candidates: count(new_candidates),
            decisions: count(decisions),
            deliveries: count(deliveries),
            sessions: count(sessions),
        })
    }

    fn count(
        &self,
        project_id: Option<&str>,
        statuses: &[CandidateStatus],
        min_significance: Option<f64>,
    ) -> Result<usize, InboxError> {
        if statuses.is_empty() {
            return Ok(0);
        }
        let query = InboxQuery {
            project_id: project_id.map(str::to_owned),
            statuses: statuses.to_vec(),
            min_significance,
            limit: usize::MAX,
            before: None,
        };
        Ok(self.list(&query)?.len())
    }

    fn list(&self, query: &InboxQuery) -> Result<Vec<StoredCandidate>, InboxError> {
        if query.statuses.is_empty() {
            return Ok(Vec::new());
        }
        let mut connection = self.lock();
        let transaction = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        let result = crate::review_exception::projection(&transaction, query)?;
        transaction.commit().map_err(storage_error)?;
        Ok(result)
    }

    fn get(&self, id: &str) -> Result<Option<StoredCandidate>, InboxError> {
        self.lock()
            .query_row(
                &format!("SELECT {CANDIDATE_COLUMNS} {CANDIDATE_FROM} WHERE dc.id = ?1"),
                [id],
                map_row,
            )
            .optional()
            .map_err(storage_error)
    }

    fn artifacts(
        &self,
        capture_id: &str,
        refs: &[String],
    ) -> Result<Vec<ArtifactView>, InboxError> {
        if refs.is_empty() {
            return Ok(Vec::new());
        }
        let connection = self.lock();
        let mut sql = String::from(
            "SELECT artifact_id, kind, content, metadata FROM capture_artifacts \
             WHERE capture_id = ?1 AND artifact_id IN (",
        );
        push_placeholders(&mut sql, refs.len());
        sql.push(')');
        let mut binds: Vec<Value> = Vec::with_capacity(refs.len() + 1);
        binds.push(Value::Text(capture_id.to_string()));
        for reference in refs {
            binds.push(Value::Text(reference.clone()));
        }
        let mut statement = connection.prepare(&sql).map_err(storage_error)?;
        let found = statement
            .query_map(params_from_iter(binds), |row| {
                Ok(ArtifactView {
                    artifact_id: row.get(0)?,
                    kind: row.get(1)?,
                    content: row.get(2)?,
                    metadata: row.get(3)?,
                })
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;

        let mut index: HashMap<String, ArtifactView> = found
            .into_iter()
            .map(|artifact| (artifact.artifact_id.clone(), artifact))
            .collect();
        let mut ordered = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for reference in refs {
            if !seen.insert(reference.clone()) {
                continue;
            }
            if let Some(artifact) = index.remove(reference) {
                ordered.push(artifact);
            }
        }
        Ok(ordered)
    }

    fn confirm_one(
        &self,
        id: &str,
        expected: &StoredCandidate,
        edits: Option<&ValidatedEdits>,
        seed: &DecisionSeed,
        updated_at: &str,
    ) -> Result<bool, InboxError> {
        let to = if edits.is_some() {
            CandidateStatus::EditedAndAccepted
        } else {
            CandidateStatus::Accepted
        };

        let mut connection = self.lock();
        let transaction = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;

        crate::review_exception::reconcile(&transaction)?;
        if crate::extraction::nature_destination(&transaction, id)?
            != application::review_exception::NatureDestination::ReviewRequired
        {
            return Ok(false);
        }
        if crate::review_exception::represented(&transaction, expected)? {
            return Ok(false);
        }

        let mut sql = String::from("UPDATE decision_candidates SET status = ?, updated_at = ?");
        let mut binds: Vec<Value> = vec![
            Value::Text(to.as_str().to_string()),
            Value::Text(updated_at.to_string()),
        ];
        if let Some(edits) = edits {
            sql.push_str(", question = ?, choice = ?, rationale = ?, qualifiers = ?");
            binds.push(Value::Text(edits.question().to_string()));
            binds.push(Value::Text(edits.choice().to_string()));
            binds.push(Value::Text(edits.rationale().to_string()));
            binds.push(Value::Text(edits.qualifiers_json()));
        }
        sql.push_str(
            " WHERE id = ? AND status IN (?, ?) AND status = ? \
            AND question = ? AND choice = ? AND rationale = ? \
            AND evidence_refs = ? AND updated_at = ? AND project_id = ? AND capture_id = ? AND qualifiers = ?",
        );
        binds.push(Value::Text(id.to_string()));
        binds.push(Value::Text(CandidateStatus::Pending.as_str().to_string()));
        binds.push(Value::Text(CandidateStatus::Snoozed.as_str().to_string()));
        binds.extend([
            Value::Text(expected.status.as_str().to_string()),
            Value::Text(expected.question.clone()),
            Value::Text(expected.choice.clone()),
            Value::Text(expected.rationale.clone()),
            Value::Text(expected.evidence_refs.clone()),
            Value::Text(expected.updated_at.clone()),
            Value::Text(expected.project_id.clone()),
            Value::Text(expected.capture_id.clone()),
            Value::Text(expected.qualifiers.clone()),
        ]);
        let changed = transaction
            .execute(&sql, params_from_iter(binds))
            .map_err(storage_error)?;
        if changed == 0 {
            return Ok(false);
        }

        if seed.as_rule {
            // A rule becomes a project claim, valid from now, citing no decision.
            transaction
                .execute(
                    "INSERT INTO context_claims \
                     (claim_id, project_id, kind, statement, valid_from, valid_until, \
                      source_decision_id, created_at, updated_at, source_candidate_id, qualifiers) \
                     VALUES (?1, ?2, 'constraint', ?3, ?4, NULL, NULL, ?4, ?4, ?5, ?6)",
                    params![
                        seed.decision_id,
                        seed.project_id,
                        seed.choice,
                        updated_at,
                        id,
                        seed.qualifiers
                    ],
                )
                .map_err(storage_error)?;
            transaction
                .execute(
                    "INSERT INTO claims_fts (claim_id, statement) VALUES (?1, ?2)",
                    params![seed.decision_id, seed.choice],
                )
                .map_err(storage_error)?;
            crate::review_exception::record_confirmation(
                &transaction,
                expected,
                &seed.decision_id,
                true,
                updated_at,
            )?;
            transaction.commit().map_err(storage_error)?;
            return Ok(true);
        }

        transaction
            .execute(
                "INSERT INTO engineering_decisions \
                 (decision_id, candidate_id, project_id, capture_id, status, question, choice, \
                  rationale, assumptions, reconsider_when, scope, consequences, created_at, \
                  confirmed_at, updated_at, version, qualifiers) \
                 VALUES (?1, ?2, ?3, ?4, 'accepted', ?5, ?6, ?7, '[]', '[]', '[]', '[]', ?8, ?8, ?8, 1, ?9)",
                params![
                    seed.decision_id,
                    id,
                    seed.project_id,
                    seed.capture_id,
                    seed.question,
                    seed.choice,
                    seed.rationale,
                    updated_at,
                    seed.qualifiers,
                ],
            )
            .map_err(storage_error)?;

        transaction
            .execute(
                "INSERT INTO decision_revisions \
                 (decision_id, version, question, choice, rationale, assumptions, reconsider_when, \
                  scope, consequences, created_at, qualifiers) \
                 VALUES (?1, 1, ?2, ?3, ?4, '[]', '[]', '[]', '[]', ?5, ?6)",
                params![
                    seed.decision_id,
                    seed.question,
                    seed.choice,
                    seed.rationale,
                    updated_at,
                    seed.qualifiers,
                ],
            )
            .map_err(storage_error)?;

        let mut position = 0i64;
        for artifact_id in &seed.evidence_refs {
            let inserted = transaction
                .execute(
                    "INSERT OR IGNORE INTO evidence_links \
                     (decision_id, artifact_id, position, created_at) VALUES (?1, ?2, ?3, ?4)",
                    params![seed.decision_id, artifact_id, position, updated_at],
                )
                .map_err(storage_error)?;
            if inserted > 0 {
                position += 1;
            }
        }

        transaction
            .execute(
                "INSERT INTO decisions_fts (decision_id, question, choice, rationale) \
                 VALUES (?1, ?2, ?3, ?4)",
                params![seed.decision_id, seed.question, seed.choice, seed.rationale],
            )
            .map_err(storage_error)?;

        crate::review_exception::record_confirmation(
            &transaction,
            expected,
            &seed.decision_id,
            false,
            updated_at,
        )?;
        transaction.commit().map_err(storage_error)?;
        Ok(true)
    }

    fn adjust_one(
        &self,
        id: &str,
        edits: &ValidatedEdits,
        updated_at: &str,
    ) -> Result<bool, InboxError> {
        let mut guard = self.lock();
        let transaction = guard
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        let changed = transaction
            .execute(
                "UPDATE decision_candidates \
                 SET question = ?1, choice = ?2, rationale = ?3, updated_at = ?4, qualifiers = ?8 \
                 WHERE id = ?5 AND status IN (?6, ?7)",
                rusqlite::params![
                    edits.question(),
                    edits.choice(),
                    edits.rationale(),
                    updated_at,
                    id,
                    CandidateStatus::Pending.as_str(),
                    CandidateStatus::Snoozed.as_str(),
                    edits.qualifiers_json(),
                ],
            )
            .map_err(storage_error)?;
        if changed > 0 {
            transaction
                .execute(
                    "UPDATE candidate_nature SET nature='unknown',
                destination='review_required',observation_reference=NULL WHERE candidate_id=?1",
                    [id],
                )
                .map_err(storage_error)?;
        }
        transaction.commit().map_err(storage_error)?;
        Ok(changed > 0)
    }

    fn dismiss_one(&self, id: &str, updated_at: &str) -> Result<bool, InboxError> {
        cas(
            self,
            id,
            &[CandidateStatus::Pending, CandidateStatus::Snoozed],
            CandidateStatus::Dismissed,
            None,
            updated_at,
        )
    }

    fn dismiss_batch(&self, ids: &[String], updated_at: &str) -> Result<usize, InboxError> {
        cas_batch(
            self,
            ids,
            &[CandidateStatus::Pending, CandidateStatus::Snoozed],
            CandidateStatus::Dismissed,
            updated_at,
        )
    }

    fn snooze_one(&self, id: &str, updated_at: &str) -> Result<bool, InboxError> {
        cas(
            self,
            id,
            &[CandidateStatus::Pending],
            CandidateStatus::Snoozed,
            None,
            updated_at,
        )
    }

    fn snooze_batch(&self, ids: &[String], updated_at: &str) -> Result<usize, InboxError> {
        cas_batch(
            self,
            ids,
            &[CandidateStatus::Pending],
            CandidateStatus::Snoozed,
            updated_at,
        )
    }

    fn unsnooze_one(&self, id: &str, updated_at: &str) -> Result<bool, InboxError> {
        cas(
            self,
            id,
            &[CandidateStatus::Snoozed],
            CandidateStatus::Pending,
            None,
            updated_at,
        )
    }

    fn reopen_one(&self, id: &str, updated_at: &str) -> Result<bool, InboxError> {
        cas(
            self,
            id,
            &[CandidateStatus::Dismissed],
            CandidateStatus::Pending,
            None,
            updated_at,
        )
    }
}

/// Appends `count` comma-separated `?` placeholders to `sql`.
fn push_placeholders(sql: &mut String, count: usize) {
    for index in 0..count {
        if index > 0 {
            sql.push_str(", ");
        }
        sql.push('?');
    }
}
