//! Current-source group projection and transactionally recorded lateral results.

use application::inbox::{CandidateStatus, InboxError, InboxQuery, StoredCandidate};
use application::review_exception::{
    group_fingerprint, source_fingerprint, ReviewExceptionStore, ReviewGroup, ReviewMember,
    ReviewMetrics,
};
use rusqlite::{params, Connection, OptionalExtension};

use crate::inbox::{map_row, CANDIDATE_COLUMNS, CANDIDATE_FROM};
use crate::SqliteStore;

const VALID_TARGET: &str = "((j.target_kind='decision' AND EXISTS
    (SELECT 1 FROM engineering_decisions d WHERE d.decision_id=j.target_id
     AND CAST(d.version AS TEXT)=j.target_version AND d.status='accepted'
     AND d.project_id=?4)) OR (j.target_kind='claim' AND EXISTS
    (SELECT 1 FROM context_claims c WHERE c.claim_id=j.target_id
     AND c.updated_at=j.target_version AND c.valid_until IS NULL AND c.project_id=?4)))";

fn error(e: rusqlite::Error) -> InboxError {
    InboxError::Storage(e.to_string())
}

pub(crate) fn represented(
    connection: &Connection,
    row: &StoredCandidate,
) -> Result<bool, InboxError> {
    let source = source_fingerprint(row);
    let fingerprint = group_fingerprint(row)?;
    connection
        .query_row(
            &format!(
                "SELECT 1 FROM review_target_resolutions rr JOIN review_targets j USING(process_id)
         JOIN review_group_members m ON m.candidate_id = j.representative_id
         WHERE rr.candidate_id = ?1 AND rr.source_fingerprint = ?2
          AND j.fingerprint = ?3 AND {VALID_TARGET}
          AND m.source_fingerprint = j.representative_source"
            ),
            params![row.id, source, fingerprint, row.project_id],
            |_| Ok(true),
        )
        .optional()
        .map(|v| v.unwrap_or(false))
        .map_err(error)
}

fn index(connection: &Connection, rows: &[StoredCandidate]) -> Result<(), InboxError> {
    for row in rows {
        let fingerprint = group_fingerprint(row)?;
        connection
            .execute(
                "INSERT OR IGNORE INTO review_groups VALUES (?1, ?2)",
                params![fingerprint, row.project_id],
            )
            .map_err(error)?;
        connection
            .execute(
                "INSERT INTO review_group_members(candidate_id,fingerprint,source_fingerprint,review_required)
                 VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(candidate_id) DO UPDATE SET fingerprint=excluded.fingerprint,
              source_fingerprint=excluded.source_fingerprint,review_required=excluded.review_required",
                params![row.id, fingerprint, source_fingerprint(row),
                    crate::extraction::nature_destination(connection, &row.id)? ==
                        application::review_exception::NatureDestination::ReviewRequired],
            )
            .map_err(error)?;
    }
    for row in rows.iter().filter(|r| r.status == CandidateStatus::Pending) {
        let fingerprint = group_fingerprint(row)?;
        // New occurrences may reference an existing exact revision. Previously
        // resolved sources are never refreshed after edits by this projection.
        connection
            .execute(
                &format!(
                    "INSERT OR IGNORE INTO review_target_resolutions
              SELECT ?1, j.process_id, ?2 FROM review_targets j
             JOIN review_group_members m ON m.candidate_id = j.representative_id
              WHERE j.fingerprint = ?3 AND {VALID_TARGET}
             AND m.source_fingerprint = j.representative_source
              ORDER BY j.process_id LIMIT 1"
                ),
                params![row.id, source_fingerprint(row), fingerprint, row.project_id],
            )
            .map_err(error)?;
    }
    Ok(())
}

pub(crate) fn reconcile(connection: &Connection) -> Result<(), InboxError> {
    let changed = connection
        .prepare(&format!(
            "SELECT {CANDIDATE_COLUMNS} {CANDIDATE_FROM}
         JOIN review_dirty dirty ON dirty.candidate_id=dc.id"
        ))
        .map_err(error)?
        .query_map([], map_row)
        .map_err(error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(error)?;
    index(connection, &changed)?;
    connection
        .execute("DELETE FROM review_dirty", [])
        .map_err(error)?;
    Ok(())
}

pub(crate) fn projection(
    connection: &Connection,
    query: &InboxQuery,
) -> Result<Vec<StoredCandidate>, InboxError> {
    reconcile(connection)?;
    let target = VALID_TARGET.replace("?4", "dc.project_id");
    let statuses = serde_json::to_string(
        &query
            .statuses
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>(),
    )
    .map_err(|e| InboxError::InvalidData(e.to_string()))?;
    let sql = format!("WITH eligible AS (
        SELECT dc.id,dc.created_at,dc.status,m.fingerprint,
          ROW_NUMBER() OVER (PARTITION BY CASE WHEN dc.status='pending' THEN m.fingerprint
            ELSE dc.id END ORDER BY dc.created_at DESC,dc.id DESC) position
        FROM decision_candidates dc JOIN review_group_members m ON m.candidate_id=dc.id
        WHERE m.review_required=1 AND dc.status IN (SELECT value FROM json_each(?1))
        AND (?2 IS NULL OR dc.project_id=?2) AND (?3 IS NULL OR dc.significance>=?3)
        AND (dc.status!='pending' OR NOT EXISTS (
          SELECT 1 FROM review_target_resolutions rr JOIN review_targets j USING(process_id)
          JOIN review_group_members representative ON representative.candidate_id=j.representative_id
          WHERE rr.candidate_id=dc.id AND rr.source_fingerprint=m.source_fingerprint
          AND j.fingerprint=m.fingerprint AND representative.source_fingerprint=j.representative_source
          AND {target}))
       ) SELECT {CANDIDATE_COLUMNS} {CANDIDATE_FROM}
       JOIN eligible e ON e.id=dc.id WHERE e.position=1
       AND (?4 IS NULL OR dc.created_at<?4 OR (dc.created_at=?4 AND dc.id<?5))
       ORDER BY dc.created_at DESC,dc.id DESC LIMIT ?6");
    connection
        .prepare(&sql)
        .map_err(error)?
        .query_map(
            params![
                statuses,
                query.project_id,
                query.min_significance,
                query.before.as_ref().map(|c| &c.created_at),
                query.before.as_ref().map(|c| &c.id),
                i64::try_from(query.limit).unwrap_or(i64::MAX)
            ],
            map_row,
        )
        .map_err(error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(error)
}

pub(crate) fn eligible_in_review(
    connection: &Connection,
    project_id: &str,
    candidate_id: &str,
    query: &InboxQuery,
) -> Result<bool, InboxError> {
    reconcile(connection)?;
    let statuses = serde_json::to_string(
        &query
            .statuses
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>(),
    )
    .map_err(|e| InboxError::InvalidData(e.to_string()))?;
    let target = VALID_TARGET.replace("?4", "dc.project_id");
    // Scope the representative search to the selected fingerprint, not the
    // whole queue. The validity predicate is identical to list projection.
    let sql = format!("SELECT id=?2 FROM (
        SELECT dc.id FROM decision_candidates dc
        JOIN review_group_members m ON m.candidate_id=dc.id
        WHERE dc.project_id=?1 AND m.review_required=1
        AND dc.status IN (SELECT value FROM json_each(?3))
        AND (?4 IS NULL OR dc.significance>=?4)
        AND m.fingerprint=(SELECT fingerprint FROM review_group_members WHERE candidate_id=?2)
        AND (dc.status!='pending' OR NOT EXISTS(
            SELECT 1 FROM review_target_resolutions rr JOIN review_targets j USING(process_id)
            JOIN review_group_members representative ON representative.candidate_id=j.representative_id
            WHERE rr.candidate_id=dc.id AND rr.source_fingerprint=m.source_fingerprint
            AND j.fingerprint=m.fingerprint AND representative.source_fingerprint=j.representative_source
            AND {target}))
        AND ((dc.status='pending' AND (SELECT status FROM decision_candidates WHERE id=?2)='pending')
            OR (dc.status!='pending' AND dc.id=?2))
        ORDER BY dc.created_at DESC,dc.id DESC LIMIT 1
    )");
    connection
        .query_row(
            &sql,
            params![project_id, candidate_id, statuses, query.min_significance],
            |r| r.get(0),
        )
        .optional()
        .map(|v| v.unwrap_or(false))
        .map_err(error)
}

pub(crate) fn record_confirmation(
    connection: &Connection,
    representative: &StoredCandidate,
    decision_id: &str,
    rule: bool,
    updated_at: &str,
) -> Result<(), InboxError> {
    reconcile(connection)?;
    let fingerprint = group_fingerprint(representative)?;
    let current = connection
        .query_row(
            &format!("SELECT {CANDIDATE_COLUMNS} {CANDIDATE_FROM} WHERE dc.id=?1"),
            [&representative.id],
            map_row,
        )
        .optional()
        .map_err(error)?
        .ok_or(InboxError::NotFound)?;
    // Edited confirmation has a new identity: no old sibling resolution.
    if group_fingerprint(&current)? != fingerprint {
        return Ok(());
    }
    let process_id = format!("confirm:{}", representative.id);
    connection
        .execute(
            "INSERT OR IGNORE INTO review_targets VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7,
          'unknown', 'already_represented')",
            params![
                process_id,
                fingerprint,
                representative.id,
                if rule { "claim" } else { "decision" },
                decision_id,
                if rule { updated_at } else { "1" },
                source_fingerprint(&current)
            ],
        )
        .map_err(error)?;
    connection
        .execute(
            "INSERT INTO review_target_resolutions
        SELECT m.candidate_id,?1,m.source_fingerprint FROM review_group_members m
        JOIN decision_candidates dc ON dc.id=m.candidate_id
        WHERE m.fingerprint=?2 AND dc.status='pending' AND m.review_required=1
        ON CONFLICT(candidate_id) DO UPDATE SET process_id=excluded.process_id,
        source_fingerprint=excluded.source_fingerprint",
            params![process_id, fingerprint],
        )
        .map_err(error)?;
    Ok(())
}

impl ReviewExceptionStore for SqliteStore {
    fn review_group(
        &self,
        candidate_id: &str,
        offset: usize,
        limit: usize,
    ) -> Result<ReviewGroup, InboxError> {
        if limit == 0 || limit > 100 {
            return Err(InboxError::InvalidFilter("limite inválido".into()));
        }
        let mut guard = self.lock();
        let connection = guard
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(error)?;
        reconcile(&connection)?;
        let fingerprint: String = connection
            .query_row(
                "SELECT fingerprint FROM review_group_members WHERE candidate_id=?1",
                [candidate_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(error)?
            .ok_or(InboxError::NotFound)?;
        let occurrence_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM review_group_members WHERE fingerprint=?1",
                [&fingerprint],
                |r| r.get(0),
            )
            .map_err(error)?;
        let matching = connection
            .prepare(&format!(
                "SELECT {CANDIDATE_COLUMNS} {CANDIDATE_FROM}
             JOIN review_group_members m ON m.candidate_id=dc.id WHERE m.fingerprint=?1
             ORDER BY dc.created_at DESC,dc.id DESC LIMIT ?2 OFFSET ?3"
            ))
            .map_err(error)?
            .query_map(
                params![
                    fingerprint,
                    limit as i64,
                    i64::try_from(offset)
                        .map_err(|_| InboxError::InvalidFilter("offset inválido".into()))?
                ],
                map_row,
            )
            .map_err(error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(error)?;
        let members = matching
            .iter()
            .map(|row| {
                Ok(ReviewMember {
                    candidate_id: row.id.clone(),
                    capture_id: row.capture_id.clone(),
                    evidence_refs: serde_json::from_str(&row.evidence_refs)
                        .map_err(|_| InboxError::InvalidData("evidence_refs inválido".into()))?,
                    already_represented: represented(&connection, row)?,
                })
            })
            .collect::<Result<Vec<_>, InboxError>>()?;
        let result = ReviewGroup {
            fingerprint,
            occurrence_count: usize::try_from(occurrence_count)
                .map_err(|_| InboxError::InvalidData("contagem inválida".into()))?,
            members,
        };
        connection.commit().map_err(error)?;
        Ok(result)
    }

    fn review_metrics(&self, project_id: &str) -> Result<ReviewMetrics, InboxError> {
        let mut guard = self.lock();
        let connection = guard
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(error)?;
        let pending_occurrences: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM decision_candidates WHERE project_id=?1 AND status='pending'",
                [project_id],
                |r| r.get(0),
            )
            .map_err(error)?;
        let query = InboxQuery {
            project_id: Some(project_id.into()),
            statuses: vec![CandidateStatus::Pending],
            min_significance: None,
            limit: usize::MAX,
            before: None,
        };
        let result = ReviewMetrics {
            pending_occurrences: usize::try_from(pending_occurrences)
                .map_err(|_| InboxError::InvalidData("contagem inválida".into()))?,
            potential_review_opportunities: projection(&connection, &query)?.len(),
        };
        connection.commit().map_err(error)?;
        Ok(result)
    }
}
