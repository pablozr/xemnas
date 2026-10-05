//! Extraction persistence tests: migration 0005, deterministic evidence loading,
//! idempotent candidate insertion, the status check and FK cascade.

use application::captures::{
    CaptureArtifactRecord, CaptureCheckpointRecord, CaptureReceiptRecord, CaptureRepository,
    CaptureWrite,
};
use application::extract::{DecisionCandidateRecord, ExtractionStore, MAX_EVIDENCE_CONTENT_BYTES};
use application::jobs::{JobRecord, JobState, ANALYZE_CAPTURE_KIND};
use application::projects::{ProjectRecord, ProjectRepository};
use rusqlite::{params, Connection};
use storage_sqlite::SqliteStore;

const LOCATION: &str = "C:/synthetic/project";

fn temporary_directory(tag: &str) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let directory = std::env::temp_dir().join(format!(
        "xemnas-extraction-{tag}-{}-{nanos}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir_all(&directory).expect("create temporary directory");
    directory
}

fn artifact(
    capture_id: &str,
    artifact_id: &str,
    kind: &str,
    content: String,
    seed: u8,
) -> CaptureArtifactRecord {
    CaptureArtifactRecord {
        capture_id: capture_id.to_string(),
        artifact_id: artifact_id.to_string(),
        kind: kind.to_string(),
        content,
        metadata: "{}".to_string(),
        fingerprint: format!("{:064x}", seed as u128),
    }
}

fn seed_capture(store: &SqliteStore, capture_id: &str, artifacts: Vec<CaptureArtifactRecord>) {
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
            capture_id: capture_id.to_string(),
            idempotency_key: format!("key-{capture_id}"),
            canonical_path: LOCATION.to_string(),
            received_at: timestamp.clone(),
            artifact_count: artifacts.len() as i64,
        },
        artifacts,
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
            message_id: "message-1".to_string(),
            capture_id: capture_id.to_string(),
            observed_at: "2026-01-02T00:00:00Z".to_string(),
            updated_at: "2026-01-02T00:00:00Z".to_string(),
        },
    };
    store.insert_capture(&write).expect("seed capture");
}

fn candidate(id: &str, capture_id: &str, dedup: &str) -> DecisionCandidateRecord {
    DecisionCandidateRecord {
        qualifiers: "[]".into(),
        id: id.to_string(),
        project_id: "project-1".to_string(),
        capture_id: capture_id.to_string(),
        status: "pending".to_string(),
        question: "q".to_string(),
        choice: "c".to_string(),
        rationale: "r".to_string(),
        signals: "[]".to_string(),
        confidence: 0.6,
        confidence_reason: "x".to_string(),
        evidence_refs: "[]".to_string(),
        diff_summary: "{\"files\":[],\"artifacts\":0}".to_string(),
        dedup_hash: dedup.to_string(),
        created_at: "2026-01-01T00:00:00Z".to_string(),
        updated_at: "2026-01-01T00:00:00Z".to_string(),
        kind: "decision".to_string(),
        significance: 1.0,
        criteria: "[]".to_string(),
    }
}

fn row_count(connection: &Connection, table: &str) -> i64 {
    connection
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .expect("count rows")
}

fn table_exists(connection: &Connection, name: &str) -> bool {
    connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [name],
            |row| row.get::<_, i64>(0),
        )
        .expect("query sqlite_master")
        > 0
}

#[test]
fn load_evidence_returns_ordered_artifacts_with_bounds() {
    let root = temporary_directory("evidence");
    let store = SqliteStore::open(root.join("app.db")).expect("open store");
    let capture_id = "capture-1";
    let long_content = "x".repeat(MAX_EVIDENCE_CONTENT_BYTES * 2);
    seed_capture(
        &store,
        capture_id,
        vec![
            artifact(
                capture_id,
                "b-artifact",
                "diff_hunk",
                "second".to_string(),
                2,
            ),
            artifact(capture_id, "a-artifact", "user_text", long_content, 1),
        ],
    );

    let evidence = store
        .load_evidence(capture_id)
        .expect("load")
        .expect("evidence");
    assert_eq!(evidence.project_id, "project-1");
    assert_eq!(evidence.adapter.as_deref(), Some("opencode"));
    assert_eq!(evidence.session_id.as_deref(), Some("session-1"));
    assert_eq!(
        evidence.observed_at.as_deref(),
        Some("2026-01-02T00:00:00Z")
    );
    assert_eq!(evidence.artifacts.len(), 2);
    assert_eq!(evidence.artifacts[0].artifact_id, "a-artifact");
    assert_eq!(evidence.artifacts[1].artifact_id, "b-artifact");
    assert!(
        evidence.artifacts[0].content.len() <= MAX_EVIDENCE_CONTENT_BYTES,
        "content must be bounded"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn load_evidence_for_unknown_capture_is_none() {
    let root = temporary_directory("evidence-missing");
    let store = SqliteStore::open(root.join("app.db")).expect("open store");
    assert!(store.load_evidence("nope").expect("load").is_none());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn insert_candidates_is_idempotent_by_dedup_hash() {
    let root = temporary_directory("insert");
    let database = root.join("app.db");
    let store = SqliteStore::open(&database).expect("open store");
    let capture_id = "capture-1";
    seed_capture(
        &store,
        capture_id,
        vec![artifact(
            capture_id,
            "a1",
            "user_text",
            "content".to_string(),
            1,
        )],
    );

    let record = candidate("candidate-1", capture_id, "dedup-1");
    assert_eq!(
        store
            .insert_candidates(std::slice::from_ref(&record))
            .expect("first insert"),
        1
    );
    assert_eq!(
        store.insert_candidates(&[record]).expect("second insert"),
        0
    );

    let connection = Connection::open(&database).expect("open raw connection");
    assert_eq!(row_count(&connection, "decision_candidates"), 1);

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn status_check_and_project_cascade_are_enforced() {
    let root = temporary_directory("constraints");
    let database = root.join("app.db");
    let store = SqliteStore::open(&database).expect("open store");
    let capture_id = "capture-1";
    seed_capture(
        &store,
        capture_id,
        vec![artifact(
            capture_id,
            "a1",
            "user_text",
            "content".to_string(),
            1,
        )],
    );

    let mut invalid = candidate("candidate-bad", capture_id, "dedup-bad");
    invalid.status = "bogus".to_string();
    let ignored = store
        .insert_candidates(&[invalid])
        .expect("ignore the rejected row");
    assert_eq!(ignored, 0, "the status CHECK must reject an unknown status");

    store
        .insert_candidates(&[candidate("candidate-1", capture_id, "dedup-1")])
        .expect("insert candidate");
    let connection = Connection::open(&database).expect("open raw connection");
    assert_eq!(row_count(&connection, "decision_candidates"), 1);

    let bad = connection.execute(
        "INSERT INTO decision_candidates \
         (id, project_id, capture_id, status, question, choice, rationale, signals, confidence, \
          confidence_reason, evidence_refs, diff_summary, dedup_hash, created_at, updated_at) \
         VALUES (?1, ?2, ?3, 'bogus', 'q', 'c', 'r', '[]', 0.5, 'x', '[]', '{}', 'dedup-bad', \
                 '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        params!["candidate-bad", "project-1", capture_id],
    );
    assert!(
        bad.is_err(),
        "the status CHECK must reject an unknown status"
    );
    drop(connection);

    ProjectRepository::remove(&store, "project-1").expect("remove project");
    let connection = Connection::open(&database).expect("open raw connection");
    assert_eq!(
        row_count(&connection, "decision_candidates"),
        0,
        "deleting the project cascades its candidates"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn migration_0005_applies_on_fresh_and_upgraded_databases() {
    let root = temporary_directory("migration");
    let database = root.join("app.db");
    SqliteStore::open(&database).expect("open store");

    {
        let connection = Connection::open(&database).expect("open raw connection");
        assert!(table_exists(&connection, "decision_candidates"));
        let versions: i64 = connection
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .expect("count");
        assert!(versions >= 32);
        // This synthetic rollback removes a table newer triggers reference.
        // Drop/reapply only those triggers; real migrations remain forward-only.
        let triggers = connection
            .prepare(
                "SELECT name FROM sqlite_master WHERE type='trigger'
            AND sql LIKE '%decision_candidates%'",
            )
            .expect("triggers")
            .query_map([], |r| r.get::<_, String>(0))
            .expect("query")
            .collect::<Result<Vec<_>, _>>()
            .expect("names");
        for name in triggers {
            connection
                .execute_batch(&format!("DROP TRIGGER \"{}\"", name.replace('"', "\"\"")))
                .expect("drop fixture trigger");
        }
        connection
            .execute_batch(
                "DROP TABLE decision_candidates;
                 ALTER TABLE engineering_decisions DROP COLUMN qualifiers;
                 ALTER TABLE decision_revisions DROP COLUMN qualifiers;
                 ALTER TABLE context_claims DROP COLUMN qualifiers;
                 ALTER TABLE claim_suggestions DROP COLUMN qualifiers;
                 DELETE FROM schema_migrations WHERE version IN (5, 15, 25);",
            )
            .expect("simulate version 4");
    }

    let store = SqliteStore::open(&database).expect("reopen and migrate");
    Connection::open(&database)
        .expect("raw")
        .execute_batch(
            include_str!("../src/migrations/0037_incremental_review.sql")
                .split("CREATE TRIGGER")
                .skip(1)
                .map(|part| format!("CREATE TRIGGER IF NOT EXISTS{part}"))
                .collect::<String>()
                .as_str(),
        )
        .expect("restore fixture triggers");
    seed_capture(&store, "upgrade-capture", Vec::new());
    let mut upgraded = candidate("upgrade-candidate", "upgrade-capture", "upgrade-dedup");
    upgraded.qualifiers = "[{\"kind\":\"scope\",\"text\":\"offline only\"}]".into();
    store
        .insert_candidates(&[upgraded.clone()])
        .expect("adapter writes upgraded candidate");
    let connection = Connection::open(&database).expect("open raw connection");
    assert!(table_exists(&connection, "decision_candidates"));
    let actual: (String, String, f64, String) = connection
        .query_row(
            "SELECT qualifiers,kind,significance,criteria FROM decision_candidates WHERE id=?1",
            [&upgraded.id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .unwrap();
    assert_eq!(
        actual,
        (
            upgraded.qualifiers,
            upgraded.kind,
            upgraded.significance,
            upgraded.criteria
        )
    );
    let versions: i64 = connection
        .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .expect("count");
    let distinct: i64 = connection
        .query_row(
            "SELECT COUNT(DISTINCT version) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
        .expect("count distinct");
    assert!(versions >= 32);
    assert_eq!(distinct, versions);

    let _ = std::fs::remove_dir_all(&root);
}
