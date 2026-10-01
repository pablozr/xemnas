//! Assessment persistence tests: migration 0006 on a fresh and upgraded
//! database, a full round-trip, the receipt foreign key and the indexes.

use application::captures::{
    CaptureCheckpointRecord, CaptureReceiptRecord, CaptureRepository, CaptureWrite,
};
use application::extract::{AssessmentOutcome, AssessmentRecord, AssessmentStore};
use application::jobs::{JobRecord, JobState, ANALYZE_CAPTURE_KIND};
use application::projects::{ProjectRecord, ProjectRepository};
use rusqlite::Connection;
use storage_sqlite::SqliteStore;

const LOCATION: &str = "C:/synthetic/assessments";

fn temporary_directory(tag: &str) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let directory = std::env::temp_dir().join(format!(
        "xemnas-assessments-{tag}-{}-{nanos}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir_all(&directory).expect("create temporary directory");
    directory
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

fn index_exists(connection: &Connection, name: &str) -> bool {
    connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = ?1",
            [name],
            |row| row.get::<_, i64>(0),
        )
        .expect("query sqlite_master")
        > 0
}

fn row_count(connection: &Connection, table: &str) -> i64 {
    connection
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .expect("count rows")
}

/// Seeds a project, a receipt and its job so the assessment FKs can resolve.
fn seed_capture(store: &SqliteStore, capture_id: &str) {
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
            artifact_count: 0,
        },
        artifacts: Vec::new(),
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

fn assessment(capture_id: &str) -> AssessmentRecord {
    AssessmentRecord {
        id: format!("assessment-{capture_id}"),
        capture_id: capture_id.to_string(),
        job_id: Some(format!("job-{capture_id}")),
        profile_id: "profile-1".to_string(),
        adapter: "fake".to_string(),
        model: None,
        policy: "{\"kind\":\"fake\",\"enabled\":false}".to_string(),
        consent_preview_hash: None,
        input_hash: "hash-1".to_string(),
        started_at: "2026-01-01T00:00:00Z".to_string(),
        finished_at: "2026-01-01T00:00:01Z".to_string(),
        outcome: AssessmentOutcome::Ok,
        candidates: 2,
        inserted: 1,
        error_code: None,
    }
}

#[test]
fn migration_0006_applies_on_fresh_and_upgraded_databases() {
    let root = temporary_directory("migration");
    let database = root.join("app.db");
    SqliteStore::open(&database).expect("open store");

    {
        let connection = Connection::open(&database).expect("open raw connection");
        assert!(table_exists(&connection, "assessments"));
        assert!(index_exists(&connection, "idx_assessments_capture_id"));
        assert!(index_exists(&connection, "idx_assessments_started_at"));
        connection
            .execute_batch(
                "DROP TABLE assessments; DELETE FROM schema_migrations WHERE version = 6;",
            )
            .expect("simulate version 5");
    }

    SqliteStore::open(&database).expect("reopen and migrate");
    let connection = Connection::open(&database).expect("open raw connection");
    assert!(table_exists(&connection, "assessments"));
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
    assert_eq!(versions, 16);
    assert_eq!(distinct, 16);

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn assessment_round_trips_with_its_foreign_keys() {
    let root = temporary_directory("roundtrip");
    let store = SqliteStore::open(root.join("app.db")).expect("open store");
    seed_capture(&store, "capture-1");

    store
        .record_assessment(&assessment("capture-1"))
        .expect("record assessment");

    let connection = Connection::open(root.join("app.db")).expect("open raw connection");
    assert_eq!(row_count(&connection, "assessments"), 1);
    let row: (String, i64, i64, String, String, Option<String>) = connection
        .query_row(
            "SELECT outcome, candidates, inserted, profile_id, adapter, error_code \
             FROM assessments WHERE id = ?1",
            ["assessment-capture-1"],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .expect("query the assessment row");
    assert_eq!(row.0, "ok");
    assert_eq!((row.1, row.2), (2, 1));
    assert_eq!(row.3, "profile-1");
    assert_eq!(row.4, "fake");
    assert_eq!(row.5, None);

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn assessment_with_an_unknown_capture_is_rejected() {
    let root = temporary_directory("foreign-key");
    let store = SqliteStore::open(root.join("app.db")).expect("open store");

    let result = store.record_assessment(&assessment("missing-capture"));
    assert!(
        result.is_err(),
        "the receipt foreign key must reject an unknown capture"
    );

    let connection = Connection::open(root.join("app.db")).expect("open raw connection");
    assert_eq!(row_count(&connection, "assessments"), 0);

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn deleting_a_capture_cascades_its_assessments() {
    let root = temporary_directory("cascade");
    let store = SqliteStore::open(root.join("app.db")).expect("open store");
    seed_capture(&store, "capture-1");
    store
        .record_assessment(&assessment("capture-1"))
        .expect("record assessment");

    let connection = Connection::open(root.join("app.db")).expect("open raw connection");
    connection
        .execute(
            "DELETE FROM capture_receipts WHERE capture_id = ?1",
            ["capture-1"],
        )
        .expect("delete the receipt");
    assert_eq!(
        row_count(&connection, "assessments"),
        0,
        "deleting a capture must cascade its assessments"
    );

    let _ = std::fs::remove_dir_all(&root);
}
