//! Jobs use cases: persist work, run it off the UI thread, recover it on boot.
//!
//! Jobs are persisted before any execution (spec §9) and move through the five
//! explicit states of spec §11. Handlers run in a dedicated worker thread
//! spawned by [`Jobs::spawn_worker`]; the worker never touches the UI thread
//! (ASYNC-001). A handler error or panic becomes a `failed` job with a
//! sanitized message, so neither the UI nor the process is brought down
//! (spec §14, PRIV-001).

use std::cell::Cell;
use std::collections::HashMap;
use std::io::Write;
use std::panic::{catch_unwind, AssertUnwindSafe, PanicHookInfo};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use crate::clock::now_rfc3339;

thread_local! {
    /// Set only while a job handler runs on this thread.
    ///
    /// The panic hook installed by [`install_panic_sanitizer`] reads it to
    /// decide whether a panic payload must be suppressed (PRIV-001).
    static JOB_PANIC_SANITIZED: Cell<bool> = const { Cell::new(false) };
}

/// Returns whether the current thread is inside a job-handler call.
pub fn job_panic_is_sanitized() -> bool {
    JOB_PANIC_SANITIZED.with(Cell::get)
}

/// Sets the flag and returns its previous value.
fn replace_job_panic_sanitized(value: bool) -> bool {
    JOB_PANIC_SANITIZED.with(|flag| flag.replace(value))
}

/// Writes a panic report that has no parameter for the panic payload.
///
/// Only the thread name and the source location are included. The payload is
/// omitted on purpose: a handler may panic with prompts, diffs or credentials,
/// and PRIV-001 forbids that content from reaching a log. The source location
/// (`file:line`) is safe and useful for diagnosis.
pub fn write_sanitized_panic_report(
    writer: &mut impl Write,
    thread_name: Option<&str>,
    location: Option<&str>,
) -> std::io::Result<()> {
    writeln!(writer, "job handler panic; payload omitted by PRIV-001")?;
    writeln!(writer, "thread: {}", thread_name.unwrap_or("<unnamed>"))?;
    if let Some(location) = location {
        writeln!(writer, "location: {location}")?;
    }
    Ok(())
}

/// Installs the process panic hook that sanitizes job-handler panics.
///
/// A panic raised while a handler runs writes only the sanitized report to
/// stderr; any other panic delegates to the hook installed before, so the rest
/// of the application (including the UI toolkit) keeps its own behavior.
pub fn install_panic_sanitizer() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info: &PanicHookInfo<'_>| {
        if job_panic_is_sanitized() {
            let location = info.location().map(|location| location.to_string());
            let thread_name = std::thread::current().name().map(str::to_string);
            let mut stderr = std::io::stderr().lock();
            let _ = write_sanitized_panic_report(
                &mut stderr,
                thread_name.as_deref(),
                location.as_deref(),
            );
        } else {
            previous(info);
        }
    }));
}

/// One of the five explicit job states (spec §11).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JobState {
    /// Persisted and waiting for a worker.
    Queued,
    /// Claimed by a worker and currently executing.
    Running,
    /// Finished successfully.
    Completed,
    /// Finished with an error or panic.
    Failed,
    /// Cancelled before it started running.
    Cancelled,
}

impl JobState {
    /// Returns the literal persisted in the `state` column.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    /// Parses a persisted state literal.
    ///
    /// Returns [`JobError::Storage`] for an unknown literal, which can only be
    /// database corruption because the schema carries a `CHECK` constraint.
    // The inherent name mirrors `std::str::FromStr`, but that trait's error type
    // would be opaque (`Infallible` is unused here); the domain error is the
    // useful return for the storage round-trip.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(value: &str) -> Result<Self, JobError> {
        match value {
            "queued" => Ok(Self::Queued),
            "running" => Ok(Self::Running),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            other => Err(JobError::Storage(format!(
                "estado de job desconhecido: {other}"
            ))),
        }
    }
}

/// A persisted job row.
///
/// This is a plain persistence record: `payload` is opaque text (the JSON
/// contract arrives in ticket 08) and the timestamps are RFC 3339 UTC strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobRecord {
    /// Stable identifier of the job.
    pub id: String,
    /// Registered kind that selects the handler.
    pub kind: String,
    /// Opaque work description; never interpreted by this layer.
    pub payload: String,
    /// Current state.
    pub state: JobState,
    /// Whether the job can safely resume after an interruption.
    pub idempotent: bool,
    /// Number of times the job was claimed; incremented on every claim.
    pub attempts: i64,
    /// Diagnostic from the last failure, if any.
    pub last_error: Option<String>,
    /// RFC 3339 timestamp when the job was enqueued.
    pub created_at: String,
    /// RFC 3339 timestamp of the last state change.
    pub updated_at: String,
}

/// Typed reason a handler reports a job as failed.
///
/// Variants carry **no runtime string**: every message is a fixed product text,
/// so sensitive content (payload, prompts, diffs, credentials) cannot reach
/// `last_error` even if a handler tries. Start with the one honest generic
/// reason; the tickets that need a finer taxonomy (09/11) extend this enum with
/// documented variants and fixed messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobFailure {
    /// The handler could not complete the job for a reason it owns.
    Failed,
}

impl JobFailure {
    /// Returns the fixed product message persisted in `last_error`.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Failed => "o job falhou durante a execução",
        }
    }
}

impl std::fmt::Display for JobFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Failure modes of the Jobs use cases.
///
/// These are framework and storage errors only; a handler reports through
/// [`JobFailure`], which cannot carry a free-form message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobError {
    /// The storage backend failed; the message is diagnostic only.
    Storage(String),
    /// No job with the requested identifier exists.
    NotFound,
    /// The requested state change is not allowed from the current state.
    InvalidTransition {
        /// State the transition started from.
        from: JobState,
        /// State the transition tried to reach.
        to: JobState,
    },
    /// The job kind has no registered handler, so it cannot be enqueued.
    UnknownKind(String),
}

impl std::fmt::Display for JobError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Storage(message) => write!(formatter, "falha de armazenamento: {message}"),
            Self::NotFound => formatter.write_str("job não encontrado"),
            Self::InvalidTransition { from, to } => write!(
                formatter,
                "transição de estado inválida: {} -> {}",
                from.as_str(),
                to.as_str()
            ),
            Self::UnknownKind(kind) => write!(formatter, "tipo de job desconhecido: {kind}"),
        }
    }
}

impl std::error::Error for JobError {}

/// Diagnostic written to `last_error` when a running job cannot resume.
pub const INTERRUPTED_NON_IDEMPOTENT: &str = "O job foi interrompido por um reinício do aplicativo e não pode ser retomado automaticamente porque não é idempotente.";

/// Job kind scheduled by ingest to analyze a persisted capture.
///
/// No handler is registered for this kind until ticket 12, so the job stays
/// `queued` by design: `claim_next` never drops or silently fails a kind without
/// a registered handler.
pub const ANALYZE_CAPTURE_KIND: &str = "analyze_capture";

/// Fixed diagnostic for a handler panic. The panic content is never used.
const PANIC_MESSAGE: &str = "o job falhou com pânico";

/// Result of recovering jobs interrupted by a restart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RecoveryReport {
    /// Number of interrupted idempotent jobs returned to `queued`.
    pub requeued: usize,
    /// Number of interrupted non-idempotent jobs marked `failed`.
    pub failed: usize,
}

/// Terminal state reached by the last job a worker ran.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobOutcome {
    /// Identifier of the job.
    pub job_id: String,
    /// Kind of the job.
    pub kind: String,
    /// State the job reached.
    pub state: JobState,
    /// Attempts recorded when the job was claimed.
    pub attempts: i64,
}

/// Event emitted by the worker for the composition root to log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobEvent {
    /// A job reached a terminal state.
    Finished(JobOutcome),
    /// The worker stopped because storage failed; the message is diagnostic.
    StorageError(String),
}

/// Callback a handler returns work through.
type JobHandler = Arc<dyn Fn(&JobRecord) -> Result<(), JobFailure> + Send + Sync + 'static>;

/// Callback invoked by the worker for every event, so the composition root can
/// log without this crate depending on `tracing`.
type Observer = Arc<dyn Fn(JobEvent) + Send + Sync + 'static>;

/// Port that persists jobs.
///
/// Infrastructure crates implement this trait; the application never knows how
/// rows are stored.
pub trait JobRepository {
    /// Inserts a new record.
    fn insert(&self, record: &JobRecord) -> Result<(), JobError>;

    /// Returns the record with the given identifier, if any.
    fn get(&self, id: &str) -> Result<Option<JobRecord>, JobError>;

    /// Returns every record, most recently created first.
    fn list(&self) -> Result<Vec<JobRecord>, JobError>;

    /// Atomically claims the oldest queued job whose kind is in `registered_kinds`.
    ///
    /// Claims `queued -> running` and increments `attempts`. Jobs whose kind is
    /// not registered stay `queued`: they never disappear and are never failed
    /// silently.
    fn claim_next(&self, registered_kinds: &[String]) -> Result<Option<JobRecord>, JobError>;

    /// Compare-and-set transition: changes state only when the row is in `from`.
    ///
    /// Returns `false` when no row matched, which is how an invalid transition
    /// (for example cancelling a `running` job) is detected.
    fn transition(
        &self,
        id: &str,
        from: JobState,
        to: JobState,
        last_error: Option<&str>,
    ) -> Result<bool, JobError>;

    /// Requeues interrupted idempotent jobs and fails interrupted
    /// non-idempotent ones.
    fn recover_interrupted(&self) -> Result<RecoveryReport, JobError>;
}

/// Jobs use cases over a [`JobRepository`].
///
/// Handlers are registered before the instance is shared and are frozen
/// afterwards, so every clone of a shared `Jobs` sees the same handlers.
#[derive(Clone)]
pub struct Jobs<R> {
    repository: R,
    handlers: HashMap<String, JobHandler>,
    kinds: Vec<String>,
    observer: Observer,
    signal: Arc<Signal>,
    stop: Arc<AtomicBool>,
}

/// Wakeup primitive shared between `enqueue` and the worker thread.
struct Signal {
    generation: Mutex<u64>,
    condvar: Condvar,
}

impl<R> Jobs<R> {
    /// Wraps a repository with the Jobs use cases and no registered handlers.
    pub fn new(repository: R) -> Self {
        Self {
            repository,
            handlers: HashMap::new(),
            kinds: Vec::new(),
            observer: Arc::new(|_event: JobEvent| {}),
            signal: Arc::new(Signal {
                generation: Mutex::new(0),
                condvar: Condvar::new(),
            }),
            stop: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Registers the handler for a job kind.
    ///
    /// Must be called before the instance is shared; registering after a clone
    /// exists only affects the caller's copy and is therefore not the contract.
    pub fn register(&mut self, kind: impl Into<String>, handler: JobHandler) {
        let kind = kind.into();
        if !self.handlers.contains_key(&kind) {
            self.kinds.push(kind.clone());
        }
        self.handlers.insert(kind, handler);
    }

    /// Installs the callback the worker uses to report events.
    ///
    /// This is how the composition root logs outcomes while this crate stays
    /// free of `tracing` (spec §14).
    pub fn observe_with<F>(&mut self, observer: F)
    where
        F: Fn(JobEvent) + Send + Sync + 'static,
    {
        self.observer = Arc::new(observer);
    }

    /// Wakes the worker after a new job is persisted or on shutdown.
    fn wake(&self) {
        let mut generation = self
            .signal
            .generation
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *generation = generation.wrapping_add(1);
        self.signal.condvar.notify_all();
    }
}

impl<R: JobRepository> Jobs<R> {
    /// Persists a job and wakes the worker.
    ///
    /// The job is written before any execution (spec §9). The kind must have a
    /// registered handler, otherwise nothing could ever run it.
    pub fn enqueue(
        &self,
        kind: &str,
        payload: &str,
        idempotent: bool,
    ) -> Result<JobRecord, JobError> {
        if !self.handlers.contains_key(kind) {
            return Err(JobError::UnknownKind(kind.to_string()));
        }
        let timestamp = now_rfc3339();
        let record = JobRecord {
            id: uuid::Uuid::now_v7().to_string(),
            kind: kind.to_string(),
            payload: payload.to_string(),
            state: JobState::Queued,
            idempotent,
            attempts: 0,
            last_error: None,
            created_at: timestamp.clone(),
            updated_at: timestamp,
        };
        self.repository.insert(&record)?;
        self.wake();
        Ok(record)
    }

    /// Lists tracked jobs, most recently created first.
    pub fn list(&self) -> Result<Vec<JobRecord>, JobError> {
        self.repository.list()
    }

    /// Cancels a queued job.
    ///
    /// Only `queued -> cancelled` is allowed; a `running` or terminal job
    /// cannot be cancelled and returns [`JobError::InvalidTransition`].
    pub fn cancel(&self, id: &str) -> Result<(), JobError> {
        let record = self.repository.get(id)?.ok_or(JobError::NotFound)?;
        if record.state != JobState::Queued {
            return Err(JobError::InvalidTransition {
                from: record.state,
                to: JobState::Cancelled,
            });
        }
        let changed =
            self.repository
                .transition(id, JobState::Queued, JobState::Cancelled, None)?;
        if !changed {
            return Err(JobError::InvalidTransition {
                from: JobState::Queued,
                to: JobState::Cancelled,
            });
        }
        Ok(())
    }

    /// Recovers jobs left `running` by a restart.
    ///
    /// Idempotent jobs return to `queued`; non-idempotent ones become `failed`
    /// with a visible diagnostic. Call this once at boot, before the worker.
    pub fn recover(&self) -> Result<RecoveryReport, JobError> {
        self.repository.recover_interrupted()
    }

    /// Claims and runs the next eligible job.
    ///
    /// Returns `Ok(None)` when nothing is eligible, `Ok(Some(outcome))` after a
    /// handler completed, errored or panicked, and `Err` only on storage
    /// failure. A handler panic is contained here, so the caller survives.
    pub fn run_next(&self) -> Result<Option<JobOutcome>, JobError> {
        let Some(record) = self.repository.claim_next(&self.kinds)? else {
            return Ok(None);
        };
        let Some(handler) = self.handlers.get(&record.kind) else {
            // The claim filter makes this unreachable, but a job must never be
            // lost if the two views disagree.
            self.transition_checked(&record.id, JobState::Running, JobState::Queued, None)?;
            return Ok(Some(JobOutcome {
                job_id: record.id,
                kind: record.kind,
                state: JobState::Queued,
                attempts: record.attempts,
            }));
        };

        // Only the handler call may panic with sensitive content; the flag makes
        // the installed panic hook omit that payload. `catch_unwind` guarantees
        // the flag is restored even when the handler panics.
        let previous = replace_job_panic_sanitized(true);
        let result = catch_unwind(AssertUnwindSafe(|| handler(&record)));
        replace_job_panic_sanitized(previous);

        let state = match result {
            Ok(Ok(())) => {
                self.transition_checked(&record.id, JobState::Running, JobState::Completed, None)?;
                JobState::Completed
            }
            Ok(Err(failure)) => {
                // A typed failure carries a fixed product message; no runtime
                // text reaches `last_error` (PRIV-001).
                self.transition_checked(
                    &record.id,
                    JobState::Running,
                    JobState::Failed,
                    Some(failure.as_str()),
                )?;
                JobState::Failed
            }
            Err(_panic) => {
                // A fixed message: the panic content and payload are never used.
                self.transition_checked(
                    &record.id,
                    JobState::Running,
                    JobState::Failed,
                    Some(PANIC_MESSAGE),
                )?;
                JobState::Failed
            }
        };

        Ok(Some(JobOutcome {
            job_id: record.id,
            kind: record.kind,
            state,
            attempts: record.attempts,
        }))
    }

    /// Applies a compare-and-set transition, treating a miss as a conflict.
    ///
    /// A `false` result means another writer changed the row, so the caller must
    /// not report a terminal outcome the database did not record.
    fn transition_checked(
        &self,
        id: &str,
        from: JobState,
        to: JobState,
        last_error: Option<&str>,
    ) -> Result<(), JobError> {
        if self.repository.transition(id, from, to, last_error)? {
            Ok(())
        } else {
            Err(JobError::InvalidTransition { from, to })
        }
    }
}

impl<R> Jobs<R>
where
    R: JobRepository + Clone + Send + 'static,
{
    /// Starts the dedicated worker thread.
    ///
    /// The thread waits on a condition variable with a short poll timeout,
    /// drains every eligible job, and stops when [`WorkerHandle::stop`] is
    /// called. It never touches the UI (ASYNC-001).
    pub fn spawn_worker(&self) -> WorkerHandle {
        let stop = self.stop.clone();
        let signal = self.signal.clone();
        let jobs = self.clone();
        let failure: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let failure_thread = failure.clone();

        let thread = match std::thread::Builder::new()
            .name("xemnas-jobs".to_string())
            .spawn(move || worker_loop(jobs, stop, signal, failure_thread))
        {
            Ok(handle) => Some(handle),
            Err(error) => {
                *failure
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(error.to_string());
                None
            }
        };

        WorkerHandle {
            signal: self.signal.clone(),
            stop: self.stop.clone(),
            thread,
            failure,
        }
    }
}

/// Poll interval for the worker when no wakeup arrives.
const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Body of the worker thread.
fn worker_loop<R>(
    jobs: Jobs<R>,
    stop: Arc<AtomicBool>,
    signal: Arc<Signal>,
    failure: Arc<Mutex<Option<String>>>,
) where
    R: JobRepository,
{
    loop {
        if stop.load(Ordering::SeqCst) {
            return;
        }
        match jobs.run_next() {
            Ok(Some(outcome)) => {
                (jobs.observer)(JobEvent::Finished(outcome));
                continue;
            }
            Ok(None) => {}
            Err(error) => {
                let message = error.to_string();
                *failure
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(message.clone());
                (jobs.observer)(JobEvent::StorageError(message));
                return;
            }
        }

        // Wait for a wakeup or the poll timeout; holding the generation lock
        // across the wait closes the lost-wakeup window with `wake`.
        let generation = signal
            .generation
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if stop.load(Ordering::SeqCst) {
            return;
        }
        let _waited = signal
            .condvar
            .wait_timeout(generation, POLL_INTERVAL)
            .unwrap_or_else(|poisoned| poisoned.into_inner());
    }
}

/// Handle to the worker thread.
///
/// Stops and joins the thread. Dropping the handle without joining is allowed
/// but leaves the thread running until the process ends; the composition root
/// calls [`WorkerHandle::stop`] and [`WorkerHandle::join`] on shutdown.
pub struct WorkerHandle {
    signal: Arc<Signal>,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
    failure: Arc<Mutex<Option<String>>>,
}

impl WorkerHandle {
    /// Signals the worker to stop and wakes it immediately.
    pub fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
        let mut generation = self
            .signal
            .generation
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *generation = generation.wrapping_add(1);
        self.signal.condvar.notify_all();
    }

    /// Waits for the worker thread to finish.
    ///
    /// Returns an error if the thread could not be started, terminated
    /// unexpectedly, or stopped because of a storage failure.
    pub fn join(mut self) -> Result<(), JobError> {
        if let Some(thread) = self.thread.take() {
            thread.join().map_err(|_| {
                JobError::Storage("a thread de jobs terminou inesperadamente".to_string())
            })?;
        }
        let failure = self
            .failure
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
        match failure {
            Some(message) => Err(JobError::Storage(message)),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        job_panic_is_sanitized, write_sanitized_panic_report, JobError, JobEvent, JobFailure,
        JobRecord, JobRepository, JobState, Jobs, RecoveryReport, PANIC_MESSAGE,
    };
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};

    /// In-memory repository with controllable claim and transition outcomes.
    #[derive(Clone, Default)]
    struct FakeRepository {
        claim: Arc<Mutex<Vec<JobRecord>>>,
        transition_ok: Arc<AtomicBool>,
        last_errors: Arc<Mutex<Vec<Option<String>>>>,
    }

    impl FakeRepository {
        fn with_claim(record: JobRecord) -> Self {
            let repository = Self {
                transition_ok: Arc::new(AtomicBool::new(true)),
                ..Self::default()
            };
            repository
                .claim
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(record);
            repository
        }

        fn conflicting_transition(record: JobRecord) -> Self {
            let repository = Self {
                transition_ok: Arc::new(AtomicBool::new(false)),
                ..Self::default()
            };
            repository
                .claim
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(record);
            repository
        }

        fn last_errors(&self) -> Vec<Option<String>> {
            self.last_errors
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone()
        }
    }

    impl JobRepository for FakeRepository {
        fn insert(&self, _record: &JobRecord) -> Result<(), JobError> {
            Ok(())
        }

        fn get(&self, _id: &str) -> Result<Option<JobRecord>, JobError> {
            Ok(None)
        }

        fn list(&self) -> Result<Vec<JobRecord>, JobError> {
            Ok(Vec::new())
        }

        fn claim_next(&self, _registered_kinds: &[String]) -> Result<Option<JobRecord>, JobError> {
            Ok(self
                .claim
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .pop())
        }

        fn transition(
            &self,
            _id: &str,
            _from: JobState,
            _to: JobState,
            last_error: Option<&str>,
        ) -> Result<bool, JobError> {
            self.last_errors
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(last_error.map(str::to_string));
            Ok(self.transition_ok.load(Ordering::SeqCst))
        }

        fn recover_interrupted(&self) -> Result<RecoveryReport, JobError> {
            Ok(RecoveryReport::default())
        }
    }

    fn job_record(id: &str, kind: &str) -> JobRecord {
        JobRecord {
            id: id.to_string(),
            kind: kind.to_string(),
            payload: "{}".to_string(),
            state: JobState::Queued,
            idempotent: true,
            attempts: 0,
            last_error: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn state_literals_round_trip() {
        for state in [
            JobState::Queued,
            JobState::Running,
            JobState::Completed,
            JobState::Failed,
            JobState::Cancelled,
        ] {
            assert_eq!(JobState::from_str(state.as_str()), Ok(state));
        }
    }

    #[test]
    fn unknown_state_literal_is_a_storage_error() {
        assert!(matches!(
            JobState::from_str("paused"),
            Err(JobError::Storage(_))
        ));
    }

    #[test]
    fn invalid_transition_display_uses_state_literals() {
        let error = JobError::InvalidTransition {
            from: JobState::Running,
            to: JobState::Cancelled,
        };
        let rendered = error.to_string();
        assert!(rendered.contains("running"));
        assert!(rendered.contains("cancelled"));
    }

    #[test]
    fn panic_flag_is_set_only_during_the_handler_call() {
        let observed = Arc::new(AtomicBool::new(false));
        let observed_handler = observed.clone();
        let mut jobs = Jobs::new(FakeRepository::with_claim(job_record(
            "job-panic",
            "analysis",
        )));
        jobs.register(
            "analysis",
            Arc::new(move |_: &JobRecord| -> Result<(), JobFailure> {
                observed_handler.store(job_panic_is_sanitized(), Ordering::SeqCst);
                panic!("secret-42-marker");
            }),
        );

        let outcome = jobs
            .run_next()
            .expect("the panic must be contained")
            .expect("the job ran");
        assert_eq!(outcome.state, JobState::Failed);
        assert!(
            observed.load(Ordering::SeqCst),
            "the sanitizer flag must be set while the handler runs"
        );
        assert!(
            !job_panic_is_sanitized(),
            "the sanitizer flag must be restored after the handler"
        );
    }

    #[test]
    fn sanitized_panic_report_cannot_carry_the_payload() {
        let marker = "secret-42-marker";
        let mut buffer = Vec::new();
        write_sanitized_panic_report(&mut buffer, Some("xemnas-jobs"), Some("src/jobs.rs:123"))
            .expect("render the report");
        let rendered = String::from_utf8(buffer).expect("report is UTF-8");

        assert!(
            !rendered.contains(marker),
            "the report renderer has no payload channel: {rendered}"
        );
        assert!(rendered.contains("xemnas-jobs"), "thread name missing");
        assert!(rendered.contains("src/jobs.rs:123"), "location missing");
        assert!(rendered.contains("PRIV-001"), "reason missing");
    }

    #[test]
    fn installed_hook_sanitizes_a_handler_panic() {
        // stderr cannot be captured in-process without another crate, so this
        // test installs a hook that re-renders through the exact function
        // `install_panic_sanitizer` uses and captures it in memory.
        let previous = std::panic::take_hook();
        let buffer = Arc::new(Mutex::new(Vec::<u8>::new()));
        let buffer_hook = buffer.clone();
        let sanitized_seen = Arc::new(AtomicBool::new(false));
        let sanitized_seen_hook = sanitized_seen.clone();
        std::panic::set_hook(Box::new(move |info: &std::panic::PanicHookInfo<'_>| {
            if job_panic_is_sanitized() {
                sanitized_seen_hook.store(true, Ordering::SeqCst);
                let location = info.location().map(|location| location.to_string());
                let thread_name = std::thread::current().name().map(str::to_string);
                let mut guard = buffer_hook
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                let _ = write_sanitized_panic_report(
                    &mut *guard,
                    thread_name.as_deref(),
                    location.as_deref(),
                );
            } else {
                previous(info);
            }
        }));

        let mut jobs = Jobs::new(FakeRepository::with_claim(job_record(
            "job-hook", "analysis",
        )));
        jobs.register(
            "analysis",
            Arc::new(|_: &JobRecord| -> Result<(), JobFailure> {
                panic!("secret-42-marker");
            }),
        );
        let _ = jobs.run_next().expect("the panic must be contained");

        assert!(
            sanitized_seen.load(Ordering::SeqCst),
            "the hook must run while the sanitizer flag is set"
        );
        let rendered = String::from_utf8(
            buffer
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone(),
        )
        .expect("report is UTF-8");
        assert!(
            !rendered.contains("secret-42-marker"),
            "the hook output leaked the panic payload: {rendered}"
        );
    }

    #[test]
    fn terminal_transition_conflict_is_an_error_and_emits_no_finished_event() {
        let mut jobs = Jobs::new(FakeRepository::conflicting_transition(job_record(
            "job-conflict",
            "analysis",
        )));
        jobs.register(
            "analysis",
            Arc::new(|_: &JobRecord| -> Result<(), JobFailure> { Ok(()) }),
        );
        let events = Arc::new(Mutex::new(Vec::new()));
        let events_observer = events.clone();
        jobs.observe_with(move |event| {
            events_observer
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(event);
        });

        let result = jobs.run_next();
        assert_eq!(
            result,
            Err(JobError::InvalidTransition {
                from: JobState::Running,
                to: JobState::Completed,
            })
        );
        let recorded = events
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        assert!(
            recorded
                .iter()
                .all(|event| !matches!(event, JobEvent::Finished(_))),
            "a failed compare-and-set must not emit a Finished event"
        );
    }

    #[test]
    fn missing_handler_requeue_conflict_is_an_error() {
        let jobs = Jobs::new(FakeRepository::conflicting_transition(job_record(
            "job-orphan",
            "desconhecido",
        )));
        // No handler registered, so the defensive requeue branch runs.
        assert_eq!(
            jobs.run_next(),
            Err(JobError::InvalidTransition {
                from: JobState::Running,
                to: JobState::Queued,
            })
        );
    }

    #[test]
    fn handler_failure_persists_only_the_fixed_message() {
        let repository = FakeRepository::with_claim(job_record("job-failure", "analysis"));
        let probe = repository.clone();
        let mut jobs = Jobs::new(repository);
        jobs.register(
            "analysis",
            Arc::new(|_: &JobRecord| -> Result<(), JobFailure> { Err(JobFailure::Failed) }),
        );

        let outcome = jobs.run_next().expect("run").expect("the job ran");
        assert_eq!(outcome.state, JobState::Failed);
        assert_eq!(
            probe.last_errors(),
            vec![Some(JobFailure::Failed.as_str().to_string())],
            "last_error must be exactly the fixed product message"
        );
    }

    #[test]
    fn payload_marker_never_reaches_last_error_or_events() {
        let marker = "SECRET-PAYLOAD-MARKER";
        for panic_mode in [false, true] {
            let mut record = job_record("job-secret", "analysis");
            record.payload = marker.to_string();
            let repository = FakeRepository::with_claim(record);
            let probe = repository.clone();
            let mut jobs = Jobs::new(repository);
            if panic_mode {
                let marker_owned = marker.to_string();
                jobs.register(
                    "analysis",
                    Arc::new(move |_: &JobRecord| -> Result<(), JobFailure> {
                        panic!("{marker_owned}");
                    }),
                );
            } else {
                jobs.register(
                    "analysis",
                    Arc::new(|_: &JobRecord| -> Result<(), JobFailure> { Err(JobFailure::Failed) }),
                );
            }

            let events = Arc::new(Mutex::new(Vec::new()));
            let events_observer = events.clone();
            jobs.observe_with(move |event| {
                events_observer
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .push(event);
            });

            let outcome = jobs.run_next().expect("run").expect("the job ran");
            assert_eq!(outcome.state, JobState::Failed);

            let messages: Vec<String> = probe.last_errors().into_iter().flatten().collect();
            for message in &messages {
                assert!(
                    !message.contains(marker),
                    "last_error leaked the payload (panic_mode={panic_mode}): {message}"
                );
            }
            if panic_mode {
                assert_eq!(messages, vec![PANIC_MESSAGE.to_string()]);
            } else {
                assert_eq!(messages, vec![JobFailure::Failed.as_str().to_string()]);
            }

            let recorded = events
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            for event in recorded.iter() {
                let rendered = format!("{event:?}");
                assert!(
                    !rendered.contains(marker),
                    "event leaked the payload (panic_mode={panic_mode}): {rendered}"
                );
            }
        }
    }
}
