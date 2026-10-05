//! Purging a running refresh cancels only that job, not the shared worker.
mod support;

use application::jobs::{JobError, JobEvent, JobRepository, JobState, Jobs};
use application::observations::refresh::{RefreshObservations, REFRESH_OBSERVATIONS_KIND};
use application::observations::{
    CheckStatus, ObservationError, ObservationReader, SourceReadRequest, SourceReadResult,
};
use application::projects::ProjectRepository;
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

struct BlockedReader {
    entered: mpsc::Sender<()>,
    release: Mutex<mpsc::Receiver<()>>,
}

impl ObservationReader for BlockedReader {
    fn read_source(
        &self,
        request: &SourceReadRequest,
    ) -> Result<SourceReadResult, ObservationError> {
        if request.project_id == "a" && request.project_relative_path == "Cargo.toml" {
            self.entered.send(()).expect("notify blocked refresh");
            self.release
                .lock()
                .expect("release lock")
                .recv_timeout(Duration::from_secs(10))
                .expect("release refresh");
            return Ok(SourceReadResult {
                status: CheckStatus::Verified,
                bytes: b"[package]\nname = 'purged'\nversion = '0.1.0'\n".to_vec(),
            });
        }
        Ok(SourceReadResult {
            status: CheckStatus::Missing,
            bytes: Vec::new(),
        })
    }
}

#[test]
fn purged_blocked_refresh_does_not_stop_worker_or_recreate_data() {
    let test = support::open("jobs-purge-worker", &["a", "b"]);
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let handler = RefreshObservations::new(
        test.store.clone(),
        BlockedReader {
            entered: entered_tx,
            release: Mutex::new(release_rx),
        },
    );
    let mut jobs = Jobs::new(test.store.clone());
    jobs.register(
        REFRESH_OBSERVATIONS_KIND,
        Arc::new(move |job| handler.run(job)),
    );
    let (event_tx, event_rx) = mpsc::channel();
    jobs.observe_with(move |event| event_tx.send(event).expect("receive worker event"));
    let worker = jobs.spawn_worker();
    entered_rx
        .recv_timeout(Duration::from_secs(10))
        .expect("A refresh blocked");
    let running = JobRepository::list(&test.store)
        .expect("jobs")
        .into_iter()
        .find(|job| job.kind == REFRESH_OBSERVATIONS_KIND && job.payload == "a")
        .expect("A job");
    assert_eq!(running.state, JobState::Running);
    assert!(test.store.purge("a").expect("purge A"));
    release_tx.send(()).expect("release A");
    let first = event_rx.recv_timeout(Duration::from_secs(10));
    let second = event_rx.recv_timeout(Duration::from_secs(10));
    worker.stop();
    worker.join().expect("worker remained healthy");
    let JobEvent::Finished(cancelled) = first.expect("cancel event") else {
        panic!("expected cancellation");
    };
    assert_eq!(cancelled.job_id, running.id);
    assert_eq!(cancelled.kind, REFRESH_OBSERVATIONS_KIND);
    assert_eq!(cancelled.state, JobState::Cancelled);
    let JobEvent::Finished(completed) = second.expect("B event") else {
        panic!("expected completion");
    };
    assert_eq!(completed.state, JobState::Completed);
    let b = JobRepository::get(&test.store, &completed.job_id)
        .expect("get B")
        .expect("B exists");
    assert_eq!(b.payload, "b");
    assert!(JobRepository::get(&test.store, &running.id)
        .expect("get A")
        .is_none());
    assert!(ProjectRepository::get(&test.store, "a")
        .expect("project A")
        .is_none());
    let connection = rusqlite::Connection::open(test.root.join("app.db")).expect("raw DB");
    for table in [
        "observation_refresh",
        "observation_sources",
        "observation_records",
    ] {
        let count: i64 = connection
            .query_row(
                &format!("SELECT COUNT(*) FROM {table} WHERE project_id='a'"),
                [],
                |row| row.get(0),
            )
            .expect("count A data");
        assert_eq!(count, 0, "{table}");
    }
    assert_eq!(jobs.recover().expect("recover").requeued, 0);
}

#[test]
fn missing_job_with_live_project_and_unexpected_state_remain_errors() {
    for delete in [true, false] {
        let test = support::open("jobs-purge-conflict", &["a"]);
        let store = test.store.clone();
        let mut jobs = Jobs::new(test.store.clone());
        let database = test.root.join("app.db");
        jobs.register(
            REFRESH_OBSERVATIONS_KIND,
            Arc::new(move |job| {
                if delete {
                    rusqlite::Connection::open(&database)
                        .expect("raw DB")
                        .execute("DELETE FROM jobs WHERE id=?1", [&job.id])
                        .expect("delete job");
                } else {
                    assert!(store
                        .transition(&job.id, JobState::Running, JobState::Queued, None)
                        .expect("unexpected state"));
                }
                Ok(())
            }),
        );
        assert_eq!(
            jobs.run_next(),
            Err(JobError::InvalidTransition {
                from: JobState::Running,
                to: JobState::Completed,
            })
        );
    }
}

#[test]
fn terminal_storage_error_after_purge_is_not_masked() {
    let test = support::open("jobs-purge-storage-error", &["a"]);
    let store = test.store.clone();
    let database = test.root.join("app.db");
    let mut jobs = Jobs::new(test.store.clone());
    jobs.register(
        REFRESH_OBSERVATIONS_KIND,
        Arc::new(move |_| {
            store.purge("a").expect("purge");
            rusqlite::Connection::open(&database)
                .expect("raw DB")
                .execute_batch("DROP TABLE jobs;")
                .expect("reject terminal writes");
            Ok(())
        }),
    );
    assert!(matches!(jobs.run_next(), Err(JobError::Storage(_))));
}

#[test]
fn purge_cancels_failed_and_panicking_refresh_but_not_other_kinds() {
    for panic_mode in [false, true] {
        let test = support::open("jobs-purge-failure", &["a"]);
        let store = test.store.clone();
        let mut jobs = Jobs::new(test.store.clone());
        jobs.register(
            REFRESH_OBSERVATIONS_KIND,
            Arc::new(move |_| {
                store.purge("a").expect("purge");
                if panic_mode {
                    panic!("handler panic");
                }
                Err(application::jobs::JobFailure::Failed)
            }),
        );
        assert_eq!(
            jobs.run_next().expect("cancelled").expect("job").state,
            JobState::Cancelled
        );
    }
    let test = support::open("jobs-purge-other-kind", &["a"]);
    let record = test
        .store
        .claim_next(&[application::jobs::ANALYZE_CAPTURE_KIND.into()])
        .expect("claim")
        .expect("capture job");
    test.store.purge("a").expect("purge");
    assert!(!test
        .store
        .cancelled_by_project_purge(&record)
        .expect("check kind"));
}
