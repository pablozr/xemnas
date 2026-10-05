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
