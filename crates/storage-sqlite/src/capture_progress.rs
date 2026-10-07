//! Capture destination projection under one SQLite read transaction.
use crate::store::SqliteStore;
use application::capture_progress::*;
use rusqlite::{params, OptionalExtension};

impl CaptureProgressStore for SqliteStore {
    fn capture_progress(
        &self,
        project: &str,
        capture: Option<&str>,
        limit: usize,
    ) -> Result<Vec<CaptureProgress>, String> {
        let mut connection = self.lock();
        let tx = connection.transaction().map_err(|e| e.to_string())?;
        let mut query = tx
            .prepare(
                "SELECT r.capture_id,r.received_at,j.id,j.state,
            COALESCE(j.attempts,0),COALESCE(j.idempotent,0) FROM capture_receipts r
            JOIN projects p ON p.location=r.canonical_path
            LEFT JOIN jobs j ON j.id=(SELECT id FROM jobs WHERE payload=r.capture_id
                AND kind='analyze_capture' ORDER BY created_at DESC,id DESC LIMIT 1)
            WHERE p.id=?1 AND (?2 IS NULL OR r.capture_id=?2)
            ORDER BY r.received_at DESC,r.capture_id DESC LIMIT ?3",
            )
            .map_err(|e| e.to_string())?;
        let receipts = query
            .query_map(params![project, capture, limit as i64], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, i64>(4)?,
                    r.get::<_, bool>(5)?,
                ))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        drop(query);
        let mut result = Vec::new();
        for (capture_id, received_at, job_id, state, attempts, idempotent) in receipts {
            let state = match state.as_deref() {
                Some("queued") => CaptureState::Queued,
                Some("running") => CaptureState::Running,
                Some("completed") => CaptureState::Completed,
                Some("failed") => CaptureState::Failed,
                Some("cancelled") => CaptureState::Cancelled,
                _ => CaptureState::Unknown,
            };
            let assessment = if matches!(state, CaptureState::Queued | CaptureState::Running) {
                None
            } else {
                tx.query_row(
                    "SELECT reason,adapter,model,durable_count,detail_count,error_detail FROM assessments
                    WHERE capture_id=?1 AND job_id IS ?2 AND attempt=?3
                    ORDER BY finished_at DESC,id DESC LIMIT 1",
                    params![capture_id, job_id, attempts],
                    |r| {
                        Ok((
                            r.get::<_, String>(0)?,
                            r.get::<_, String>(1)?,
                            r.get::<_, Option<String>>(2)?,
                            r.get::<_, i64>(3)?,
                            r.get::<_, i64>(4)?,
                            r.get::<_, Option<String>>(5)?,
                        ))
                    },
                )
                .optional()
                .map_err(|e| e.to_string())?
            };
            // Bounded read: one indexed row, only the head of the prompt.
            let title = tx
                .query_row(
                    "SELECT substr(content,1,400) FROM capture_artifacts
                    WHERE capture_id=?1 AND kind='user_text' ORDER BY artifact_id LIMIT 1",
                    params![capture_id],
                    |r| r.get::<_, String>(0),
                )
                .optional()
                .map_err(|e| e.to_string())?
                .and_then(|text| title_of(&text));
            let adapter = tx
                .query_row(
                    "SELECT json_extract(provenance,'$.adapter') FROM capture_episode_sources
                    WHERE capture_id=?1",
                    params![capture_id],
                    |r| r.get::<_, Option<String>>(0),
                )
                .optional()
                .map_err(|e| e.to_string())?
                .flatten();
            let mut counts = CandidateCounts::default();
            let mut candidates = tx
                .prepare(
                    "SELECT status,significance,COUNT(*) FROM decision_candidates
                WHERE capture_id=?1 AND project_id=?2 GROUP BY status,significance",
                )
                .map_err(|e| e.to_string())?;
            let rows = candidates
                .query_map(params![capture_id, project], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, f64>(1)?,
                        r.get::<_, i64>(2)?,
                    ))
                })
                .map_err(|e| e.to_string())?;
            for row in rows {
                let (status, significance, n) = row.map_err(|e| e.to_string())?;
                match status.as_str() {
                    "pending" if significance < application::extract::MIN_SIGNIFICANCE => {
                        counts.hidden += n as usize
                    }
                    "pending" => counts.pending += n as usize,
                    "accepted" | "edited_and_accepted" => counts.adopted += n as usize,
                    "dismissed" => counts.dismissed += n as usize,
                    "snoozed" => counts.snoozed += n as usize,
                    _ => {}
                }
            }
            let (reason, source, model, durable, detail, failure_detail) = assessment
                .map(|(reason, source, model, durable, detail, failure_detail)| {
                    (
                        match reason.as_str() {
                            "candidates" => AssessmentReason::Candidates,
                            "detail" => AssessmentReason::Detail,
                            "empty" => AssessmentReason::Empty,
                            "failed" => AssessmentReason::Failed,
                            "skipped" => AssessmentReason::Skipped,
                            _ => AssessmentReason::Unknown,
                        },
                        Some(source),
                        model,
                        durable as usize,
                        detail as usize,
                        failure_detail,
                    )
                })
                .unwrap_or((AssessmentReason::Unknown, None, None, 0, 0, None));
            result.push(CaptureProgress {
                project_id: project.into(),
                capture_id,
                received_at,
                title,
                adapter,
                job_id,
                attempts,
                state: if reason == AssessmentReason::Skipped {
                    CaptureState::Skipped
                } else {
                    state
                },
                reason,
                source,
                model,
                durable,
                detail,
                candidates: counts,
                failure_detail,
                can_retry: state == CaptureState::Failed
                    && idempotent
                    && reason != AssessmentReason::Detail,
            });
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(result)
    }
}

/// First non-empty line, ellipsized on a character boundary.
fn title_of(text: &str) -> Option<String> {
    let line = text.lines().map(str::trim).find(|line| !line.is_empty())?;
    if line.chars().count() <= TITLE_MAX_CHARS {
        return Some(line.to_owned());
    }
    let cut: String = line.chars().take(TITLE_MAX_CHARS - 1).collect();
    Some(format!("{}…", cut.trim_end()))
}
