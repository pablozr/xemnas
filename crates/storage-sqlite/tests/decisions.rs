//! Decision persistence tests: migration 0008, the transactional promotion,
//! evidence dedup/order, rollback on failure and the FTS revision path.

use application::captures::{
    CaptureArtifactRecord, CaptureCheckpointRecord, CaptureReceiptRecord, CaptureRepository,
    CaptureWrite,
};
use application::decisions::{
    DecisionEdits, DecisionFilter, Decisions, DecisionsError, SearchQuery,
};
use application::extract::{DecisionCandidateRecord, ExtractionStore};
use application::inbox::{CandidateStatus, DecisionSeed, Inbox, InboxStore};
use application::jobs::{JobRecord, JobState, ANALYZE_CAPTURE_KIND};
use application::projects::{ProjectRecord, ProjectRepository};
use rusqlite::Connection;
use storage_sqlite::SqliteStore;

const LOCATION: &str = "C:/synthetic/decisions";

fn temporary_directory(tag: &str) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let directory = std::env::temp_dir().join(format!(
        "xemnas-decisions-{tag}-{}-{nanos}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir_all(&directory).expect("create temporary directory");
    directory
}

fn artifact(capture_id: &str, artifact_id: &str, seed: u8) -> CaptureArtifactRecord {
    CaptureArtifactRecord {
        capture_id: capture_id.to_string(),
        artifact_id: artifact_id.to_string(),
        kind: "diff_hunk".to_string(),
        content: format!("content-{artifact_id}"),
        metadata: "{}".to_string(),
        fingerprint: format!("{:064x}", seed as u128),
    }
}

fn seed_project_and_capture(store: &SqliteStore) {
    store
        .insert(&ProjectRecord::new(
            "project-1".to_string(),
            LOCATION.to_string(),
            "2026-01-01T00:00:00Z".to_string(),
        ))
        .expect("seed project");
    let timestamp = "2026-01-01T00:00:00Z".to_string();
    let write = CaptureWrite {
        receipt: CaptureReceiptRecord {
            capture_id: "capture-1".to_string(),
            idempotency_key: "key-capture-1".to_string(),
            canonical_path: LOCATION.to_string(),
            received_at: timestamp.clone(),
            artifact_count: 2,
        },
        artifacts: vec![
            artifact("capture-1", "art-1", 1),
            artifact("capture-1", "art-2", 2),
        ],
        job: JobRecord {
            id: "job-capture-1".to_string(),
            kind: ANALYZE_CAPTURE_KIND.to_string(),
            payload: "capture-1".to_string(),
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
            message_id: "message-1".to_string(),
            capture_id: "capture-1".to_string(),
            observed_at: "2026-01-02T00:00:00Z".to_string(),
            updated_at: "2026-01-02T00:00:00Z".to_string(),
        },
    };
    store.insert_capture(&write).expect("seed capture");
}

fn candidate(id: &str, refs: &str) -> DecisionCandidateRecord {
    DecisionCandidateRecord {
        id: id.to_string(),
        project_id: "project-1".to_string(),
        capture_id: "capture-1".to_string(),
        status: "pending".to_string(),
        question: "alpha question".to_string(),
        choice: "alpha choice".to_string(),
        rationale: "alpha rationale".to_string(),
        signals: "[\"public_contract\"]".to_string(),
        confidence: 0.7,
        confidence_reason: "x".to_string(),
        evidence_refs: refs.to_string(),
        diff_summary: "{\"files\":[],\"artifacts\":2}".to_string(),
        dedup_hash: format!("dedup-{id}"),
        created_at: "2026-01-03T00:00:00Z".to_string(),
        updated_at: "2026-01-03T00:00:00Z".to_string(),
    }
}

fn table_exists(connection: &Connection, name: &str) -> bool {
    connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE name = ?1",
            [name],
            |row| row.get::<_, i64>(0),
        )
        .expect("query sqlite_master")
        > 0
}

#[test]
fn migration_0008_applies_on_fresh_and_upgraded_databases() {
    let root = temporary_directory("migration");
    let database = root.join("app.db");
    SqliteStore::open(&database).expect("open store");

    {
        let connection = Connection::open(&database).expect("raw");
        assert!(table_exists(&connection, "engineering_decisions"));
        assert!(table_exists(&connection, "decision_revisions"));
        assert!(table_exists(&connection, "evidence_links"));
        assert!(table_exists(&connection, "decisions_fts"));
        connection
            .execute_batch(
                "DROP TABLE decisions_fts; DROP TABLE evidence_links; \
                 DROP TABLE decision_revisions; DROP TABLE engineering_decisions; \
                 DELETE FROM schema_migrations WHERE version = 8;",
            )
            .expect("simulate version 6");
    }

    SqliteStore::open(&database).expect("reopen and migrate");
    let connection = Connection::open(&database).expect("raw");
    assert!(table_exists(&connection, "engineering_decisions"));
    assert!(table_exists(&connection, "decisions_fts"));
    let versions: i64 = connection
        .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .expect("count");
    assert_eq!(versions, 9);

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn confirm_promotes_transactionally() {
    let root = temporary_directory("promote");
    let store = SqliteStore::open(root.join("app.db")).expect("open store");
    seed_project_and_capture(&store);
    store
        .insert_candidates(&[candidate("cand-1", "[\"art-2\", \"art-1\"]")])
        .expect("insert candidate");

    let outcome = Inbox::new(store.clone())
        .confirm("cand-1", None)
        .expect("confirm");
    assert_eq!(outcome.status, CandidateStatus::Accepted);

    let decisions = Decisions::new(store.clone());
    let mut detail = decisions.detail(&outcome.decision_id).expect("detail");
    detail
        .evidence
        .push(application::decisions::EvidenceLinkView {
            artifact_id: "unavailable".into(),
            kind: None,
            position: 2,
        });
    let sources = decisions.sources(&detail).expect("redacted sources");
    assert_eq!(
        sources
            .iter()
            .map(|source| source.link.artifact_id.as_str())
            .collect::<Vec<_>>(),
        vec!["art-2", "art-1", "unavailable"]
    );
    assert_eq!(
        sources[0].artifact.as_ref().unwrap().content,
        "content-art-2"
    );
    assert!(
        sources[2].artifact.is_none(),
        "missing evidence retains its link"
    );

    let connection = Connection::open(root.join("app.db")).expect("raw");
    let (status, version, candidate_id): (String, i64, String) = connection
        .query_row(
            "SELECT status, version, candidate_id FROM engineering_decisions WHERE decision_id = ?1",
            [&outcome.decision_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("decision row");
    assert_eq!(status, "accepted");
    assert_eq!(version, 1);
    assert_eq!(candidate_id, "cand-1");

    let revisions: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM decision_revisions WHERE decision_id = ?1",
            [&outcome.decision_id],
            |row| row.get(0),
        )
        .expect("revisions");
    assert_eq!(revisions, 1, "the birth snapshot is v1");

    let links: Vec<(String, i64)> = {
        let mut statement = connection
            .prepare(
                "SELECT artifact_id, position FROM evidence_links \
                 WHERE decision_id = ?1 ORDER BY position",
            )
            .expect("prepare");
        statement
            .query_map([&outcome.decision_id], |row| Ok((row.get(0)?, row.get(1)?)))
            .expect("query")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect")
    };
    assert_eq!(
        links,
        vec![("art-2".to_string(), 0), ("art-1".to_string(), 1)]
    );

    let indexed: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM decisions_fts WHERE decision_id = ?1",
            [&outcome.decision_id],
            |row| row.get(0),
        )
        .expect("fts");
    assert_eq!(indexed, 1);

    let candidate_status: String = connection
        .query_row(
            "SELECT status FROM decision_candidates WHERE id = 'cand-1'",
            [],
            |row| row.get(0),
        )
        .expect("candidate status");
    assert_eq!(candidate_status, "accepted");

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn evidence_refs_deduplicate_and_keep_position_order() {
    let root = temporary_directory("dedup");
    let store = SqliteStore::open(root.join("app.db")).expect("open store");
    seed_project_and_capture(&store);
    store
        .insert_candidates(&[candidate("cand-1", "[\"art-1\", \"art-1\", \"art-2\"]")])
        .expect("insert candidate");

    let outcome = Inbox::new(store.clone())
        .confirm("cand-1", None)
        .expect("confirm");
    let connection = Connection::open(root.join("app.db")).expect("raw");
    let links: Vec<(String, i64)> = {
        let mut statement = connection
            .prepare(
                "SELECT artifact_id, position FROM evidence_links \
                 WHERE decision_id = ?1 ORDER BY position",
            )
            .expect("prepare");
        statement
            .query_map([&outcome.decision_id], |row| Ok((row.get(0)?, row.get(1)?)))
            .expect("query")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect")
    };
    assert_eq!(
        links,
        vec![("art-1".to_string(), 0), ("art-2".to_string(), 1)]
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn promote_rolls_back_completely_on_failure() {
    let root = temporary_directory("rollback");
    let store = SqliteStore::open(root.join("app.db")).expect("open store");
    seed_project_and_capture(&store);
    store
        .insert_candidates(&[
            candidate("cand-1", "[\"art-1\"]"),
            candidate("cand-2", "[\"art-1\"]"),
        ])
        .expect("insert candidates");

    let first = Inbox::new(store.clone())
        .confirm("cand-1", None)
        .expect("confirm first");

    let colliding = DecisionSeed {
        decision_id: first.decision_id.clone(),
        project_id: "project-1".to_string(),
        capture_id: Some("capture-1".to_string()),
        question: "q".to_string(),
        choice: "c".to_string(),
        rationale: "r".to_string(),
        evidence_refs: vec!["art-1".to_string()],
    };
    let result = InboxStore::confirm_one(
        &store,
        "cand-2",
        &InboxStore::get(&store, "cand-2")
            .expect("get")
            .expect("row"),
        None,
        &colliding,
        "2026-01-04T00:00:00Z",
    );
    assert!(result.is_err(), "the collision must surface as an error");

    let status: String = Connection::open(root.join("app.db"))
        .expect("raw")
        .query_row(
            "SELECT status FROM decision_candidates WHERE id = 'cand-2'",
            [],
            |row| row.get(0),
        )
        .expect("status");
    assert_eq!(status, "pending", "the candidate update must roll back too");

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn revise_updates_the_index_and_keeps_the_history() {
    let root = temporary_directory("revise");
    let store = SqliteStore::open(root.join("app.db")).expect("open store");
    seed_project_and_capture(&store);
    store
        .insert_candidates(&[candidate("cand-1", "[\"art-1\", \"art-2\"]")])
        .expect("insert candidate");

    let outcome = Inbox::new(store.clone())
        .confirm("cand-1", None)
        .expect("confirm");
    let decisions = Decisions::new(store.clone());

    let detail = decisions
        .revise(
            &outcome.decision_id,
            DecisionEdits {
                question: Some("gamma distinctive".to_string()),
                ..DecisionEdits::default()
            },
        )
        .expect("revise");
    assert_eq!(detail.summary.version, 2);
    assert_eq!(detail.rationale, "alpha rationale", "untouched field");
    let versions: Vec<i64> = detail.revisions.iter().map(|row| row.version).collect();
    assert_eq!(versions, vec![2, 1], "both snapshots remain readable");
    assert_eq!(detail.evidence.len(), 2);

    let stale = decisions.revise_version(
        &outcome.decision_id,
        1,
        DecisionEdits {
            rationale: Some("stale editor must not overwrite".into()),
            ..DecisionEdits::default()
        },
    );
    assert_eq!(stale.map(|_| ()), Err(DecisionsError::Conflict));
    assert_eq!(
        decisions
            .detail(&outcome.decision_id)
            .unwrap()
            .summary
            .version,
        2
    );

    let hits = decisions
        .search(&SearchQuery {
            query: "gamma".to_string(),
            ..SearchQuery::default()
        })
        .expect("search new");
    assert_eq!(hits.len(), 1);
    assert!(!hits[0].snippet.is_empty());

    let old = decisions
        .search(&SearchQuery {
            query: "question".to_string(),
            ..SearchQuery::default()
        })
        .expect("search old");
    assert!(old.is_empty(), "the superseded text leaves the index");

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn revision_content_is_reconstructible_in_full() {
    let root = temporary_directory("revision-content");
    let store = SqliteStore::open(root.join("app.db")).expect("open store");
    seed_project_and_capture(&store);
    store
        .insert_candidates(&[candidate("cand-1", "[\"art-1\", \"art-2\"]")])
        .expect("insert candidate");

    let outcome = Inbox::new(store.clone())
        .confirm("cand-1", None)
        .expect("confirm");
    let decisions = Decisions::new(store.clone());
    let detail = decisions
        .revise(
            &outcome.decision_id,
            DecisionEdits {
                question: Some("pergunta \"nova\" — ç".to_string()),
                choice: Some("escolha ç".to_string()),
                rationale: Some("razão \"x\"".to_string()),
                assumptions: Some(vec!["premissa ç".to_string(), "com \"aspas\"".to_string()]),
                reconsider_when: Some(vec!["quando ç".to_string()]),
                scope: Some(vec!["escopo ç".to_string()]),
                consequences: Some(vec!["consequência ç".to_string()]),
            },
        )
        .expect("revise all fields");

    let original = detail
        .revisions
        .iter()
        .find(|revision| revision.version == 1)
        .expect("v1 must remain readable");
    assert_eq!(original.question, "alpha question");
    assert_eq!(original.choice, "alpha choice");
    assert_eq!(original.rationale, "alpha rationale");
    assert!(original.assumptions.is_empty());
    assert!(original.scope.is_empty());

    let live = detail
        .revisions
        .iter()
        .find(|revision| revision.version == 2)
        .expect("v2");
    assert_eq!(live.question, "pergunta \"nova\" — ç");
    assert_eq!(
        live.assumptions,
        vec!["premissa ç".to_string(), "com \"aspas\"".to_string()]
    );
    assert_eq!(live.reconsider_when, vec!["quando ç"]);
    assert_eq!(live.scope, vec!["escopo ç"]);
    assert_eq!(live.consequences, vec!["consequência ç"]);

    let connection = Connection::open(root.join("app.db")).expect("raw");
    let (question, assumptions): (String, String) = connection
        .query_row(
            "SELECT question, assumptions FROM decision_revisions \
             WHERE decision_id = ?1 AND version = 2",
            [&outcome.decision_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("v2 row");
    assert_eq!(question, "pergunta \"nova\" — ç");
    assert!(assumptions.contains("ç"));
    assert!(assumptions.contains("aspas"));

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn decisions_list_join_exposes_project_location() {
    let root = temporary_directory("list");
    let store = SqliteStore::open(root.join("app.db")).expect("open store");
    seed_project_and_capture(&store);
    store
        .insert_candidates(&[candidate("cand-1", "[\"art-1\"]")])
        .expect("insert candidate");
    Inbox::new(store.clone())
        .confirm("cand-1", None)
        .expect("confirm");

    let page = Decisions::new(store)
        .list(&DecisionFilter::new())
        .expect("list");
    assert_eq!(page.decisions.len(), 1);
    assert_eq!(page.decisions[0].project_location, LOCATION);
    assert_eq!(page.decisions[0].version, 1);

    let _ = std::fs::remove_dir_all(&root);
}
