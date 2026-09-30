//! Inbox persistence tests: the join behind the list, the artifact order in the
//! detail, compare-and-set exclusivity and the default status filter.

use application::captures::{
    CaptureArtifactRecord, CaptureCheckpointRecord, CaptureReceiptRecord, CaptureRepository,
    CaptureWrite,
};
use application::extract::{DecisionCandidateRecord, ExtractionStore};
use application::inbox::{
    CandidateEdits, CandidateStatus, DecisionSeed, Inbox, InboxFilter, InboxStore,
};
use application::jobs::{JobRecord, JobState, ANALYZE_CAPTURE_KIND};
use application::projects::{ProjectRecord, ProjectRepository};
use storage_sqlite::SqliteStore;

const LOCATION: &str = "C:/synthetic/inbox";

#[test]
fn queue_count_ignores_page_size_and_cursor_and_tracks_review_actions() {
    let store = SqliteStore::open(":memory:").expect("store");
    seed_project(&store);
    seed_capture(&store, "capture-count", "message-count");
    let rows: Vec<_> = (0..57)
        .map(|index| {
            candidate(
                &format!("count-{index:03}"),
                "capture-count",
                "2026-09-29T15:00:00Z",
                "pending",
            )
        })
        .collect();
    store.insert_candidates(&rows).expect("candidates");
    let inbox = Inbox::new(store);
    let mut filter = InboxFilter {
        project_id: Some("project-1".into()),
        limit: 2,
        ..InboxFilter::default()
    };
    let first = inbox.list(&filter).expect("page");
    assert_eq!(first.candidates.len(), 2);
    filter.cursor = first.next_cursor;
    assert_eq!(inbox.count(&filter).expect("count"), 57);
    inbox.snooze("count-000").expect("snooze");
    assert_eq!(
        inbox
            .count(&filter)
            .expect("deferred still awaiting review"),
        57
    );
    inbox.reject("count-001").expect("reject");
    inbox.confirm("count-002", None).expect("confirm");
    assert_eq!(inbox.count(&filter).expect("remaining"), 55);
    filter.project_id = Some("another-project".into());
    assert_eq!(inbox.count(&filter).expect("other project"), 0);
}

fn temporary_directory(tag: &str) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let directory = std::env::temp_dir().join(format!(
        "xemnas-inbox-{tag}-{}-{nanos}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir_all(&directory).expect("create temporary directory");
    directory
}

fn artifact(
    capture_id: &str,
    artifact_id: &str,
    kind: &str,
    content: &str,
    seed: u8,
) -> CaptureArtifactRecord {
    CaptureArtifactRecord {
        capture_id: capture_id.to_string(),
        artifact_id: artifact_id.to_string(),
        kind: kind.to_string(),
        content: content.to_string(),
        metadata: "{}".to_string(),
        fingerprint: format!("{:064x}", seed as u128),
    }
}

/// Seeds one project and one capture; the adapter session is shared, so a later
/// `seed_capture` call moves the checkpoint away from the earlier capture.
fn seed_capture(store: &SqliteStore, capture_id: &str, message_id: &str) {
    let timestamp = "2026-01-01T00:00:00Z".to_string();
    let write = CaptureWrite {
        receipt: CaptureReceiptRecord {
            capture_id: capture_id.to_string(),
            idempotency_key: format!("key-{capture_id}"),
            canonical_path: LOCATION.to_string(),
            received_at: timestamp.clone(),
            artifact_count: 2,
        },
        artifacts: vec![
            artifact(capture_id, "art-1", "diff_hunk", "content-1", 1),
            artifact(capture_id, "art-2", "user_text", "content-2", 2),
        ],
        job: JobRecord {
            id: format!("job-{capture_id}"),
            kind: ANALYZE_CAPTURE_KIND.to_string(),
            payload: capture_id.to_string(),
            state: JobState::Queued,
            idempotent: true,
            attempts: 0,
            last_error: None,
            created_at: timestamp.clone(),
            updated_at: timestamp,
        },
        checkpoint: CaptureCheckpointRecord {
            adapter: "opencode".to_string(),
            adapter_version: "0.1.0".to_string(),
            session_id: "session-1".to_string(),
            message_id: message_id.to_string(),
            capture_id: capture_id.to_string(),
            observed_at: "2026-01-02T00:00:00Z".to_string(),
            updated_at: "2026-01-02T00:00:00Z".to_string(),
        },
    };
    store.insert_capture(&write).expect("seed capture");
}

fn seed_project(store: &SqliteStore) {
    store
        .insert(&ProjectRecord::new(
            "project-1".to_string(),
            LOCATION.to_string(),
            "2026-01-01T00:00:00Z".to_string(),
        ))
        .expect("seed project");
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
        signals: "[\"public_contract\"]".to_string(),
        confidence: 0.7,
        confidence_reason: "x".to_string(),
        evidence_refs: "[\"art-2\", \"art-1\"]".to_string(),
        diff_summary: "{\"files\":[\"src/a.rs\"],\"artifacts\":2}".to_string(),
        dedup_hash: format!("dedup-{id}"),
        created_at: created_at.to_string(),
        updated_at: created_at.to_string(),
    }
}

#[test]
fn list_joins_project_and_checkpoint_with_receipt_fallback() {
    let root = temporary_directory("list");
    let store = SqliteStore::open(root.join("app.db")).expect("open store");
    seed_project(&store);
    seed_capture(&store, "capture-1", "message-1");
    seed_capture(&store, "capture-2", "message-2");
    store
        .insert_candidates(&[
            candidate("cand-1", "capture-1", "2026-01-03T00:00:00Z", "pending"),
            candidate("cand-2", "capture-2", "2026-01-04T00:00:00Z", "pending"),
        ])
        .expect("insert candidates");

    let page = Inbox::new(store).list(&InboxFilter::new()).expect("list");
    assert_eq!(page.candidates.len(), 2);

    let first = page
        .candidates
        .iter()
        .find(|row| row.id == "cand-2")
        .expect("cand-2");
    assert_eq!(first.project_location, LOCATION);
    assert_eq!(first.adapter.as_deref(), Some("opencode"));
    assert_eq!(first.session_id.as_deref(), Some("session-1"));
    assert_eq!(first.observed_at.as_deref(), Some("2026-01-02T00:00:00Z"));
    assert_eq!(first.received_at, "2026-01-01T00:00:00Z");

    let fallback = page
        .candidates
        .iter()
        .find(|row| row.id == "cand-1")
        .expect("cand-1");
    assert_eq!(fallback.adapter, None);
    assert_eq!(fallback.session_id, None);
    assert_eq!(
        fallback.observed_at.as_deref(),
        Some("2026-01-01T00:00:00Z"),
        "without a checkpoint the observation falls back to the receipt"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn default_list_excludes_terminal_statuses() {
    let root = temporary_directory("default-filter");
    let store = SqliteStore::open(root.join("app.db")).expect("open store");
    seed_project(&store);
    seed_capture(&store, "capture-1", "message-1");
    store
        .insert_candidates(&[
            candidate("pending", "capture-1", "2026-01-01T00:00:00Z", "pending"),
            candidate("accepted", "capture-1", "2026-01-02T00:00:00Z", "accepted"),
            candidate(
                "dismissed",
                "capture-1",
                "2026-01-03T00:00:00Z",
                "dismissed",
            ),
            candidate("snoozed", "capture-1", "2026-01-04T00:00:00Z", "snoozed"),
        ])
        .expect("insert candidates");

    let page = Inbox::new(store).list(&InboxFilter::new()).expect("list");
    let ids: Vec<&str> = page.candidates.iter().map(|row| row.id.as_str()).collect();
    assert_eq!(ids, vec!["snoozed", "pending"]);

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn detail_returns_referenced_artifacts_in_order() {
    let root = temporary_directory("detail");
    let store = SqliteStore::open(root.join("app.db")).expect("open store");
    seed_project(&store);
    seed_capture(&store, "capture-1", "message-1");
    store
        .insert_candidates(&[candidate(
            "cand-1",
            "capture-1",
            "2026-01-03T00:00:00Z",
            "pending",
        )])
        .expect("insert candidate");

    let detail = Inbox::new(store).detail("cand-1").expect("detail");
    let ids: Vec<&str> = detail
        .artifacts
        .iter()
        .map(|artifact| artifact.artifact_id.as_str())
        .collect();
    assert_eq!(ids, vec!["art-2", "art-1"], "stored reference order");
    assert_eq!(detail.artifacts[0].content, "content-2");
    assert_eq!(detail.diff_summary.files, vec!["src/a.rs"]);
    assert_eq!(detail.diff_summary.artifacts, 2);
    assert_eq!(detail.evidence_refs, vec!["art-2", "art-1"]);

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn compare_and_set_reject_is_exclusive() {
    let root = temporary_directory("cas");
    let store = SqliteStore::open(root.join("app.db")).expect("open store");
    seed_project(&store);
    seed_capture(&store, "capture-1", "message-1");
    store
        .insert_candidates(&[candidate(
            "cand-1",
            "capture-1",
            "2026-01-03T00:00:00Z",
            "pending",
        )])
        .expect("insert candidate");

    assert!(InboxStore::dismiss_one(&store, "cand-1", "t1").expect("first reject"));
    assert!(!InboxStore::dismiss_one(&store, "cand-1", "t2").expect("second reject"));

    let inbox = Inbox::new(store);
    assert_eq!(
        inbox
            .reject("cand-1")
            .expect_err("already dismissed")
            .code(),
        "invalid_state"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn adjust_advances_updated_at_and_persists_the_edits() {
    let root = temporary_directory("adjust");
    let store = SqliteStore::open(root.join("app.db")).expect("open store");
    seed_project(&store);
    seed_capture(&store, "capture-1", "message-1");
    store
        .insert_candidates(&[candidate(
            "cand-1",
            "capture-1",
            "2026-01-03T00:00:00Z",
            "pending",
        )])
        .expect("insert candidate");

    let inbox = Inbox::new(store.clone());
    inbox
        .adjust(
            "cand-1",
            application::inbox::CandidateEdits {
                question: "nova pergunta".to_string(),
                choice: "nova escolha".to_string(),
                rationale: "nova justificativa".to_string(),
            },
        )
        .expect("adjust");

    let stored = InboxStore::get(&store, "cand-1")
        .expect("get")
        .expect("row");
    assert_eq!(stored.status, CandidateStatus::Pending);
    assert_eq!(stored.question, "nova pergunta");
    assert_ne!(
        stored.updated_at, stored.created_at,
        "adjust must advance updated_at"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn semantic_port_fixes_batch_destinations_and_rejects_wrong_sources() {
    let root = temporary_directory("semantic-port");
    let store = SqliteStore::open(root.join("app.db")).expect("open store");
    seed_project(&store);
    seed_capture(&store, "capture-1", "message-1");
    store
        .insert_candidates(&[
            candidate("cand-1", "capture-1", "2026-01-03T00:00:00Z", "pending"),
            candidate("cand-2", "capture-1", "2026-01-04T00:00:00Z", "pending"),
            candidate(
                "cand-terminal",
                "capture-1",
                "2026-01-05T00:00:00Z",
                "dismissed",
            ),
        ])
        .expect("insert candidates");

    assert_eq!(
        InboxStore::snooze_batch(&store, &["cand-1".to_string()], "t1").expect("snooze batch"),
        1
    );
    assert_eq!(status_of(&store, "cand-1"), CandidateStatus::Snoozed);
    assert_eq!(
        InboxStore::dismiss_batch(&store, &["cand-2".to_string()], "t1").expect("dismiss batch"),
        1
    );
    assert_eq!(status_of(&store, "cand-2"), CandidateStatus::Dismissed);

    assert_eq!(
        InboxStore::dismiss_batch(&store, &["cand-terminal".to_string()], "t2").expect("no-op"),
        0
    );
    assert!(!InboxStore::confirm_one(
        &store,
        "cand-terminal",
        &InboxStore::get(&store, "cand-terminal")
            .expect("get")
            .expect("row"),
        None,
        &seed("d-terminal"),
        "t2"
    )
    .expect("no-op"));

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn confirmation_rejects_an_adjustment_even_with_the_same_timestamp() {
    let root = temporary_directory("stale-confirm");
    let store = SqliteStore::open(root.join("app.db")).expect("open store");
    seed_project(&store);
    seed_capture(&store, "capture-1", "message-1");
    store
        .insert_candidates(&[candidate(
            "cand-1",
            "capture-1",
            "2026-01-03T00:00:00Z",
            "pending",
        )])
        .expect("candidate");
    let reviewed = InboxStore::get(&store, "cand-1")
        .expect("get")
        .expect("row");
    let edits = CandidateEdits {
        question: "corrected question".into(),
        choice: "corrected choice".into(),
        rationale: "corrected rationale".into(),
    }
    .validate()
    .expect("edits");
    assert!(
        InboxStore::adjust_one(&store, "cand-1", &edits, &reviewed.updated_at).expect("adjust")
    );
    for confirm_edits in [None, Some(&edits)] {
        assert!(!InboxStore::confirm_one(
            &store,
            "cand-1",
            &reviewed,
            confirm_edits,
            &seed("stale-decision"),
            "later"
        )
        .expect("stale confirmation"));
    }
    let current = InboxStore::get(&store, "cand-1")
        .expect("get")
        .expect("row");
    assert_eq!(current.status, CandidateStatus::Pending);
    assert_eq!(current.question, "corrected question");
    assert!(application::decisions::Decisions::new(store.clone())
        .detail("stale-decision")
        .is_err());
    let confirmed = Inbox::new(store.clone())
        .confirm("cand-1", None)
        .expect("reviewed confirmation");
    let decision = application::decisions::Decisions::new(store)
        .detail(&confirmed.decision_id)
        .expect("decision");
    assert_eq!(decision.summary.question, "corrected question");
    let _ = std::fs::remove_dir_all(&root);
}

/// Reads a candidate status through the inbox port.
fn status_of(store: &SqliteStore, id: &str) -> CandidateStatus {
    InboxStore::get(store, id)
        .expect("get")
        .expect("row")
        .status
}

/// Builds a decision seed for a direct port-level confirmation.
fn seed(decision_id: &str) -> DecisionSeed {
    DecisionSeed {
        decision_id: decision_id.to_string(),
        project_id: "project-1".to_string(),
        capture_id: Some("capture-1".to_string()),
        question: "q".to_string(),
        choice: "c".to_string(),
        rationale: "r".to_string(),
        evidence_refs: vec!["art-2".to_string(), "art-1".to_string()],
    }
}

#[test]
fn validated_edits_are_persisted_by_confirm_and_adjust() {
    let root = temporary_directory("validated-edits");
    let store = SqliteStore::open(root.join("app.db")).expect("open store");
    seed_project(&store);
    seed_capture(&store, "capture-1", "message-1");
    store
        .insert_candidates(&[
            candidate("cand-1", "capture-1", "2026-01-03T00:00:00Z", "pending"),
            candidate("cand-2", "capture-1", "2026-01-04T00:00:00Z", "pending"),
        ])
        .expect("insert candidates");

    let confirm_edits = CandidateEdits {
        question: " confirma q ".to_string(),
        choice: " confirma c ".to_string(),
        rationale: " confirma r ".to_string(),
    }
    .validate()
    .expect("valid confirm edits");
    assert!(InboxStore::confirm_one(
        &store,
        "cand-1",
        &InboxStore::get(&store, "cand-1")
            .expect("get")
            .expect("row"),
        Some(&confirm_edits),
        &seed("decision-1"),
        "t1"
    )
    .expect("confirm"));
    let stored = InboxStore::get(&store, "cand-1")
        .expect("get")
        .expect("row");
    assert_eq!(stored.status, CandidateStatus::EditedAndAccepted);
    assert_eq!(stored.question, "confirma q");
    assert_eq!(stored.choice, "confirma c");
    assert_eq!(stored.rationale, "confirma r");

    let adjust_edits = CandidateEdits {
        question: "ajusta q".to_string(),
        choice: "ajusta c".to_string(),
        rationale: "ajusta r".to_string(),
    }
    .validate()
    .expect("valid adjust edits");
    assert!(InboxStore::adjust_one(&store, "cand-2", &adjust_edits, "t2").expect("adjust"));
    let stored = InboxStore::get(&store, "cand-2")
        .expect("get")
        .expect("row");
    assert_eq!(stored.status, CandidateStatus::Pending);
    assert_eq!(stored.question, "ajusta q");
    assert_eq!(stored.choice, "ajusta c");

    let _ = std::fs::remove_dir_all(&root);
}
