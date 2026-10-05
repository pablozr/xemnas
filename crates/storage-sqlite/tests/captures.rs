//! Capture persistence tests: migration 0003 on a fresh and upgraded database,
//! the `idempotency_key` unique constraint and transactional rollback.

use application::captures::{
    CaptureArtifactRecord, CaptureCheckpointRecord, CaptureError, CaptureReceiptRecord,
    CaptureRepository, CaptureWrite,
};
use application::jobs::{JobRecord, JobState, ANALYZE_CAPTURE_KIND};
use application::projects::{ProjectRecord, ProjectRepository};
use rusqlite::{params, Connection, OptionalExtension};
use storage_sqlite::SqliteStore;

mod rewind;

/// Canonical path the seeded project and every `CaptureWrite` share.
const PROJECT_LOCATION: &str = "C:/synthetic/project";

/// Inserts the project the capture writes must be allowed for.
fn seed_project(store: &SqliteStore) {
    store
        .insert(&ProjectRecord::new(
            "project-1".to_string(),
            PROJECT_LOCATION.to_string(),
            "2026-01-01T00:00:00Z".to_string(),
        ))
        .expect("seed project");
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
        "xemnas-captures-{tag}-{}-{nanos}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir_all(&directory).expect("create temporary directory");
    directory
}

fn artifact(
    capture_id: &str,
    artifact_id: &str,
    kind: &str,
    fingerprint: u8,
) -> CaptureArtifactRecord {
    CaptureArtifactRecord {
        capture_id: capture_id.to_string(),
        artifact_id: artifact_id.to_string(),
        kind: kind.to_string(),
        content: format!("synthetic content {fingerprint}"),
        metadata: "{}".to_string(),
        fingerprint: format!("{:064x}", fingerprint as u128),
    }
}

fn write(
    capture_id: &str,
    idempotency_key: &str,
    artifacts: Vec<CaptureArtifactRecord>,
) -> CaptureWrite {
    let timestamp = "2026-01-01T00:00:00Z".to_string();
    let checkpoint = CaptureCheckpointRecord {
        adapter: "opencode".to_string(),
        adapter_version: "0.1.0".to_string(),
        session_id: "session-1".to_string(),
        message_id: "message-1".to_string(),
        capture_id: capture_id.to_string(),
        observed_at: timestamp.clone(),
        updated_at: timestamp.clone(),
    };
    CaptureWrite {
        receipt: CaptureReceiptRecord {
            capture_id: capture_id.to_string(),
            idempotency_key: idempotency_key.to_string(),
            canonical_path: PROJECT_LOCATION.to_string(),
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
        checkpoint,
    }
}

/// Reads `(message_id, capture_id, observed_at, updated_at)` for a session.
fn checkpoint_row(
    connection: &Connection,
    adapter: &str,
    session_id: &str,
) -> Option<(String, String, String, String)> {
    connection
        .query_row(
            "SELECT message_id, capture_id, observed_at, updated_at \
             FROM adapter_checkpoints WHERE adapter = ?1 AND session_id = ?2",
            params![adapter, session_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()
        .expect("query checkpoint")
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

fn row_count(connection: &Connection, table: &str) -> i64 {
    connection
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .expect("count rows")
}

#[test]
fn fresh_database_applies_capture_migration() {
    let root = temporary_directory("fresh");
    let database = root.join("app.db");
    SqliteStore::open(&database).expect("open store");

    let connection = Connection::open(&database).expect("open raw connection");
    assert!(table_exists(&connection, "capture_receipts"));
    assert!(table_exists(&connection, "capture_artifacts"));
    assert!(table_exists(&connection, "adapter_checkpoints"));
    let versions: i64 = connection
        .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .expect("count migrations");
    let distinct: i64 = connection
        .query_row(
            "SELECT COUNT(DISTINCT version) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
        .expect("count distinct migrations");
    assert_eq!(versions, distinct);
    assert_eq!(versions, 39);

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn upgrade_reapplies_the_missing_migrations() {
    let root = temporary_directory("upgrade");
    let database = root.join("app.db");

    {
        let _ = SqliteStore::open(&database).expect("open store");
        let connection = Connection::open(&database).expect("open raw connection");
        rewind::rewind(&connection, 3);
    }

    SqliteStore::open(&database).expect("open store and migrate");
    let connection = Connection::open(&database).expect("open raw connection");
    assert!(table_exists(&connection, "adapter_checkpoints"));
    assert!(table_exists(&connection, "decision_candidates"));
    let versions: i64 = connection
        .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .expect("count migrations");
    let distinct: i64 = connection
        .query_row(
            "SELECT COUNT(DISTINCT version) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
        .expect("count distinct migrations");
    assert_eq!(
        versions, 39,
        "0004, 0005, 0006 and 0008 to 0040 must be re-applied on upgrade"
    );
    assert_eq!(distinct, 39);

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn idempotency_key_is_unique() {
    let root = temporary_directory("unique");
    let store = SqliteStore::open(root.join("app.db")).expect("open store");
    seed_project(&store);

    store
        .insert_capture(&write(
            "capture-1",
            "key-1",
            vec![artifact("capture-1", "artifact-1", "user_text", 1)],
        ))
        .expect("first insert");
    let duplicate = store.insert_capture(&write(
        "capture-2",
        "key-1",
        vec![artifact("capture-2", "artifact-2", "user_text", 2)],
    ));
    assert_eq!(duplicate, Err(CaptureError::DuplicateIdempotencyKey));

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn duplicate_artifact_id_rolls_the_whole_capture_back() {
    let root = temporary_directory("rollback");
    let database = root.join("app.db");
    let store = SqliteStore::open(&database).expect("open store");
    seed_project(&store);
    let connection = Connection::open(&database).expect("open raw connection");
    let jobs_before: Vec<JobRecord> = {
        use application::jobs::JobRepository;
        JobRepository::list(&store).expect("snapshot existing jobs")
    };

    let result = store.insert_capture(&write(
        "capture-duplicate",
        "key-duplicate",
        vec![
            artifact("capture-duplicate", "artifact-dup", "user_text", 1),
            artifact("capture-duplicate", "artifact-dup", "assistant_text", 2),
        ],
    ));
    assert_eq!(result, Err(CaptureError::DuplicateArtifact));

    assert_eq!(row_count(&connection, "capture_receipts"), 0);
    assert_eq!(row_count(&connection, "capture_artifacts"), 0);
    {
        use application::jobs::JobRepository;
        assert_eq!(
            JobRepository::list(&store).expect("jobs after rollback"),
            jobs_before
        );
    }

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn ingest_writes_adapter_checkpoint() {
    let root = temporary_directory("checkpoint");
    let database = root.join("app.db");
    let store = SqliteStore::open(&database).expect("open store");
    seed_project(&store);

    store
        .insert_capture(&write(
            "capture-1",
            "key-1",
            vec![artifact("capture-1", "artifact-1", "user_text", 1)],
        ))
        .expect("insert");

    let connection = Connection::open(&database).expect("open raw connection");
    let row = checkpoint_row(&connection, "opencode", "session-1").expect("checkpoint row");
    assert_eq!(row.0, "message-1");
    assert_eq!(row.1, "capture-1");
    assert_eq!(row.2, "2026-01-01T00:00:00Z");
    assert_eq!(row.3, "2026-01-01T00:00:00Z");

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn same_session_advances_checkpoint() {
    let root = temporary_directory("checkpoint-advance");
    let database = root.join("app.db");
    let store = SqliteStore::open(&database).expect("open store");
    seed_project(&store);

    store
        .insert_capture(&write(
            "capture-a",
            "key-a",
            vec![artifact("capture-a", "artifact-a", "user_text", 1)],
        ))
        .expect("first capture");

    let mut second = write(
        "capture-b",
        "key-b",
        vec![artifact("capture-b", "artifact-b", "user_text", 2)],
    );
    second.checkpoint.message_id = "message-2".to_string();
    second.checkpoint.observed_at = "2026-01-02T00:00:00Z".to_string();
    second.checkpoint.updated_at = "2026-01-02T00:00:00Z".to_string();
    store.insert_capture(&second).expect("second capture");

    let connection = Connection::open(&database).expect("open raw connection");
    assert_eq!(row_count(&connection, "adapter_checkpoints"), 1);
    let row = checkpoint_row(&connection, "opencode", "session-1").expect("checkpoint row");
    assert_eq!(row.0, "message-2");
    assert_eq!(row.1, "capture-b");
    assert_eq!(row.2, "2026-01-02T00:00:00Z");
    assert_eq!(row.3, "2026-01-02T00:00:00Z");

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn new_session_adds_checkpoint_row() {
    let root = temporary_directory("checkpoint-session");
    let database = root.join("app.db");
    let store = SqliteStore::open(&database).expect("open store");
    seed_project(&store);

    store
        .insert_capture(&write(
            "capture-1",
            "key-1",
            vec![artifact("capture-1", "artifact-1", "user_text", 1)],
        ))
        .expect("first session");

    let mut other = write(
        "capture-2",
        "key-2",
        vec![artifact("capture-2", "artifact-2", "user_text", 2)],
    );
    other.checkpoint.session_id = "session-2".to_string();
    other.checkpoint.message_id = "message-other".to_string();
    other.checkpoint.capture_id = "capture-2".to_string();
    store.insert_capture(&other).expect("second session");

    let connection = Connection::open(&database).expect("open raw connection");
    assert_eq!(row_count(&connection, "adapter_checkpoints"), 2);
    assert!(checkpoint_row(&connection, "opencode", "session-1").is_some());
    let row = checkpoint_row(&connection, "opencode", "session-2").expect("second session row");
    assert_eq!(row.0, "message-other");
    assert_eq!(row.1, "capture-2");

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn failed_ingest_leaves_no_checkpoint() {
    let root = temporary_directory("checkpoint-rollback");
    let database = root.join("app.db");
    let store = SqliteStore::open(&database).expect("open store");

    let rejected = store.insert_capture(&write(
        "capture-missing",
        "key-missing",
        vec![artifact("capture-missing", "artifact-1", "user_text", 1)],
    ));
    assert_eq!(rejected, Err(CaptureError::ProjectNotRegistered));

    seed_project(&store);
    let duplicate = store.insert_capture(&write(
        "capture-duplicate",
        "key-duplicate",
        vec![
            artifact("capture-duplicate", "artifact-dup", "user_text", 1),
            artifact("capture-duplicate", "artifact-dup", "assistant_text", 2),
        ],
    ));
    assert_eq!(duplicate, Err(CaptureError::DuplicateArtifact));

    let connection = Connection::open(&database).expect("open raw connection");
    assert_eq!(row_count(&connection, "adapter_checkpoints"), 0);

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn replay_keeps_checkpoint_idempotent() {
    let root = temporary_directory("checkpoint-replay");
    let database = root.join("app.db");
    let store = SqliteStore::open(&database).expect("open store");
    seed_project(&store);

    let write = write(
        "capture-1",
        "key-1",
        vec![artifact("capture-1", "artifact-1", "user_text", 1)],
    );
    store.insert_capture(&write).expect("first insert");

    let replay = store.insert_capture(&write);
    assert_eq!(replay, Err(CaptureError::DuplicateIdempotencyKey));

    let connection = Connection::open(&database).expect("open raw connection");
    assert_eq!(row_count(&connection, "adapter_checkpoints"), 1);
    let row = checkpoint_row(&connection, "opencode", "session-1").expect("checkpoint row");
    assert_eq!(row.0, "message-1");
    assert_eq!(row.1, "capture-1");

    let _ = std::fs::remove_dir_all(&root);
}
