//! Atomic local queue/cache and publication CAS for optional context routing.
use application::context_routing::{
    fingerprint as artifact_fingerprint, RoutingJudgment, RoutingRequest, RoutingStore,
    RoutingWork, CONTEXT_ROUTING_KIND, COOLDOWN_SECONDS, MAX_DAILY_CALLS, MAX_ENTRIES, MAX_PENDING,
    TTL_SECONDS,
};
use domain::time::Timestamp;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::json;

use crate::{knowledge_review::snapshot_on, SqliteStore};

fn error(_: impl std::fmt::Display) -> String {
    "context routing storage unavailable".into()
}

// One connection/transaction captures the full normative inventory, project root,
// eligible source semantics and dirty identity. Pending proposals are not memory.
fn snapshot(c: &Connection, project: &str) -> Result<Option<(String, i64, bool)>, String> {
    let Some(review) = snapshot_on(c, project).map_err(error)? else {
        return Ok(None);
    };
    let root: String = c
        .query_row(
            "SELECT location FROM projects WHERE id=?1",
            [project],
            |r| r.get(0),
        )
        .map_err(error)?;
    let now: String = c
        .query_row("SELECT strftime('%Y-%m-%dT%H:%M:%SZ','now')", [], |r| {
            r.get(0)
        })
        .map_err(error)?;
    let at = Timestamp::parse(&now).ok_or_else(|| error("time"))?;
    // Debug serializes the complete typed records, including raw updated/source
    // metadata. No claim version is manufactured. Ordering is deterministic.
    let validity: Vec<_> = review
        .claims
        .iter()
        .map(|claim| (&claim.claim_id, claim.is_valid_at(&at)))
        .collect();
    let normative = format!(
        "{:?}{:?}{:?}{:?}{:?}{validity:?}",
        review.decisions, review.claims, review.relations, review.entities, review.edges
    );
    let mut stmt = c
        .prepare(
            "SELECT source_id,project_relative_path,manifest_kind,sha256,
        parser_policy_version,last_check_status FROM observation_sources
        WHERE project_id=?1 ORDER BY source_id",
        )
        .map_err(error)?;
    let sources = stmt
        .query_map([project], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, String>(5)?,
            ))
        })
        .map_err(error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(error)?;
    let mut stmt = c
        .prepare(
            "SELECT record_json FROM observation_records
        WHERE project_id=?1 AND status='current' ORDER BY observation_id,version",
        )
        .map_err(error)?;
    let records = stmt
        .query_map([project], |r| r.get::<_, String>(0))
        .map_err(error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(error)?;
    let records = records
        .iter()
        .map(|encoded| {
            let record: application::observations::ObservationRecord =
                serde_json::from_str(encoded).map_err(error)?;
            let eligible = sources.iter().any(|s| {
                s.0 == record.provenance.source_id
                    && s.3.as_deref() == Some(record.provenance.source_sha256.as_str())
                    && s.4 == record.provenance.parser_policy_version
                    && s.5 == "\"verified\""
            }) && record.provenance.supporting_sources.iter().all(|support| {
                sources.iter().any(|s| {
                    s.0 == support.source_id
                        && s.3.as_deref() == Some(support.source_sha256.as_str())
                        && s.5 == "\"verified\""
                })
            });
            // observed_at/check time and capture trigger are audit, not semantic identity.
            Ok(json!({"id":record.observation_id,"version":record.version,
            "subject":record.subject,"value":record.value,"scope":record.path_scope,
            "source":record.provenance.source_id,"hash":record.provenance.source_sha256,
            "policy":record.provenance.parser_policy_version,
            "supports":record.provenance.supporting_sources,"field":record.provenance.field_pointer,
            "eligible":eligible}))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let dirty: (i64, bool) = c
        .query_row(
            "SELECT generation,dirty FROM observation_refresh WHERE project_id=?1",
            [project],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(error)?
        .unwrap_or_default();
    Ok(Some((
        artifact_fingerprint(&format!(
            "context-routing-v1{root}{normative}{sources:?}{records:?}"
        )),
        dirty.0,
        dirty.1,
    )))
}

fn seconds(c: &Connection) -> Result<i64, String> {
    c.query_row("SELECT CAST(strftime('%s','now') AS INTEGER)", [], |r| {
        r.get(0)
    })
    .map_err(error)
}

// Prevent a query assembled across earlier reads from attaching stale candidate
// content to a newer project snapshot. Protection happens before comparison.
fn candidates_current(c: &Connection, request: &RoutingRequest) -> Result<bool, String> {
    let Some(review) = snapshot_on(c, &request.project_id).map_err(error)? else {
        return Ok(false);
    };
    Ok(request.candidates.iter().all(|candidate| {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&candidate.content) else {
            return false;
        };
        let protected =
            |text: &str| application::external::protected_text(&json!(text).to_string());
        let matches = |field: &str, text: &str| {
            value.get(field).map(|v| v.to_string()) == Some(protected(text))
        };
        let matches_json = |field: &str, encoded: &str| {
            serde_json::from_str::<serde_json::Value>(encoded)
                .ok()
                .is_some_and(|v| {
                    value.get(field).map(|item| item.to_string())
                        == Some(application::external::protected_text(&v.to_string()))
                })
        };
        match candidate.kind.as_str() {
            "decision" => review.decisions.iter().any(|d| {
                d.decision_id == candidate.id
                    && value["version"].as_i64() == Some(d.version)
                    && matches("question", &d.question)
                    && matches("choice", &d.choice)
                    && matches("rationale", &d.rationale)
                    && matches("confirmed_at", &d.confirmed_at)
                    && matches_json("scope", &d.scope)
                    && matches_json("qualifiers", &d.qualifiers)
            }),
            "claim" => review.claims.iter().any(|claim| {
                claim.claim_id == candidate.id
                    && matches("statement", &claim.statement)
                    && matches("valid_from", &claim.valid_from)
                    && value["valid_until"] == json!(claim.valid_until)
                    && value["source_decision_id"] == json!(claim.source_decision_id)
                    && value["source_version"] == json!(claim.source_version)
                    && matches_json("inherited_scope", &claim.inherited_scope)
                    && matches_json("qualifiers", &claim.qualifiers)
            }),
            _ => false,
        }
    }))
}

impl RoutingStore for SqliteStore {
    fn lookup(&self, request: &RoutingRequest) -> Result<Option<Vec<RoutingJudgment>>, String> {
        let mut c = self.lock();
        let tx = c
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(error)?;
        let now = seconds(&tx)?;
        tx.execute(
            "UPDATE context_routing_entries SET state='failed',request_json=NULL,result_json=NULL,
            expires_at=MIN(expires_at,?2+60)
            WHERE project_id=?1 AND state IN ('pending','running') AND (expires_at<=?2 OR
            NOT EXISTS (SELECT 1 FROM jobs WHERE kind='context_routing' AND id=context_routing_entries.owner_job_id
            AND state IN ('queued','running')))",
            params![request.project_id, now],
        )
        .map_err(error)?;
        let Some((fingerprint, generation, dirty)) = snapshot(&tx, &request.project_id)? else {
            tx.commit().map_err(error)?;
            return Ok(None);
        };
        if dirty || !candidates_current(&tx, request)? {
            tx.commit().map_err(error)?;
            return Ok(None);
        }
        let serialized = serde_json::to_string(request).map_err(error)?;
        let key = artifact_fingerprint(&format!("{fingerprint}:{serialized}"));
        // Original v28 included generation in the key. Terminal payloads were
        // erased, so reconstruct only from this incoming exact protected request
        // and the row's own generation; never infer task/config from a project.
        let legacy = {
            let mut stmt = tx
                .prepare(
                    "SELECT key,generation,result_json FROM context_routing_entries
                WHERE project_id=?1 AND snapshot=?2 AND profile_hash=?3
                AND state='completed' AND expires_at>?4 AND result_json IS NOT NULL",
                )
                .map_err(error)?;
            let rows = stmt
                .query_map(
                    params![request.project_id, fingerprint, request.profile_hash, now],
                    |r| {
                        Ok((
                            r.get::<_, String>(0)?,
                            r.get::<_, i64>(1)?,
                            r.get::<_, String>(2)?,
                        ))
                    },
                )
                .map_err(error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(error)?;
            rows.into_iter().find(|(old, old_generation, _)| {
                *old == artifact_fingerprint(&format!(
                    "{fingerprint}:{old_generation}:{serialized}"
                ))
            })
        };
        if let Some((old, _, encoded)) = legacy {
            if let Ok(judgments) = serde_json::from_str::<Vec<RoutingJudgment>>(&encoded) {
                if application::context_routing::valid_judgments(request, &judgments) {
                    tx.execute(
                        "UPDATE OR IGNORE context_routing_entries SET key=?2 WHERE key=?1",
                        params![old, key],
                    )
                    .map_err(error)?;
                    tx.commit().map_err(error)?;
                    return Ok(Some(judgments));
                }
            }
        }
        let result: Option<(Option<String>, String)> = tx.query_row(
            "SELECT result_json,state FROM context_routing_entries WHERE key=?1 AND expires_at>?2",
            params![key, now], |r| Ok((r.get(0)?,r.get(1)?))).optional().map_err(error)?;
        if let Some((result, _state)) = result {
            tx.commit().map_err(error)?;
            return result
                .map(|v| serde_json::from_str(&v).map_err(error))
                .transpose();
        }
        // Expired terminal entries can be reclaimed, but daily accounting lives 24h.
        tx.execute(
            "DELETE FROM context_routing_entries WHERE project_id=?1
            AND created_at<=?2 AND state IN ('completed','failed')",
            params![request.project_id, now - 86_400],
        )
        .map_err(error)?;
        let (pending, recent, daily, retained): (i64, i64, i64, i64) = tx
            .query_row(
                "SELECT SUM(CASE WHEN state IN ('pending','running') THEN 1 ELSE 0 END),
            SUM(CASE WHEN created_at>?2 THEN 1 ELSE 0 END),
             SUM(CASE WHEN created_at>?3 AND (state IN ('pending','running') OR logical_invocations=1) THEN 1 ELSE 0 END),COUNT(*)
            FROM context_routing_entries WHERE project_id=?1",
                params![request.project_id, now - COOLDOWN_SECONDS, now - 86_400],
                |r| {
                    Ok((
                        r.get::<_, Option<i64>>(0)?.unwrap_or(0),
                        r.get::<_, Option<i64>>(1)?.unwrap_or(0),
                        r.get::<_, Option<i64>>(2)?.unwrap_or(0),
                        r.get(3)?,
                    ))
                },
            )
            .map_err(error)?;
        if pending >= MAX_PENDING as i64
            || recent > 0
            || daily >= MAX_DAILY_CALLS as i64
            || retained >= MAX_ENTRIES as i64
        {
            tx.commit().map_err(error)?;
            return Ok(None);
        }
        // Preserve the old accounting row under a retired identity before renewing
        // the semantic key. Its terminal job cannot own the new incarnation.
        tx.execute(
            "UPDATE context_routing_entries SET key=key || ':retired:' || created_at
            WHERE key=?1 AND expires_at<=?2",
            params![key, now],
        )
        .map_err(error)?;
        tx.execute("INSERT OR IGNORE INTO context_routing_entries
            (key,project_id,snapshot,generation,profile_hash,request_json,state,created_at,expires_at)
            VALUES (?1,?2,?3,?4,?5,?6,'pending',?7,?8)",
            params![key,request.project_id,fingerprint,generation,request.profile_hash,
                serialized,now,now + TTL_SECONDS]).map_err(error)?;
        let nonce: String = tx
            .query_row("SELECT lower(hex(randomblob(16)))", [], |r| r.get(0))
            .map_err(error)?;
        let job_id = format!("context-routing-{key}:{nonce}");
        tx.execute(
            "UPDATE context_routing_entries SET owner_job_id=?2 WHERE key=?1",
            params![key, job_id],
        )
        .map_err(error)?;
        tx.execute(
            "INSERT INTO jobs (id,kind,payload,state,idempotent,attempts,created_at,updated_at)
            VALUES (?1,?2,?3,'queued',0,0,strftime('%Y-%m-%dT%H:%M:%SZ','now'),
            strftime('%Y-%m-%dT%H:%M:%SZ','now'))",
            params![job_id, CONTEXT_ROUTING_KIND, request.project_id],
        )
        .map_err(error)?;
        tx.commit().map_err(error)?;
        Ok(None)
    }

    fn begin(&self, job_id: &str, profile_hash: &str) -> Result<Option<RoutingWork>, String> {
        let mut c = self.lock();
        let tx = c
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(error)?;
        let row: Option<(String, String, i64, String, String, i64)> = tx
            .query_row(
                "SELECT key,snapshot,generation,request_json,profile_hash,expires_at
             FROM context_routing_entries WHERE owner_job_id=?1 AND state='pending'
             AND EXISTS (SELECT 1 FROM jobs WHERE id=?1 AND kind='context_routing'
             AND payload=context_routing_entries.project_id AND state IN ('queued','running'))",
                [job_id],
                |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                    ))
                },
            )
            .optional()
            .map_err(error)?;
        let Some((key, fingerprint, generation, encoded, expected_profile, expires)) = row else {
            return Ok(None);
        };
        let request: RoutingRequest = serde_json::from_str(&encoded).map_err(error)?;
        let current = snapshot(&tx, &request.project_id)?;
        if current != Some((fingerprint.clone(), generation, false))
            || expected_profile != profile_hash
            || expires <= seconds(&tx)?
        {
            tx.execute(
                "UPDATE context_routing_entries SET state='failed',request_json=NULL,
                result_json=NULL WHERE key=?1",
                [key],
            )
            .map_err(error)?;
            tx.commit().map_err(error)?;
            return Ok(None);
        }
        let chars =
            json!({"task":request.task,"files":request.files,"candidates":request.candidates})
                .to_string()
                .chars()
                .count();
        tx.execute(
            "UPDATE context_routing_entries SET state='running',
            input_chars=?2,estimated_tokens=?3 WHERE key=?1 AND logical_invocations=0",
            params![key, chars as i64, chars.div_ceil(4) as i64],
        )
        .map_err(error)?;
        tx.commit().map_err(error)?;
        Ok(Some(RoutingWork {
            key,
            snapshot: fingerprint,
            generation,
            request,
        }))
    }

    fn record_call(&self, work: &RoutingWork) -> Result<bool, String> {
        let mut c = self.lock();
        let tx = c
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(error)?;
        if snapshot(&tx, &work.request.project_id)?
            != Some((work.snapshot.clone(), work.generation, false))
        {
            return Ok(false);
        }
        let changed = tx
            .execute(
                "UPDATE context_routing_entries SET logical_invocations=1
            WHERE key=?1 AND state='running' AND logical_invocations=0 AND expires_at>?2",
                params![work.key, seconds(&tx)?],
            )
            .map_err(error)?;
        tx.commit().map_err(error)?;
        Ok(changed == 1)
    }

    fn finish(
        &self,
        work: &RoutingWork,
        result: Option<&[RoutingJudgment]>,
        authorized: &dyn Fn() -> bool,
    ) -> Result<(), String> {
        let mut c = self.lock();
        let tx = c
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(error)?;
        let current = snapshot(&tx, &work.request.project_id)?;
        let result =
            if current == Some((work.snapshot.clone(), work.generation, false)) && authorized() {
                result
                    .map(serde_json::to_string)
                    .transpose()
                    .map_err(error)?
            } else {
                None
            };
        tx.execute(
            "UPDATE context_routing_entries SET state=?2,request_json=NULL,result_json=?3,
            expires_at=CASE WHEN ?2='failed' THEN ?4+60 ELSE expires_at END
            WHERE key=?1 AND state='running' AND expires_at>?4",
            params![
                work.key,
                if result.is_some() {
                    "completed"
                } else {
                    "failed"
                },
                result,
                seconds(&tx)?
            ],
        )
        .map_err(error)?;
        // Also erase payload on an expired in-flight operation.
        tx.execute(
            "UPDATE context_routing_entries SET state='failed',request_json=NULL,result_json=NULL
            WHERE key=?1 AND state='running'",
            [&work.key],
        )
        .map_err(error)?;
        tx.commit().map_err(error)?;
        Ok(())
    }
}
