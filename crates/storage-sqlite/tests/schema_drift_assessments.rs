//! Regression for an already-recorded historical migration 26 without its columns.
use application::extract::{AssessmentOutcome, AssessmentRecord, AssessmentStore};
use rusqlite::Connection;
use storage_sqlite::SqliteStore;

#[test]
fn historical_22_is_repaired_without_rewriting_history() {
    verify_historical_22(false);
}

#[test]
fn concurrent_wal_opens_repair_historical_22_once() {
    verify_historical_22(true);
}

fn verify_historical_22(concurrent: bool) {
    let root = std::env::temp_dir().join(format!(
        "xemnas-assessment-drift-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("synthetic.db");
    {
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch("PRAGMA journal_mode=WAL;")
            .unwrap();
        connection.execute_batch(
            "CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL);",
        ).unwrap();
        let mut files: Vec<_> =
            std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/src/migrations"))
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .collect();
        files.sort();
        for file in files {
            let name = file.file_name().unwrap().to_str().unwrap();
            let version: i64 = name[..4].parse().unwrap();
            if version > 26 {
                continue;
            }
            if version != 26 {
                connection
                    .execute_batch(&std::fs::read_to_string(&file).unwrap())
                    .unwrap();
            }
            connection
                .execute(
                    "INSERT INTO schema_migrations VALUES (?1, 'historical')",
                    [version],
                )
                .unwrap();
        }
        connection.execute_batch(
            "INSERT INTO capture_receipts VALUES ('capture', 'key', 'C:/synthetic', '2026-01-01', 0);
             INSERT INTO jobs (id, kind, payload, state, idempotent, attempts, last_error,
                created_at, updated_at) VALUES ('job', 'analyze_capture', 'capture', 'failed', 1, 3,
                'CaptureAnalysisFailed', '2026-01-01', '2026-01-02');
             INSERT INTO assessments VALUES ('legacy', 'capture', 'job', 'profile', 'adapter',
                'historical-model', 'historical-policy', NULL, 'hash', 'start', 'finish',
                'ok', 2, 1, NULL);",
        ).unwrap();
        let maximum: i64 = connection
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(maximum, 26);
    }
    let store = if concurrent {
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let workers: Vec<_> = (0..2)
            .map(|_| {
                let barrier = barrier.clone();
                let path = path.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    SqliteStore::open(path)
                })
            })
            .collect();
        let mut stores: Vec<_> = workers
            .into_iter()
            .map(|worker| worker.join().unwrap().expect("concurrent open succeeds"))
            .collect();
        stores.pop().unwrap()
    } else {
        SqliteStore::open(&path).unwrap()
    };
    let record = AssessmentRecord {
        id: "new".into(),
        capture_id: "capture".into(),
        job_id: Some("job".into()),
        profile_id: "profile".into(),
        adapter: "adapter".into(),
        model: None,
        policy: "policy".into(),
        consent_preview_hash: None,
        input_hash: "hash".into(),
        started_at: "start".into(),
        finished_at: "finish".into(),
        outcome: AssessmentOutcome::Ok,
        candidates: 1,
        inserted: 1,
        error_code: None,
        attempt: Some(3),
        reason: "candidates".into(),
        durable_count: 1,
        detail_count: 0,
    };
    store
        .record_assessment(&record)
        .expect("real AssessmentStore insert after reopen");
    drop(store);
    drop(SqliteStore::open(&path).unwrap());
    let connection = Connection::open(&path).unwrap();
    let legacy: (String, String, i64, i64, Option<i64>, String, i64, i64) = connection
        .query_row(
            "SELECT model,policy,candidates,inserted,attempt,reason,durable_count,detail_count
         FROM assessments WHERE id='legacy'",
            [],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                    r.get(7)?,
                ))
            },
        )
        .unwrap();
    assert_eq!(
        legacy,
        (
            "historical-model".into(),
            "historical-policy".into(),
            2,
            1,
            None,
            "unknown".into(),
            0,
            0
        )
    );
    let job: (String, i64, String) = connection
        .query_row(
            "SELECT state,attempts,last_error FROM jobs WHERE id='job'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(job, ("failed".into(), 3, "CaptureAnalysisFailed".into()));
    let receipt: (String, String, String, i64) = connection.query_row(
        "SELECT idempotency_key,canonical_path,received_at,artifact_count FROM capture_receipts",
        [], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)),
    ).unwrap();
    assert_eq!(
        receipt,
        ("key".into(), "C:/synthetic".into(), "2026-01-01".into(), 0)
    );
    let audit: (String, i64) = connection
        .query_row(
            "SELECT (SELECT applied_at FROM schema_migrations WHERE version=26),
                (SELECT COUNT(*) FROM schema_migrations WHERE version=39)",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(audit, ("historical".into(), 1));
    let attempt: i64 = connection
        .query_row("SELECT attempt FROM assessments WHERE id='new'", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(attempt, 3);
    drop(connection);
    std::fs::remove_dir_all(root).unwrap();
}
