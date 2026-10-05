//! Capture progress snapshots do not confuse prior attempts with current execution.
mod support;
use application::capture_progress::{AssessmentReason, CaptureProgressReader, CaptureState};

#[test]
fn project_scope_counts_and_retry_running_hide_old_failure() {
    let test = support::open("capture-progress", &["p1", "p2"]);
    support::decision(&test.store, "p1", "one", "Question", "Choice");
    let reader = CaptureProgressReader(test.store.clone());
    assert!(reader.capture("p2", "capture-p1").unwrap().is_none());
    let row = reader.capture("p1", "capture-p1").unwrap().unwrap();
    assert_eq!(row.candidates.adopted, 1);
    let db = rusqlite::Connection::open(test.root.join("app.db")).unwrap();
    db.execute(
        "INSERT INTO assessments (id,capture_id,job_id,profile_id,adapter,policy,input_hash,
        started_at,finished_at,outcome,candidates,inserted,attempt,reason)
        SELECT 'old-failure','capture-p1',id,'fake','fake','{}','hash','old','old','failed',0,0,1,'failed'
        FROM jobs WHERE payload='capture-p1'",[]).unwrap();
    db.execute(
        "UPDATE jobs SET state='queued',attempts=1 WHERE payload='capture-p1'",
        [],
    )
    .unwrap();
    let queued = reader.capture("p1", "capture-p1").unwrap().unwrap();
    assert_eq!(queued.state, CaptureState::Queued);
    assert_eq!(queued.reason, AssessmentReason::Unknown);
    db.execute(
        "UPDATE jobs SET state='running',attempts=2 WHERE payload='capture-p1'",
        [],
    )
    .unwrap();
    let row = reader.capture("p1", "capture-p1").unwrap().unwrap();
    assert_eq!(row.state, CaptureState::Running);
    assert_eq!(row.reason, AssessmentReason::Unknown);
    assert!(!row.can_retry);
    assert!(reader.recent("never-received", 10).unwrap().is_empty());
}

#[test]
fn detail_has_no_retry_and_hidden_candidates_are_not_empty() {
    let test = support::open("capture-progress-detail", &["p1"]);
    support::decision(&test.store, "p1", "hidden", "Question", "Choice");
    let db = rusqlite::Connection::open(test.root.join("app.db")).unwrap();
    db.execute(
        "UPDATE decision_candidates SET status='pending',significance=0.1",
        [],
    )
    .unwrap();
    db.execute(
        "UPDATE jobs SET state='completed',attempts=1 WHERE payload='capture-p1'",
        [],
    )
    .unwrap();
    db.execute("INSERT INTO assessments (id,capture_id,job_id,profile_id,adapter,policy,input_hash,
        started_at,finished_at,outcome,candidates,inserted,attempt,reason,durable_count,detail_count)
        SELECT 'detail','capture-p1',id,'fake','fake','{}','hash','now','now','ok',0,0,1,'detail',0,2
        FROM jobs WHERE payload='capture-p1'",[]).unwrap();
    let row = CaptureProgressReader(test.store.clone())
        .capture("p1", "capture-p1")
        .unwrap()
        .unwrap();
    assert_eq!(row.reason, AssessmentReason::Detail);
    assert_eq!(row.detail, 2);
    assert_eq!(row.candidates.hidden, 1);
    assert_eq!(row.candidates.pending, 0);
    assert!(!row.can_retry);
}

fn prompt_capture(project: &str, n: usize, prompt: &str) -> application::captures::CaptureWrite {
    use application::captures::*;
    use application::jobs::{JobRecord, JobState, ANALYZE_CAPTURE_KIND};
    let capture = format!("title-{n}");
    let at = format!("2026-02-01T00:{:02}:{:02}Z", n / 60, n % 60);
    CaptureWrite {
        receipt: CaptureReceiptRecord {
            capture_id: capture.clone(),
            idempotency_key: format!("key-{capture}"),
            canonical_path: format!("C:/synthetic/{project}"),
            received_at: at.clone(),
            artifact_count: 1,
        },
        artifacts: vec![CaptureArtifactRecord {
            capture_id: capture.clone(),
            artifact_id: "a1".into(),
            kind: "user_text".into(),
            content: prompt.into(),
            metadata: "{}".into(),
            fingerprint: format!("{n:0>64}"),
        }],
        job: JobRecord {
            id: format!("job-{capture}"),
            kind: ANALYZE_CAPTURE_KIND.into(),
            payload: capture.clone(),
            state: JobState::Queued,
            idempotent: true,
            attempts: 0,
            last_error: None,
            created_at: at.clone(),
            updated_at: at.clone(),
        },
        checkpoint: CaptureCheckpointRecord {
            adapter: "claude-code".into(),
            adapter_version: "1".into(),
            session_id: format!("s{n}"),
            message_id: format!("m{n}"),
            capture_id: capture,
            observed_at: at.clone(),
            updated_at: at,
        },
    }
}

#[test]
fn title_is_first_prompt_line_ellipsized_and_recent_stays_fast() {
    use application::capture_episode::CaptureProvenance;
    use application::captures::CaptureRepository;
    let test = support::open("capture-progress-title", &["p1"]);
    let provenance = CaptureProvenance {
        adapter: Some("claude-code".into()),
        adapter_version: None,
        session_id: None,
        message_id: None,
        observed_at: None,
    };
    let long = format!(
        "\n  Vamos usar SQLite {}ção\nsegunda linha",
        "á".repeat(200)
    );
    test.store
        .insert_capture_with_provenance(&prompt_capture("p1", 0, &long), &provenance)
        .unwrap();
    for n in 1..100 {
        test.store
            .insert_capture_with_provenance(
                &prompt_capture("p1", n, "Ajustar o painel"),
                &provenance,
            )
            .unwrap();
    }
    let reader = CaptureProgressReader(test.store.clone());

    let started = std::time::Instant::now();
    let rows = reader.recent("p1", 100).unwrap();
    let elapsed = started.elapsed();
    eprintln!("recent(100 captures) took {elapsed:?}");
    assert!(
        elapsed < std::time::Duration::from_millis(250),
        "{elapsed:?}"
    );
    assert_eq!(rows.len(), 100);

    let first = reader.capture("p1", "title-0").unwrap().unwrap();
    let title = first.title.unwrap();
    assert_eq!(title.chars().count(), 90);
    assert!(title.starts_with("Vamos usar SQLite") && title.ends_with('…'));
    assert!(!title.contains("segunda"));
    assert_eq!(first.adapter.as_deref(), Some("claude-code"));
    assert_eq!(rows[0].title.as_deref(), Some("Ajustar o painel"));
    let seeded = reader.capture("p1", "capture-p1").unwrap().unwrap();
    assert_eq!((seeded.title, seeded.adapter), (None, None));
}
