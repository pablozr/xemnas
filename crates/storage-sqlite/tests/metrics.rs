//! Diagnostics metrics against the real database: exact percentiles from seeded
//! timestamps, noise ratio, losses, and tolerant handling of malformed dates.

use std::cell::RefCell;

use application::captures::{
    CaptureArtifactRecord, CaptureCheckpointRecord, CaptureReceiptRecord, CaptureRepository,
    CaptureWrite,
};
use application::diagnostics::Diagnostics;
use application::extract::{
    AssessmentOutcome, AssessmentRecord, AssessmentStore, DecisionCandidateRecord, ExtractionStore,
};
use application::inbox::Inbox;
use application::jobs::{JobRecord, JobRepository, JobState, ANALYZE_CAPTURE_KIND};
use application::profile::{AiProfile, AiSettings, ProfileError, ProfileStore, SecretStore};
use application::projects::{ProjectRecord, ProjectRepository};
use storage_sqlite::SqliteStore;

const LOCATION: &str = "C:/synthetic/metrics";

fn temporary_directory(tag: &str) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let directory = std::env::temp_dir().join(format!(
        "xemnas-metrics-{tag}-{}-{nanos}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir_all(&directory).expect("create temporary directory");
    directory
}

#[derive(Default)]
struct NoProfiles(RefCell<Option<AiProfile>>);

impl ProfileStore for NoProfiles {
    fn load(&self) -> Result<Option<AiProfile>, ProfileError> {
        Ok(self.0.borrow().clone())
    }
    fn save(&self, profile: &AiProfile) -> Result<(), ProfileError> {
        *self.0.borrow_mut() = Some(profile.clone());
        Ok(())
    }
}

#[derive(Default)]
struct NoSecrets;

impl SecretStore for NoSecrets {
    fn set_secret(&self, _account: &str, _secret: &str) -> Result<(), ProfileError> {
        Ok(())
    }
    fn get_secret(&self, _account: &str) -> Result<Option<String>, ProfileError> {
        Ok(None)
    }
    fn delete_secret(&self, _account: &str) -> Result<(), ProfileError> {
        Ok(())
    }
}

fn seed_capture(store: &SqliteStore, capture_id: &str, session_id: &str, received_at: &str) {
    store
        .insert_capture(&CaptureWrite {
            receipt: CaptureReceiptRecord {
                capture_id: capture_id.to_string(),
                idempotency_key: format!("key-{capture_id}"),
                canonical_path: LOCATION.to_string(),
                received_at: received_at.to_string(),
                artifact_count: 1,
            },
            artifacts: vec![CaptureArtifactRecord {
                capture_id: capture_id.to_string(),
                artifact_id: "art-1".to_string(),
                kind: "diff_hunk".to_string(),
                content: "content".to_string(),
                metadata: "{}".to_string(),
                fingerprint: format!("{:064x}", 1u128),
            }],
            job: JobRecord {
                id: format!("job-{capture_id}"),
                kind: ANALYZE_CAPTURE_KIND.to_string(),
                payload: capture_id.to_string(),
                state: JobState::Queued,
                idempotent: true,
                attempts: 0,
                last_error: None,
                created_at: received_at.to_string(),
                updated_at: received_at.to_string(),
            },
            checkpoint: CaptureCheckpointRecord {
                adapter: "opencode".to_string(),
                adapter_version: "0.1.0".to_string(),
                session_id: session_id.to_string(),
                message_id: "message-1".to_string(),
                capture_id: capture_id.to_string(),
                observed_at: received_at.to_string(),
                updated_at: received_at.to_string(),
            },
        })
        .expect("seed capture");
}

fn candidate(
    id: &str,
    capture_id: &str,
    created_at: &str,
    status: &str,
) -> DecisionCandidateRecord {
    DecisionCandidateRecord {
        id: id.to_string(),
        project_id: "project-1".to_string(),
        capture_id: capture_id.to_string(),
        status: status.to_string(),
        question: "q".to_string(),
        choice: "c".to_string(),
        rationale: "r".to_string(),
        signals: "[]".to_string(),
        confidence: 0.5,
        confidence_reason: "x".to_string(),
        evidence_refs: "[\"art-1\"]".to_string(),
        diff_summary: "{\"files\":[],\"artifacts\":1}".to_string(),
        dedup_hash: format!("dedup-{id}"),
        created_at: created_at.to_string(),
        updated_at: created_at.to_string(),
    }
}

#[test]
fn metrics_are_aggregated_from_seeded_timestamps() {
    let root = temporary_directory("aggregate");
    let store = SqliteStore::open(root.join("app.db")).expect("open store");

    ProjectRepository::insert(
        &store,
        &ProjectRecord::new(
            "project-1".to_string(),
            LOCATION.to_string(),
            "2026-01-01T00:00:00Z".to_string(),
        ),
    )
    .expect("seed project");

    // Two receipts at the same instant; candidate offsets give known deltas.
    seed_capture(&store, "capture-1", "session-1", "2026-01-01T00:00:00Z");
    seed_capture(&store, "capture-2", "session-2", "2026-01-01T00:00:00Z");

    store
        .insert_candidates(&[
            candidate("cand-1", "capture-1", "2026-01-01T00:00:01Z", "pending"),
            candidate("cand-2", "capture-2", "2026-01-01T00:00:03Z", "dismissed"),
            candidate(
                "cand-3",
                "capture-1",
                "2026-01-01T00:00:02Z",
                "edited_and_accepted",
            ),
            candidate("cand-4", "capture-2", "2026-01-01T00:00:05Z", "accepted"),
            // Malformed date: it must be ignored, not fail the export.
            candidate("cand-bad", "capture-1", "not-a-date", "pending"),
        ])
        .expect("insert candidates");

    // Confirm cand-1, then pin the confirmation time so review time is exact.
    Inbox::new(store.clone())
        .confirm("cand-1", None)
        .expect("confirm");
    {
        let connection = rusqlite::Connection::open(root.join("app.db")).expect("raw");
        connection
            .execute(
                "UPDATE engineering_decisions SET confirmed_at = '2026-01-01T00:00:11Z' \
                 WHERE candidate_id = 'cand-1'",
                [],
            )
            .expect("pin confirmation time");
    }

    // Losses: two failed assessments, one skipped, one failed job.
    for (index, outcome) in [
        (1, AssessmentOutcome::Failed),
        (2, AssessmentOutcome::Failed),
        (3, AssessmentOutcome::Skipped),
    ] {
        store
            .record_assessment(&AssessmentRecord {
                id: format!("assessment-{index}"),
                capture_id: "capture-1".to_string(),
                job_id: None,
                profile_id: "profile-1".to_string(),
                adapter: "fake".to_string(),
                model: None,
                policy: "{}".to_string(),
                consent_preview_hash: None,
                input_hash: "hash".to_string(),
                started_at: "2026-01-01T00:00:00Z".to_string(),
                finished_at: "2026-01-01T00:00:01Z".to_string(),
                outcome,
                candidates: 0,
                inserted: 0,
                error_code: None,
            })
            .expect("record assessment");
    }
    JobRepository::insert(
        &store,
        &JobRecord {
            id: "job-failed".to_string(),
            kind: ANALYZE_CAPTURE_KIND.to_string(),
            payload: "capture-1".to_string(),
            state: JobState::Failed,
            idempotent: true,
            attempts: 1,
            last_error: Some("o job falhou durante a execução".to_string()),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        },
    )
    .expect("insert failed job");

    // One rejected outbox file.
    let outbox = root.join("outbox");
    std::fs::create_dir_all(outbox.join("rejected")).expect("create rejected dir");
    std::fs::write(outbox.join("rejected").join("item.json"), "{}").expect("write rejected");

    let document = Diagnostics::new(
        store.clone(),
        AiSettings::new(NoProfiles::default(), NoSecrets),
        &outbox,
    )
    .export()
    .expect("export");

    // Latency deltas: [1000, 2000, 3000, 5000] ms (the malformed row is dropped).
    let latency = &document.metrics.latency_capture_to_candidate_ms;
    assert_eq!(latency.samples, 4);
    assert_eq!(latency.p50, Some(3000));
    assert_eq!(latency.p95, Some(5000));

    // Review: one decision, 11s after the candidate.
    let review = &document.metrics.review_time_ms;
    assert_eq!(review.samples, 1);
    assert_eq!(review.p50, Some(10_000));
    assert_eq!(review.p95, Some(10_000));

    // Noise: 1 dismissed out of 4 decided.
    assert_eq!(document.metrics.noise.decided_total, 4);
    assert_eq!(document.metrics.noise.dismissed_ratio, Some(0.25));

    // Losses.
    assert_eq!(document.metrics.losses.assessments_failed, 2);
    assert_eq!(document.metrics.losses.assessments_skipped, 1);
    assert_eq!(document.metrics.losses.jobs_failed, 1);
    assert_eq!(document.metrics.losses.outbox_rejected, 1);

    let _ = std::fs::remove_dir_all(&root);
}
