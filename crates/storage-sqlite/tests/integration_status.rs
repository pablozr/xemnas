//! Settings → OpenCode status over SQLite: the adapter version recorded by
//! migration 0009, per-adapter aggregation and the upgrade path from v8.

use application::captures::{
    CaptureArtifactRecord, CaptureCheckpointRecord, CaptureReceiptRecord, CaptureRepository,
    CaptureWrite,
};
use application::integration::{
    Compatibility, Integration, IntegrationEnvironment, IntegrationState, IntegrationStore,
    LocalApiEndpoint,
};
use application::jobs::{JobRecord, JobState, ANALYZE_CAPTURE_KIND};
use application::projects::{ProjectRecord, ProjectRepository};
use rusqlite::Connection;
use storage_sqlite::SqliteStore;

const PROJECT_LOCATION: &str = "C:/synthetic/project";

fn temporary_directory(tag: &str) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let directory = std::env::temp_dir().join(format!(
        "xemnas-integration-status-{tag}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("create temporary directory");
    directory
}

fn open(root: &std::path::Path) -> SqliteStore {
    let store = SqliteStore::open(root.join("app.db")).expect("open store");
    store
        .insert(&ProjectRecord::new(
            "project-1".to_string(),
            PROJECT_LOCATION.to_string(),
            "2026-01-01T00:00:00Z".to_string(),
        ))
        .expect("seed project");
    store
}

fn capture(
    store: &SqliteStore,
    capture_id: &str,
    adapter: &str,
    version: &str,
    session: &str,
    at: &str,
) {
    store
        .insert_capture(&CaptureWrite {
            receipt: CaptureReceiptRecord {
                capture_id: capture_id.to_string(),
                idempotency_key: format!("key-{capture_id}"),
                canonical_path: PROJECT_LOCATION.to_string(),
                received_at: at.to_string(),
                artifact_count: 1,
            },
            artifacts: vec![CaptureArtifactRecord {
                capture_id: capture_id.to_string(),
                artifact_id: format!("artifact-{capture_id}"),
                kind: "user_text".to_string(),
                content: "synthetic".to_string(),
                metadata: "{}".to_string(),
                fingerprint: format!("{:0>64}", capture_id.len()),
            }],
            job: JobRecord {
                id: format!("job-{capture_id}"),
                kind: ANALYZE_CAPTURE_KIND.to_string(),
                payload: capture_id.to_string(),
                state: JobState::Queued,
                idempotent: true,
                attempts: 0,
                last_error: None,
                created_at: at.to_string(),
                updated_at: at.to_string(),
            },
            checkpoint: CaptureCheckpointRecord {
                adapter: adapter.to_string(),
                adapter_version: version.to_string(),
                session_id: session.to_string(),
                message_id: format!("message-{capture_id}"),
                capture_id: capture_id.to_string(),
                observed_at: at.to_string(),
                updated_at: at.to_string(),
            },
        })
        .expect("insert capture");
}

#[test]
fn aggregates_sessions_and_latest_version_per_adapter() {
    let root = temporary_directory("aggregate");
    let store = open(&root);
    capture(
        &store,
        "c1",
        "opencode",
        "0.1.0",
        "session-a",
        "2026-01-01T00:00:00Z",
    );
    capture(
        &store,
        "c2",
        "opencode",
        "0.2.0",
        "session-b",
        "2026-01-03T00:00:00Z",
    );
    capture(
        &store,
        "c3",
        "other",
        "9.9.9",
        "session-z",
        "2026-01-02T00:00:00Z",
    );

    let rows = store.adapter_activity().expect("activity");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].adapter, "opencode");
    assert_eq!(rows[0].adapter_version.as_deref(), Some("0.2.0"));
    assert_eq!(rows[0].sessions, 2);
    assert_eq!(rows[0].last_received_at, "2026-01-03T00:00:00Z");
    assert_eq!(rows[1].adapter, "other");

    let integration = Integration::new(
        store,
        IntegrationEnvironment {
            data_dir: root.clone(),
            outbox_dir: root.join("outbox"),
            runtime_dir: root.join("state"),
            api: Some(LocalApiEndpoint {
                port: 1,
                protocol_version: 1,
            }),
        },
    );
    let status = integration.status().expect("status");
    assert_eq!(status.state, IntegrationState::Receiving);
    assert_eq!(status.adapters[0].compatibility, Compatibility::Compatible);
    let json = application::serde_json::to_string(&status).expect("serialize");
    assert!(
        !json.contains("session-a"),
        "session ids never leave the store"
    );
    assert!(
        !json.contains("message-c1"),
        "message ids never leave the store"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn checkpoints_from_before_0009_report_an_unknown_version() {
    let root = temporary_directory("upgrade");
    let database = root.join("app.db");
    {
        let store = open(&root);
        capture(
            &store,
            "c1",
            "opencode",
            "0.1.0",
            "session-a",
            "2026-01-01T00:00:00Z",
        );
    }
    {
        let connection = Connection::open(&database).expect("raw");
        connection
            .execute_batch(
                "CREATE TABLE legacy AS SELECT adapter, session_id, message_id, capture_id, \
                     observed_at, updated_at FROM adapter_checkpoints; \
                 DROP TABLE adapter_checkpoints; \
                 CREATE TABLE adapter_checkpoints (adapter TEXT NOT NULL, \
                     session_id TEXT NOT NULL, message_id TEXT NOT NULL, \
                     capture_id TEXT NOT NULL, observed_at TEXT NOT NULL, \
                     updated_at TEXT NOT NULL, PRIMARY KEY (adapter, session_id), \
                     FOREIGN KEY (capture_id) REFERENCES capture_receipts (capture_id) \
                     ON DELETE CASCADE); \
                 INSERT INTO adapter_checkpoints SELECT * FROM legacy; \
                 DROP TABLE legacy; \
                 DELETE FROM schema_migrations WHERE version = 9;",
            )
            .expect("simulate version 8");
    }

    let store = SqliteStore::open(&database).expect("reopen and migrate");
    let rows = store.adapter_activity().expect("activity");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].adapter_version, None);
    let version: i64 = Connection::open(&database)
        .expect("raw")
        .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .expect("version");
    assert_eq!(version, 16);

    capture(
        &store,
        "c2",
        "opencode",
        "0.3.0",
        "session-a",
        "2026-01-02T00:00:00Z",
    );
    let rows = store.adapter_activity().expect("activity");
    assert_eq!(rows[0].adapter_version.as_deref(), Some("0.3.0"));

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn empty_database_has_no_activity() {
    let root = temporary_directory("empty");
    let store = open(&root);
    assert!(store.adapter_activity().expect("activity").is_empty());
    let _ = std::fs::remove_dir_all(&root);
}
