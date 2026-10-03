//! Persisted-job tests: the happy path, the failure path (handler error and
//! panic), atomic claiming, compare-and-set transitions, recovery after a
//! restart, and the worker lifecycle.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use application::jobs::{JobError, JobFailure, JobRecord, JobRepository, JobState, Jobs};
use storage_sqlite::SqliteStore;

/// Creates a unique temporary directory for a test.
fn temporary_directory(tag: &str) -> std::path::PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let directory = std::env::temp_dir().join(format!(
        "xemnas-jobs-{tag}-{}-{nanos}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir_all(&directory).expect("create temporary directory");
    directory
}

/// Opens a store in a fresh temporary directory and returns it with that root.
fn store_in(tag: &str) -> (SqliteStore, std::path::PathBuf) {
    let root = temporary_directory(tag);
    let store = SqliteStore::open(root.join("app.db")).expect("open store");
    (store, root)
}

/// Builds a persisted job row with explicit ordering fields.
fn record(id: &str, kind: &str, state: JobState, idempotent: bool, created_at: &str) -> JobRecord {
    JobRecord {
        id: id.to_string(),
        kind: kind.to_string(),
        payload: "{}".to_string(),
        state,
        idempotent,
        attempts: 0,
        last_error: None,
        created_at: created_at.to_string(),
        updated_at: created_at.to_string(),
    }
}

#[test]
fn enqueue_persists_before_execution() {
    let (store, root) = store_in("enqueue-before-execute");
    let mut jobs = Jobs::new(store.clone());
    jobs.register("analysis", Arc::new(|_: &JobRecord| Ok(())));

    let job = jobs.enqueue("analysis", "{}", true).expect("enqueue job");

    let reopened = SqliteStore::open(root.join("app.db")).expect("reopen store");
    let persisted = reopened
        .get(&job.id)
        .expect("get job")
        .expect("job row must exist");
    assert_eq!(persisted.state, JobState::Queued);
    assert_eq!(persisted.kind, "analysis");

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn claim_next_is_atomic_and_skips_unregistered_kinds() {
    let (store, root) = store_in("claim");
    store
        .insert(&record(
            "known",
            "analysis",
            JobState::Queued,
            true,
            "2026-01-01T00:00:00Z",
        ))
        .expect("insert known");
    store
        .insert(&record(
            "unknown",
            "desconhecido",
            JobState::Queued,
            true,
            "2026-01-01T00:00:01Z",
        ))
        .expect("insert unknown");

    let kinds = vec!["analysis".to_string()];
    let first = store.claim_next(&kinds).expect("first claim");
    assert_eq!(first.expect("a claimable job").id, "known");
    assert!(
        store.claim_next(&kinds).expect("second claim").is_none(),
        "a claimed job must not be claimable again"
    );
    assert_eq!(
        store
            .get("unknown")
            .expect("get unknown")
            .expect("row")
            .state,
        JobState::Queued,
        "a job without a handler must stay queued, never disappear"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn happy_path_reaches_completed_with_one_attempt() {
    let (store, root) = store_in("happy");
    let mut jobs = Jobs::new(store.clone());
    jobs.register("analysis", Arc::new(|_: &JobRecord| Ok(())));

    let job = jobs.enqueue("analysis", "{}", true).expect("enqueue");
    let outcome = jobs.run_next().expect("run").expect("a job ran");
    assert_eq!(outcome.state, JobState::Completed);
    assert_eq!(outcome.attempts, 1);
    assert_eq!(
        store.get(&job.id).expect("get").expect("row").state,
        JobState::Completed
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn handler_error_is_sanitized_and_does_not_include_payload() {
    let (store, root) = store_in("handler-error");
    let mut jobs = Jobs::new(store.clone());
    jobs.register(
        "analysis",
        Arc::new(|_: &JobRecord| Err(JobFailure::Failed)),
    );

    let secret = "SECRET-MARKER-123";
    let job = jobs.enqueue("analysis", secret, true).expect("enqueue");
    let outcome = jobs.run_next().expect("run").expect("a job ran");
    assert_eq!(outcome.state, JobState::Failed);

    let stored = store.get(&job.id).expect("get").expect("row");
    let last_error = stored.last_error.expect("a diagnostic is recorded");
    assert_eq!(
        last_error,
        JobFailure::Failed.as_str(),
        "last_error must be exactly the fixed product message"
    );
    assert!(
        !last_error.contains(secret),
        "the payload must never appear in last_error: {last_error}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn handler_panic_is_contained_and_the_next_job_still_runs() {
    let (store, root) = store_in("panic");
    let mut jobs = Jobs::new(store.clone());
    jobs.register(
        "analysis",
        Arc::new(|record: &JobRecord| {
            if record.payload == "panic" {
                panic!("synthetic handler panic");
            }
            Ok(())
        }),
    );

    let mut failing = record(
        "job-a",
        "analysis",
        JobState::Queued,
        true,
        "2026-01-01T00:00:00Z",
    );
    failing.payload = "panic".to_string();
    store.insert(&failing).expect("insert failing");
    store
        .insert(&record(
            "job-b",
            "analysis",
            JobState::Queued,
            true,
            "2026-01-01T00:00:01Z",
        ))
        .expect("insert next");

    let first = jobs.run_next().expect("run").expect("first job ran");
    assert_eq!(first.state, JobState::Failed);
    let second = jobs.run_next().expect("run").expect("second job ran");
    assert_eq!(second.state, JobState::Completed);

    let failed = store.get("job-a").expect("get").expect("row");
    assert_eq!(failed.state, JobState::Failed);
    assert_eq!(
        failed.last_error.as_deref(),
        Some("o job falhou com pânico")
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn recover_requeues_interrupted_idempotent_jobs() {
    let (store, root) = store_in("recover-idempotent");
    store
        .insert(&record(
            "idem",
            "analysis",
            JobState::Running,
            true,
            "2026-01-01T00:00:00Z",
        ))
        .expect("insert running idempotent");

    let jobs = Jobs::new(store.clone());
    let report = jobs.recover().expect("recover");
    assert_eq!(report.requeued, 1);
    assert_eq!(report.failed, 0);

    let recovered = store.get("idem").expect("get").expect("row");
    assert_eq!(recovered.state, JobState::Queued);
    assert_eq!(recovered.last_error, None);

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn recover_fails_interrupted_non_idempotent_jobs_with_a_diagnostic() {
    let (store, root) = store_in("recover-non-idempotent");
    store
        .insert(&record(
            "noidem",
            "analysis",
            JobState::Running,
            false,
            "2026-01-01T00:00:00Z",
        ))
        .expect("insert running non-idempotent");

    let jobs = Jobs::new(store.clone());
    let report = jobs.recover().expect("recover");
    assert_eq!(report.requeued, 0);
    assert_eq!(report.failed, 1);

    let recovered = store.get("noidem").expect("get").expect("row");
    assert_eq!(recovered.state, JobState::Failed);
    let message = recovered.last_error.expect("a diagnostic is recorded");
    assert!(
        message.contains("idempotente"),
        "unexpected message: {message}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn recover_leaves_queued_jobs_untouched() {
    let (store, root) = store_in("recover-queued");
    store
        .insert(&record(
            "waiting",
            "analysis",
            JobState::Queued,
            true,
            "2026-01-01T00:00:00Z",
        ))
        .expect("insert queued");

    let jobs = Jobs::new(store.clone());
    let report = jobs.recover().expect("recover");
    assert_eq!(report.requeued, 0);
    assert_eq!(report.failed, 0);
    assert_eq!(
        store.get("waiting").expect("get").expect("row").state,
        JobState::Queued
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn cancel_only_accepts_queued_jobs() {
    let (store, root) = store_in("cancel");
    let mut jobs = Jobs::new(store.clone());
    jobs.register("analysis", Arc::new(|_: &JobRecord| Ok(())));

    let job = jobs.enqueue("analysis", "{}", true).expect("enqueue");
    jobs.cancel(&job.id).expect("cancel queued job");
    assert_eq!(
        store.get(&job.id).expect("get").expect("row").state,
        JobState::Cancelled
    );
    assert_eq!(
        jobs.cancel(&job.id),
        Err(JobError::InvalidTransition {
            from: JobState::Cancelled,
            to: JobState::Cancelled,
        })
    );

    store
        .insert(&record(
            "running",
            "analysis",
            JobState::Running,
            true,
            "2026-01-01T00:00:00Z",
        ))
        .expect("insert running");
    assert_eq!(
        jobs.cancel("running"),
        Err(JobError::InvalidTransition {
            from: JobState::Running,
            to: JobState::Cancelled,
        })
    );
    assert_eq!(jobs.cancel("missing"), Err(JobError::NotFound));

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn transition_is_compare_and_set() {
    let (store, root) = store_in("transition-cas");
    store
        .insert(&record(
            "cas",
            "analysis",
            JobState::Queued,
            true,
            "2026-01-01T00:00:00Z",
        ))
        .expect("insert");

    assert!(!store
        .transition("cas", JobState::Running, JobState::Completed, None)
        .expect("transition from wrong state"));
    assert!(store
        .transition("cas", JobState::Queued, JobState::Cancelled, None)
        .expect("valid transition"));
    assert!(!store
        .transition("cas", JobState::Queued, JobState::Cancelled, None)
        .expect("repeat transition"));
    assert_eq!(
        store.get("cas").expect("get").expect("row").state,
        JobState::Cancelled
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn worker_processes_enqueued_job_and_stops_cleanly() {
    let (store, root) = store_in("worker");
    let mut jobs = Jobs::new(store.clone());
    let runs = Arc::new(AtomicUsize::new(0));
    let runs_handler = runs.clone();
    jobs.register(
        "analysis",
        Arc::new(move |_: &JobRecord| {
            runs_handler.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }),
    );

    let handle = jobs.spawn_worker();
    let job = jobs.enqueue("analysis", "{}", true).expect("enqueue");

    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let state = store.get(&job.id).expect("get").expect("row").state;
        if state == JobState::Completed {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "job did not complete in time; last state: {state:?}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(runs.load(Ordering::SeqCst), 1);

    handle.stop();
    handle.join().expect("worker stops cleanly");

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn reprocess_requeues_a_failed_job_and_clears_the_diagnostic() {
    let (store, root) = store_in("reprocess");
    let mut jobs = Jobs::new(store.clone());
    jobs.register(
        "analysis",
        Arc::new(|_: &JobRecord| Err(JobFailure::Failed)),
    );

    let job = jobs.enqueue("analysis", "{}", true).expect("enqueue");
    let outcome = jobs.run_next().expect("run").expect("a job ran");
    assert_eq!(outcome.state, JobState::Failed);
    assert_eq!(outcome.attempts, 1);
    let failed = store.get(&job.id).expect("get").expect("row");
    assert_eq!(
        failed.last_error.as_deref(),
        Some(JobFailure::Failed.as_str())
    );

    jobs.reprocess(&job.id).expect("reprocess a failed job");
    let queued = store.get(&job.id).expect("get").expect("row");
    assert_eq!(queued.state, JobState::Queued);
    assert_eq!(
        queued.last_error, None,
        "reprocess must clear the diagnostic"
    );

    let claimed = store
        .claim_next(&["analysis".to_string()])
        .expect("claim")
        .expect("a claimable job");
    assert_eq!(claimed.attempts, 2, "attempts keep growing on the re-claim");
    assert!(matches!(
        jobs.reprocess(&job.id),
        Err(JobError::InvalidTransition {
            from: JobState::Running,
            to: JobState::Queued,
        })
    ));
    assert!(matches!(jobs.reprocess("missing"), Err(JobError::NotFound)));

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn concurrent_claims_never_hand_out_a_job_twice() {
    let (store, root) = store_in("concurrent-claims");
    const JOBS: usize = 200;
    for index in 0..JOBS {
        store
            .insert(&record(
                &format!("job-{index:03}"),
                "analysis",
                JobState::Queued,
                true,
                &format!("2026-01-01T00:00:{:02}Z", index % 60),
            ))
            .expect("insert queued job");
    }

    // Half of the workers share the store handle, the other half open their
    // own connection to the same file, as a second process would.
    let kinds = vec!["analysis".to_string()];
    let workers: Vec<_> = (0..8)
        .map(|worker| {
            let store = if worker % 2 == 0 {
                store.clone()
            } else {
                SqliteStore::open(root.join("app.db")).expect("open a second connection")
            };
            let kinds = kinds.clone();
            std::thread::spawn(move || {
                let mut claimed = Vec::new();
                while let Some(job) = store.claim_next(&kinds).expect("claim") {
                    assert!(store
                        .transition(&job.id, JobState::Running, JobState::Completed, None)
                        .expect("complete"));
                    claimed.push(job.id);
                }
                claimed
            })
        })
        .collect();

    let mut claimed: Vec<String> = workers
        .into_iter()
        .flat_map(|worker| worker.join().expect("worker thread"))
        .collect();
    assert_eq!(claimed.len(), JOBS, "every job is claimed exactly once");
    claimed.sort();
    claimed.dedup();
    assert_eq!(claimed.len(), JOBS, "no job was claimed twice");
    assert!(store
        .list()
        .expect("list")
        .iter()
        .all(|job| job.state == JobState::Completed && job.attempts == 1));

    let _ = std::fs::remove_dir_all(&root);
}
