//! Jobs use cases: persist work, run it off the UI thread, recover it on boot.

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
    static JOB_PANIC_SANITIZED: Cell<bool> = const { Cell::new(false) };
    /// Lane of the job a handler runs on this thread, if any.
    static CURRENT_LANE: Cell<Option<Lane>> = const { Cell::new(None) };
}

/// Lane of the job running on this thread, so a shared resource (the
/// provider limiter) can favour the `now` lane.
pub fn current_lane() -> Option<Lane> {
    CURRENT_LANE.with(Cell::get)
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
    /// Cancelled before execution, or invalidated by project purge while running.
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

/// Typed reason a handler reports a job as not completed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobFailure {
    /// The handler could not complete the job for a reason it owns.
    Failed,
    /// The provider asked to slow down or did not answer in time: the job
    /// goes back to the queue after `retry_after` (or an exponential backoff
    /// with jitter when the provider gave no time).
    Deferred {
        /// Wait the provider asked for, if it said.
        retry_after: Option<Duration>,
    },
}

impl JobFailure {
    /// Returns the fixed product message persisted in `last_error`.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Failed => "o job falhou durante a execução",
            Self::Deferred { .. } => {
                "o provedor de IA pediu uma pausa; a tarefa voltou para a fila"
            }
        }
    }
}

/// Claims after which a job the provider keeps deferring is marked failed
/// (it can still be reprocessed by hand, so nothing is lost).
pub const MAX_DEFERRED_ATTEMPTS: i64 = 8;

/// Message of a job that gave up after [`MAX_DEFERRED_ATTEMPTS`].
pub const DEFERRED_TOO_OFTEN: &str =
    "o provedor de IA seguiu indisponível; reprocesse a tarefa quando ele voltar";

/// First wait of the job-level backoff.
const BACKOFF_BASE: Duration = Duration::from_secs(15);

/// Longest wait before a deferred job is claimable again.
pub const MAX_BACKOFF: Duration = Duration::from_secs(10 * 60);

/// Wait before a deferred job is claimable again: the provider's
/// `Retry-After` when it gave one, otherwise `15 s · 2^(attempts-1)` with
/// ±25 % jitter, both capped at [`MAX_BACKOFF`].
pub fn deferral_delay(attempts: i64, retry_after: Option<Duration>) -> Duration {
    if let Some(after) = retry_after {
        return after.clamp(Duration::from_secs(1), MAX_BACKOFF);
    }
    let exponent = u32::try_from(attempts.saturating_sub(1).clamp(0, 16)).unwrap_or(16);
    let base = BACKOFF_BASE
        .saturating_mul(1u32 << exponent)
        .min(MAX_BACKOFF);
    // Jitter in [0.75, 1.25) from the std random hasher; no new dependency.
    let noise = {
        use std::hash::{BuildHasher, Hasher};
        let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
        hasher.write_i64(attempts);
        hasher.finish() % 500
    };
    let factor = 0.75 + noise as f64 / 1000.0;
    base.mul_f64(factor).min(MAX_BACKOFF)
}

impl std::fmt::Display for JobFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Failure modes of the Jobs use cases.
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
    /// A job setting outside its allowed range.
    InvalidSetting,
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
            Self::InvalidSetting => formatter.write_str("análises em paralelo vão de 1 a 4"),
        }
    }
}

impl std::error::Error for JobError {}

/// Diagnostic written to `last_error` when a running job cannot resume.
pub const INTERRUPTED_NON_IDEMPOTENT: &str = "O job foi interrompido por um reinício do aplicativo e não pode ser retomado automaticamente porque não é idempotente.";

/// Job kind scheduled by ingest to analyze a persisted capture.
pub const ANALYZE_CAPTURE_KIND: &str = JobKind::AnalyzeCapture.as_str();

/// Job kind scheduled by a documentation import to analyze one document.
/// Same handler as [`ANALYZE_CAPTURE_KIND`]; a kind of its own so a large
/// import waits in its own lane.
pub const ANALYZE_DOCUMENT_KIND: &str = JobKind::AnalyzeDocument.as_str();

/// A group of job kinds served by its own workers, so a long documentation
/// import never delays the analysis of the session that just ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Lane {
    /// Analysis of captures from agent sessions: what the person waits for.
    Now,
    /// Analysis of imported documentation.
    Documents,
    /// Relation and rule suggestions after an adoption.
    Suggestions,
}

impl Lane {
    /// Every lane, in priority order.
    pub const ALL: [Self; 3] = [Self::Now, Self::Documents, Self::Suggestions];

    /// Stable literal for logs and diagnostics.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Now => "now",
            Self::Documents => "documents",
            Self::Suggestions => "suggestions",
        }
    }

    /// Workers of this lane for a provider limit of `parallel` calls: the two
    /// analysis lanes get up to two each, suggestions always one.
    pub fn workers(&self, parallel: usize) -> usize {
        match self {
            Self::Now | Self::Documents => parallel.clamp(1, 2),
            Self::Suggestions => 1,
        }
    }
}

/// The product's job kinds. The match in [`JobKind::lane`] is exhaustive, so
/// a new kind cannot compile without a lane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobKind {
    /// Analysis of a capture from an agent session.
    AnalyzeCapture,
    /// Analysis of an imported document.
    AnalyzeDocument,
    /// Relation suggestions for an adopted decision.
    SuggestRelations,
    /// Rule suggestions derived from an adopted decision.
    DeriveClaims,
    /// Local, provider-free refresh of descriptive observations.
    RefreshObservations,
    /// Optional AI relevance judgement for ambiguous context.
    ContextRouting,
    /// Search terms for a project's adopted decisions.
    DeriveSearchTerms,
    /// Map components a decision applies to, proposed by the AI.
    SuggestLinks,
    /// The automatic judge, over everything that waits for a person.
    AutoReview,
}

impl JobKind {
    /// Every product kind.
    pub const ALL: [Self; 9] = [
        Self::AnalyzeCapture,
        Self::AnalyzeDocument,
        Self::SuggestRelations,
        Self::DeriveClaims,
        Self::RefreshObservations,
        Self::ContextRouting,
        Self::DeriveSearchTerms,
        Self::SuggestLinks,
        Self::AutoReview,
    ];

    /// Literal persisted in the `kind` column.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::AnalyzeCapture => "analyze_capture",
            Self::AnalyzeDocument => "analyze_document",
            Self::SuggestRelations => "suggest_relations",
            Self::DeriveClaims => "derive_claims",
            Self::RefreshObservations => crate::observations::refresh::REFRESH_OBSERVATIONS_KIND,
            Self::ContextRouting => crate::context_routing::CONTEXT_ROUTING_KIND,
            Self::DeriveSearchTerms => "derive_search_terms",
            Self::SuggestLinks => "suggest_links",
            Self::AutoReview => "auto_review",
        }
    }

    /// Parses a persisted kind literal.
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == value)
    }

    /// Claim order inside a lane, lowest first: what the agent's hot path
    /// needs (the context judgement, then the map components a decision
    /// governs) goes before the relations, rules and search terms, which only
    /// refine what is already there. Equal priorities run oldest first.
    pub const fn priority(&self) -> u8 {
        match self {
            Self::AnalyzeCapture | Self::AnalyzeDocument | Self::ContextRouting => 0,
            Self::SuggestLinks | Self::RefreshObservations => 1,
            Self::SuggestRelations => 2,
            Self::DeriveClaims => 3,
            Self::DeriveSearchTerms => 4,
            // After the links, relations, rules and terms emptied: one pass
            // then judges all of them.
            Self::AutoReview => 5,
        }
    }

    /// The lane whose workers run this kind.
    pub fn lane(&self) -> Lane {
        match self {
            Self::AnalyzeCapture => Lane::Now,
            Self::AnalyzeDocument => Lane::Documents,
            // Local and cheap: it keeps descriptive memory fresh for the
            // capture that just arrived.
            Self::RefreshObservations => Lane::Now,
            Self::SuggestRelations
            | Self::DeriveClaims
            | Self::ContextRouting
            | Self::DeriveSearchTerms
            | Self::SuggestLinks
            // One worker in the lane: two judges never run together.
            | Self::AutoReview => Lane::Suggestions,
        }
    }
}

/// Lane of a persisted kind. Kinds outside [`JobKind`] exist only in tests
/// and run in [`Lane::Now`].
pub fn lane_of(kind: &str) -> Lane {
    JobKind::parse(kind).map_or(Lane::Now, |kind| kind.lane())
}

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

/// Callback that runs several jobs of one kind with a single provider call.
/// It answers one result per record, in order; a missing result fails that
/// job.
type BatchHandler =
    Arc<dyn Fn(&[JobRecord]) -> Vec<Result<(), JobFailure>> + Send + Sync + 'static>;

/// A kind whose queued jobs are coalesced: up to `max` run together.
#[derive(Clone)]
struct Batch {
    max: usize,
    handler: BatchHandler,
}

/// Priority of a persisted kind inside its lane; kinds outside [`JobKind`]
/// (tests) keep the highest.
fn priority_of(kind: &str) -> u8 {
    JobKind::parse(kind).map_or(0, |kind| kind.priority())
}

/// Callback invoked by the worker for every event, so the composition root can
/// log without this crate depending on `tracing`.
type Observer = Arc<dyn Fn(JobEvent) + Send + Sync + 'static>;

/// Port that persists jobs.
pub trait JobRepository {
    /// Inserts a new record.
    fn insert(&self, record: &JobRecord) -> Result<(), JobError>;

    /// Returns the record with the given identifier, if any.
    fn get(&self, id: &str) -> Result<Option<JobRecord>, JobError>;

    /// Returns every record, most recently created first.
    fn list(&self) -> Result<Vec<JobRecord>, JobError>;

    /// Number of jobs per kind and state. The default reads every row; a
    /// store should answer it with one grouped query.
    fn counts(&self) -> Result<Vec<(String, JobState, usize)>, JobError> {
        let mut counts: Vec<(String, JobState, usize)> = Vec::new();
        for record in self.list()? {
            match counts
                .iter_mut()
                .find(|(kind, state, _)| *kind == record.kind && *state == record.state)
            {
                Some((_, _, count)) => *count += 1,
                None => counts.push((record.kind, record.state, 1)),
            }
        }
        Ok(counts)
    }

    /// Atomically claims the next queued job whose kind is in
    /// `registered_kinds`: the kind that comes first in the slice (the slice
    /// is in priority order), then the oldest job of that kind.
    fn claim_next(&self, registered_kinds: &[String]) -> Result<Option<JobRecord>, JobError>;

    /// Atomically claims up to `limit` more queued jobs of `kind`, oldest
    /// first, so a batch handler can answer them with one provider call. The
    /// default claims none, which makes every batch a batch of one.
    fn claim_more(&self, kind: &str, limit: usize) -> Result<Vec<JobRecord>, JobError> {
        let _ = (kind, limit);
        Ok(Vec::new())
    }

    /// Compare-and-set transition: changes state only when the row is in `from`.
    fn transition(
        &self,
        id: &str,
        from: JobState,
        to: JobState,
        last_error: Option<&str>,
    ) -> Result<bool, JobError>;

    /// Returns a `running` job to `queued`, claimable only after `delay`.
    /// The default ignores the delay; a store should persist it.
    fn defer(&self, id: &str, delay: Duration, last_error: &str) -> Result<bool, JobError> {
        let _ = delay;
        self.transition(id, JobState::Running, JobState::Queued, Some(last_error))
    }

    /// Requeues interrupted idempotent jobs and fails interrupted
    /// non-idempotent ones.
    fn recover_interrupted(&self) -> Result<RecoveryReport, JobError>;

    /// Confirms that a claimed job disappeared with its owning project during purge.
    /// Called only after a terminal compare-and-set miss, never after a storage error.
    fn cancelled_by_project_purge(&self, _record: &JobRecord) -> Result<bool, JobError> {
        Ok(false)
    }
}

/// Port that persists the global job settings.
pub trait JobSettingsStore {
    /// Provider calls at once chosen by the person, if any.
    fn parallel_analyses(&self) -> Result<Option<u8>, JobError>;

    /// Persists the provider calls at once.
    fn set_parallel_analyses(&self, value: u8) -> Result<(), JobError>;
}

/// Jobs use cases over a [`JobRepository`].
#[derive(Clone)]
pub struct Jobs<R> {
    repository: R,
    handlers: HashMap<String, JobHandler>,
    batches: HashMap<String, Batch>,
    kinds: Vec<String>,
    lanes: HashMap<Lane, Vec<String>>,
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
            batches: HashMap::new(),
            kinds: Vec::new(),
            lanes: HashMap::new(),
            observer: Arc::new(|_event: JobEvent| {}),
            signal: Arc::new(Signal {
                generation: Mutex::new(0),
                condvar: Condvar::new(),
            }),
            stop: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Registers the handler for a job kind, in the lane [`lane_of`] gives it.
    pub fn register(&mut self, kind: impl Into<String>, handler: JobHandler) {
        let kind = kind.into();
        self.add_kind(&kind);
        self.batches.remove(&kind);
        self.handlers.insert(kind, handler);
    }

    /// Registers a handler that runs up to `max` queued jobs of `kind` with
    /// one call: the claimed job plus the oldest others of the same kind.
    pub fn register_batch(&mut self, kind: impl Into<String>, max: usize, handler: BatchHandler) {
        let kind = kind.into();
        self.add_kind(&kind);
        self.handlers.remove(&kind);
        self.batches.insert(
            kind,
            Batch {
                max: max.max(1),
                handler,
            },
        );
    }

    /// Adds the kind to the claim lists, which stay in priority order (the
    /// sort is stable, so equal priorities keep their registration order).
    fn add_kind(&mut self, kind: &str) {
        if self.kinds.iter().any(|known| known == kind) {
            return;
        }
        self.kinds.push(kind.to_string());
        self.kinds.sort_by_key(|kind| priority_of(kind));
        let lane = self.lanes.entry(lane_of(kind)).or_default();
        lane.push(kind.to_string());
        lane.sort_by_key(|kind| priority_of(kind));
    }

    /// Registered kinds served by `lane`.
    pub fn kinds_in(&self, lane: Lane) -> &[String] {
        self.lanes.get(&lane).map_or(&[], Vec::as_slice)
    }

    /// Installs the callback the worker uses to report events.
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

    /// Requeues a failed job so a worker can claim it again.
    pub fn reprocess(&self, id: &str) -> Result<(), JobError> {
        let record = self.repository.get(id)?.ok_or(JobError::NotFound)?;
        if record.state != JobState::Failed {
            return Err(JobError::InvalidTransition {
                from: record.state,
                to: JobState::Queued,
            });
        }
        let changed = self
            .repository
            .transition(id, JobState::Failed, JobState::Queued, None)?;
        if !changed {
            return Err(JobError::InvalidTransition {
                from: JobState::Failed,
                to: JobState::Queued,
            });
        }
        self.wake();
        Ok(())
    }

    /// Recovers jobs left `running` by a restart.
    pub fn recover(&self) -> Result<RecoveryReport, JobError> {
        self.repository.recover_interrupted()
    }

    /// Claims and runs the next eligible job of any lane, the one of the
    /// highest priority kind first. When it ran in a batch this is the outcome
    /// of the claimed job only; [`Jobs::run_next_group`] has them all.
    pub fn run_next(&self) -> Result<Option<JobOutcome>, JobError> {
        Ok(self.run_next_of(&self.kinds)?.into_iter().next())
    }

    /// Claims and runs the next eligible job of one lane.
    pub fn run_next_in(&self, lane: Lane) -> Result<Option<JobOutcome>, JobError> {
        Ok(self.run_next_of(self.kinds_in(lane))?.into_iter().next())
    }

    /// Like [`Jobs::run_next`], with the outcome of every job of the batch
    /// (empty when nothing was eligible).
    pub fn run_next_group(&self) -> Result<Vec<JobOutcome>, JobError> {
        self.run_next_of(&self.kinds)
    }

    /// Like [`Jobs::run_next_in`], with the outcome of every job of the batch.
    pub fn run_next_group_in(&self, lane: Lane) -> Result<Vec<JobOutcome>, JobError> {
        self.run_next_of(self.kinds_in(lane))
    }

    /// Queued, running and failed jobs per lane, every lane present.
    pub fn lane_summaries(&self) -> Result<Vec<(Lane, JobSummary)>, JobError> {
        let mut summaries: Vec<(Lane, JobSummary)> = Lane::ALL
            .into_iter()
            .map(|lane| (lane, JobSummary::default()))
            .collect();
        for (kind, state, count) in self.repository.counts()? {
            let lane = lane_of(&kind);
            if let Some((_, summary)) = summaries.iter_mut().find(|(each, _)| *each == lane) {
                summary.add(state, count);
            }
        }
        Ok(summaries)
    }

    fn run_next_of(&self, kinds: &[String]) -> Result<Vec<JobOutcome>, JobError> {
        let Some(record) = self.repository.claim_next(kinds)? else {
            return Ok(Vec::new());
        };
        if let Some(batch) = self.batches.get(&record.kind) {
            return self.run_batch(record, batch);
        }
        let Some(handler) = self.handlers.get(&record.kind) else {
            let state = if self.repository.transition(
                &record.id,
                JobState::Running,
                JobState::Queued,
                None,
            )? {
                JobState::Queued
            } else if self.repository.cancelled_by_project_purge(&record)? {
                JobState::Cancelled
            } else {
                return Err(JobError::InvalidTransition {
                    from: JobState::Running,
                    to: JobState::Queued,
                });
            };
            return Ok(vec![JobOutcome {
                job_id: record.id,
                kind: record.kind,
                state,
                attempts: record.attempts,
            }]);
        };

        let result = guarded(&record.kind, || handler(&record));
        Ok(vec![self.settle(record, result)?])
    }

    /// Runs a batch: the claimed job plus the oldest queued ones of its kind
    /// share one handler call, and each job then reaches its own state.
    fn run_batch(&self, first: JobRecord, batch: &Batch) -> Result<Vec<JobOutcome>, JobError> {
        let mut records = vec![first];
        records.extend(
            self.repository
                .claim_more(&records[0].kind, batch.max.saturating_sub(1))?,
        );
        let results = guarded(&records[0].kind, || (batch.handler)(&records));

        // Every claimed job is settled even if one settlement fails, so none
        // stays `running` until the next restart.
        let mut outcomes = Vec::with_capacity(records.len());
        let mut failure = None;
        for (index, record) in records.into_iter().enumerate() {
            let result = results.as_ref().map(|results| {
                results
                    .get(index)
                    .copied()
                    .unwrap_or(Err(JobFailure::Failed))
            });
            match self.settle(record, result) {
                Ok(outcome) => outcomes.push(outcome),
                Err(error) => failure = failure.or(Some(error)),
            }
        }
        match failure {
            Some(error) => Err(error),
            None => Ok(outcomes),
        }
    }

    /// Moves a job the handler ran to the state its result calls for; `None`
    /// is a handler that panicked.
    fn settle(
        &self,
        record: JobRecord,
        result: Option<Result<(), JobFailure>>,
    ) -> Result<JobOutcome, JobError> {
        let state = match result {
            Some(Ok(())) => self.finish(&record, JobState::Completed, None)?,
            Some(Err(JobFailure::Deferred { retry_after }))
                if record.attempts < MAX_DEFERRED_ATTEMPTS =>
            {
                let delay = deferral_delay(record.attempts, retry_after);
                let message = JobFailure::Deferred { retry_after }.as_str();
                if self.repository.defer(&record.id, delay, message)? {
                    JobState::Queued
                } else if self.repository.cancelled_by_project_purge(&record)? {
                    JobState::Cancelled
                } else {
                    return Err(JobError::InvalidTransition {
                        from: JobState::Running,
                        to: JobState::Queued,
                    });
                }
            }
            Some(Err(JobFailure::Deferred { .. })) => {
                self.finish(&record, JobState::Failed, Some(DEFERRED_TOO_OFTEN))?
            }
            Some(Err(failure)) => self.finish(&record, JobState::Failed, Some(failure.as_str()))?,
            None => self.finish(&record, JobState::Failed, Some(PANIC_MESSAGE))?,
        };
        Ok(JobOutcome {
            job_id: record.id,
            kind: record.kind,
            state,
            attempts: record.attempts,
        })
    }

    /// Moves a running job to its terminal state. A compare-and-set miss is a
    /// conflict unless the job's project was purged while it ran.
    fn finish(
        &self,
        record: &JobRecord,
        to: JobState,
        last_error: Option<&str>,
    ) -> Result<JobState, JobError> {
        if self
            .repository
            .transition(&record.id, JobState::Running, to, last_error)?
        {
            Ok(to)
        } else if self.repository.cancelled_by_project_purge(record)? {
            Ok(JobState::Cancelled)
        } else {
            Err(JobError::InvalidTransition {
                from: JobState::Running,
                to,
            })
        }
    }
}

impl<R: JobSettingsStore> Jobs<R> {
    /// "Análises em paralelo": provider calls at once, read at startup. A
    /// missing or out-of-range value reads as the default.
    pub fn parallel_analyses(&self) -> Result<u8, JobError> {
        Ok(self
            .repository
            .parallel_analyses()?
            .filter(|value| crate::limiter::PARALLEL_RANGE.contains(value))
            .unwrap_or(crate::limiter::DEFAULT_PARALLEL))
    }

    /// Saves "análises em paralelo"; it takes effect on the next start.
    pub fn set_parallel_analyses(&self, value: u8) -> Result<(), JobError> {
        if !crate::limiter::PARALLEL_RANGE.contains(&value) {
            return Err(JobError::InvalidSetting);
        }
        self.repository.set_parallel_analyses(value)
    }
}

impl<R> Jobs<R>
where
    R: JobRepository + Clone + Send + 'static,
{
    /// Starts the workers of every lane that has a registered kind, sized
    /// for a provider limit of `parallel` calls ([`Lane::workers`]). Each
    /// worker claims only kinds of its own lane.
    pub fn spawn_workers(&self, parallel: usize) -> WorkerHandle {
        let failure: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let mut threads = Vec::new();
        for lane in Lane::ALL {
            if self.kinds_in(lane).is_empty() {
                continue;
            }
            for index in 0..lane.workers(parallel) {
                let jobs = self.clone();
                let failure_thread = failure.clone();
                let spawned = std::thread::Builder::new()
                    .name(format!("xemnas-jobs-{}-{index}", lane.as_str()))
                    .spawn(move || worker_loop(jobs, lane, failure_thread));
                match spawned {
                    Ok(handle) => threads.push(handle),
                    Err(error) => record_failure(&failure, error.to_string()),
                }
            }
        }

        WorkerHandle {
            signal: self.signal.clone(),
            stop: self.stop.clone(),
            threads,
            failure,
        }
    }
}

/// Runs a handler on this thread in the lane of `kind`, with panic messages
/// sanitized; `None` when it panicked.
fn guarded<T>(kind: &str, handler: impl FnOnce() -> T) -> Option<T> {
    let previous = replace_job_panic_sanitized(true);
    let previous_lane = CURRENT_LANE.with(|lane| lane.replace(Some(lane_of(kind))));
    let result = catch_unwind(AssertUnwindSafe(handler));
    CURRENT_LANE.with(|lane| lane.set(previous_lane));
    replace_job_panic_sanitized(previous);
    result.ok()
}

/// Poll interval for the worker when no wakeup arrives.
const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Keeps the first failure a worker reports.
fn record_failure(failure: &Mutex<Option<String>>, message: String) {
    failure
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get_or_insert(message);
}

/// Body of one worker thread of `lane`.
fn worker_loop<R>(jobs: Jobs<R>, lane: Lane, failure: Arc<Mutex<Option<String>>>)
where
    R: JobRepository,
{
    let stop = jobs.stop.clone();
    let signal = jobs.signal.clone();
    loop {
        if stop.load(Ordering::SeqCst) {
            return;
        }
        match jobs.run_next_group_in(lane) {
            Ok(outcomes) if !outcomes.is_empty() => {
                for outcome in outcomes {
                    (jobs.observer)(JobEvent::Finished(outcome));
                }
                continue;
            }
            Ok(_) => {}
            Err(error) => {
                let message = error.to_string();
                record_failure(&failure, message.clone());
                (jobs.observer)(JobEvent::StorageError(message));
                return;
            }
        }

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

/// Handle to the worker threads of every lane.
pub struct WorkerHandle {
    signal: Arc<Signal>,
    stop: Arc<AtomicBool>,
    threads: Vec<std::thread::JoinHandle<()>>,
    failure: Arc<Mutex<Option<String>>>,
}

impl WorkerHandle {
    /// Number of worker threads running.
    pub fn len(&self) -> usize {
        self.threads.len()
    }

    /// Whether no worker thread started.
    pub fn is_empty(&self) -> bool {
        self.threads.is_empty()
    }

    /// Signals every worker to stop and wakes them immediately. A worker in
    /// the middle of a job finishes it first.
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

    /// Waits for every worker thread to finish.
    pub fn join(mut self) -> Result<(), JobError> {
        let mut crashed = false;
        for thread in self.threads.drain(..) {
            crashed |= thread.join().is_err();
        }
        if crashed {
            return Err(JobError::Storage(
                "a thread de jobs terminou inesperadamente".to_string(),
            ));
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

/// How much background work exists right now, for a status line.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct JobSummary {
    /// Jobs waiting for a worker.
    pub queued: usize,
    /// Jobs a worker is executing.
    pub running: usize,
    /// Jobs that ended in failure and were not retried.
    pub failed: usize,
}

impl JobSummary {
    /// Counts records by state. Completed and cancelled jobs are history.
    pub fn from_records(records: &[JobRecord]) -> Self {
        records.iter().fold(Self::default(), |mut summary, record| {
            summary.add(record.state, 1);
            summary
        })
    }

    /// Adds `count` jobs in `state`.
    pub fn add(&mut self, state: JobState, count: usize) {
        match state {
            JobState::Queued => self.queued += count,
            JobState::Running => self.running += count,
            JobState::Failed => self.failed += count,
            JobState::Completed | JobState::Cancelled => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        job_panic_is_sanitized, write_sanitized_panic_report, JobError, JobEvent, JobFailure,
        JobRecord, JobRepository, JobState, JobSummary, Jobs, RecoveryReport, PANIC_MESSAGE,
    };
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};

    /// In-memory repository that actually models state, for `reprocess` tests.
    #[derive(Clone, Default)]
    struct StateRepository {
        records: Arc<Mutex<Vec<JobRecord>>>,
    }

    impl JobRepository for StateRepository {
        fn insert(&self, record: &JobRecord) -> Result<(), JobError> {
            self.records
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(record.clone());
            Ok(())
        }

        fn get(&self, id: &str) -> Result<Option<JobRecord>, JobError> {
            Ok(self
                .records
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .iter()
                .find(|record| record.id == id)
                .cloned())
        }

        fn list(&self) -> Result<Vec<JobRecord>, JobError> {
            Ok(self
                .records
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone())
        }

        fn claim_next(&self, registered_kinds: &[String]) -> Result<Option<JobRecord>, JobError> {
            let mut records = self
                .records
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(record) = records.iter_mut().find(|record| {
                record.state == JobState::Queued && registered_kinds.contains(&record.kind)
            }) {
                record.state = JobState::Running;
                record.attempts += 1;
                return Ok(Some(record.clone()));
            }
            Ok(None)
        }

        fn transition(
            &self,
            id: &str,
            from: JobState,
            to: JobState,
            last_error: Option<&str>,
        ) -> Result<bool, JobError> {
            let mut records = self
                .records
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(record) = records
                .iter_mut()
                .find(|record| record.id == id && record.state == from)
            {
                record.state = to;
                record.last_error = last_error.map(str::to_string);
                return Ok(true);
            }
            Ok(false)
        }

        fn recover_interrupted(&self) -> Result<RecoveryReport, JobError> {
            Ok(RecoveryReport::default())
        }
    }

    #[test]
    fn reprocess_requeues_failed_and_clears_the_diagnostic() {
        let repository = StateRepository::default();
        let mut jobs = Jobs::new(repository.clone());
        jobs.register(
            "analysis",
            Arc::new(|_: &JobRecord| -> Result<(), JobFailure> { Err(JobFailure::Failed) }),
        );

        let record = jobs.enqueue("analysis", "{}", true).expect("enqueue");
        let outcome = jobs.run_next().expect("run").expect("the job ran");
        assert_eq!(outcome.state, JobState::Failed);
        assert_eq!(
            repository
                .get(&record.id)
                .expect("get")
                .expect("row")
                .last_error
                .as_deref(),
            Some(JobFailure::Failed.as_str())
        );

        jobs.reprocess(&record.id).expect("reprocess a failed job");
        let queued = repository.get(&record.id).expect("get").expect("row");
        assert_eq!(queued.state, JobState::Queued);
        assert_eq!(queued.last_error, None, "reprocess must clear last_error");

        let claimed = repository
            .claim_next(&["analysis".to_string()])
            .expect("claim")
            .expect("a claimable job");
        assert_eq!(claimed.attempts, 2);

        assert_eq!(
            jobs.reprocess(&record.id),
            Err(JobError::InvalidTransition {
                from: JobState::Running,
                to: JobState::Queued,
            })
        );
        assert_eq!(jobs.reprocess("missing"), Err(JobError::NotFound));
    }

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

    #[test]
    fn summary_counts_only_live_and_failed_jobs() {
        let record = |state| JobRecord {
            id: "j".into(),
            kind: "k".into(),
            payload: String::new(),
            state,
            idempotent: true,
            attempts: 1,
            last_error: None,
            created_at: String::new(),
            updated_at: String::new(),
        };
        let records = [
            record(JobState::Queued),
            record(JobState::Running),
            record(JobState::Running),
            record(JobState::Failed),
            record(JobState::Completed),
            record(JobState::Cancelled),
        ];
        assert_eq!(
            JobSummary::from_records(&records),
            JobSummary {
                queued: 1,
                running: 2,
                failed: 1
            }
        );
    }

    #[test]
    fn every_product_kind_has_a_lane_and_round_trips() {
        use super::{lane_of, JobKind, Lane, ANALYZE_CAPTURE_KIND, ANALYZE_DOCUMENT_KIND};
        for kind in JobKind::ALL {
            assert_eq!(JobKind::parse(kind.as_str()), Some(kind));
            assert_eq!(lane_of(kind.as_str()), kind.lane());
        }
        assert_eq!(lane_of(ANALYZE_CAPTURE_KIND), Lane::Now);
        assert_eq!(lane_of(ANALYZE_DOCUMENT_KIND), Lane::Documents);
        assert_eq!(
            lane_of(crate::relation_suggestions::RELATION_JOB_KIND),
            Lane::Suggestions
        );
        assert_eq!(
            lane_of(crate::claim_suggestions::CLAIM_JOB_KIND),
            Lane::Suggestions
        );
        for lane in Lane::ALL {
            assert!(
                JobKind::ALL.iter().any(|kind| kind.lane() == lane),
                "lane {lane:?} serves no kind"
            );
        }
    }

    #[test]
    fn registered_kinds_land_in_exactly_one_lane() {
        use super::{JobKind, Lane};
        let mut jobs = Jobs::new(StateRepository::default());
        for kind in JobKind::ALL {
            jobs.register(kind.as_str(), Arc::new(|_: &JobRecord| Ok(())));
            jobs.register(kind.as_str(), Arc::new(|_: &JobRecord| Ok(())));
        }
        let mut seen: Vec<&String> = Lane::ALL
            .into_iter()
            .flat_map(|lane| jobs.kinds_in(lane))
            .collect();
        assert_eq!(
            seen.len(),
            JobKind::ALL.len(),
            "re-registering never duplicates"
        );
        seen.sort();
        seen.dedup();
        assert_eq!(seen.len(), JobKind::ALL.len());
    }

    #[test]
    fn a_lane_worker_claims_only_its_kinds() {
        use super::{Lane, ANALYZE_CAPTURE_KIND, ANALYZE_DOCUMENT_KIND};
        let mut jobs = Jobs::new(StateRepository::default());
        jobs.register(ANALYZE_CAPTURE_KIND, Arc::new(|_: &JobRecord| Ok(())));
        jobs.register(ANALYZE_DOCUMENT_KIND, Arc::new(|_: &JobRecord| Ok(())));
        jobs.enqueue(ANALYZE_DOCUMENT_KIND, "doc", true)
            .expect("enqueue");
        assert_eq!(jobs.run_next_in(Lane::Now).expect("run"), None);
        let outcome = jobs
            .run_next_in(Lane::Documents)
            .expect("run")
            .expect("ran");
        assert_eq!(outcome.kind, ANALYZE_DOCUMENT_KIND);
        let summaries = jobs.lane_summaries().expect("summaries");
        assert_eq!(summaries.len(), Lane::ALL.len());
    }

    #[test]
    fn a_session_job_runs_while_the_documents_lane_is_saturated() {
        use super::{ANALYZE_CAPTURE_KIND, ANALYZE_DOCUMENT_KIND};
        use std::time::{Duration, Instant};
        let repository = StateRepository::default();
        let mut jobs = Jobs::new(repository.clone());
        let release = Arc::new(AtomicBool::new(false));
        let release_handler = release.clone();
        jobs.register(
            ANALYZE_DOCUMENT_KIND,
            Arc::new(move |_: &JobRecord| {
                while !release_handler.load(Ordering::SeqCst) {
                    std::thread::sleep(Duration::from_millis(5));
                }
                Ok(())
            }),
        );
        jobs.register(ANALYZE_CAPTURE_KIND, Arc::new(|_: &JobRecord| Ok(())));
        for _ in 0..6 {
            jobs.enqueue(ANALYZE_DOCUMENT_KIND, "doc", true)
                .expect("enqueue");
        }
        let handle = jobs.spawn_workers(2);
        let saturated = Instant::now() + Duration::from_secs(5);
        while repository
            .list()
            .expect("list")
            .iter()
            .filter(|job| job.state == JobState::Running)
            .count()
            < 2
        {
            assert!(Instant::now() < saturated, "document workers never started");
            std::thread::sleep(Duration::from_millis(2));
        }
        let session = jobs
            .enqueue(ANALYZE_CAPTURE_KIND, "s", true)
            .expect("enqueue");

        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let state = repository
                .get(&session.id)
                .expect("get")
                .expect("row")
                .state;
            if state == JobState::Completed {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "the session job waited: {state:?}"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        let documents = repository.list().expect("list");
        let running = documents
            .iter()
            .filter(|job| job.state == JobState::Running)
            .count();
        assert_eq!(running, 2, "both document workers are still busy");

        release.store(true, Ordering::SeqCst);
        handle.stop();
        handle.join().expect("clean shutdown");
        assert!(
            repository
                .list()
                .expect("list")
                .iter()
                .all(|job| job.state != JobState::Running),
            "stopping lets running jobs finish; none is left half done"
        );
    }

    #[test]
    fn a_deferred_job_goes_back_to_the_queue_until_it_gives_up() {
        use super::{DEFERRED_TOO_OFTEN, MAX_DEFERRED_ATTEMPTS};
        use std::time::Duration;
        let repository = StateRepository::default();
        let mut jobs = Jobs::new(repository.clone());
        jobs.register(
            "analysis",
            Arc::new(|_: &JobRecord| {
                Err(JobFailure::Deferred {
                    retry_after: Some(Duration::from_secs(7)),
                })
            }),
        );
        let record = jobs.enqueue("analysis", "{}", true).expect("enqueue");
        for attempt in 1..MAX_DEFERRED_ATTEMPTS {
            let outcome = jobs.run_next().expect("run").expect("ran");
            assert_eq!(
                outcome.state,
                JobState::Queued,
                "attempt {attempt} requeues"
            );
            let row = repository.get(&record.id).expect("get").expect("row");
            assert_eq!(row.state, JobState::Queued);
            assert_eq!(
                row.last_error.as_deref(),
                Some(JobFailure::Deferred { retry_after: None }.as_str())
            );
        }
        let outcome = jobs.run_next().expect("run").expect("ran");
        assert_eq!(outcome.state, JobState::Failed);
        let row = repository.get(&record.id).expect("get").expect("row");
        assert_eq!(row.last_error.as_deref(), Some(DEFERRED_TOO_OFTEN));
        jobs.reprocess(&record.id)
            .expect("a given-up job can be reprocessed");
    }

    #[test]
    fn deferral_delay_honours_retry_after_and_caps_the_backoff() {
        use super::{deferral_delay, MAX_BACKOFF};
        use std::time::Duration;
        assert_eq!(
            deferral_delay(1, Some(Duration::from_secs(7))),
            Duration::from_secs(7)
        );
        assert_eq!(
            deferral_delay(1, Some(Duration::from_secs(9_999))),
            MAX_BACKOFF
        );
        let first = deferral_delay(1, None);
        assert!(first >= Duration::from_millis(11_250) && first < Duration::from_millis(18_750));
        let third = deferral_delay(3, None);
        assert!(third >= Duration::from_secs(45) && third < Duration::from_secs(75));
        assert!(deferral_delay(40, None) <= MAX_BACKOFF);
    }

    #[test]
    fn the_handler_knows_its_lane() {
        use super::{current_lane, Lane, ANALYZE_DOCUMENT_KIND};
        let seen = Arc::new(Mutex::new(None));
        let seen_handler = seen.clone();
        let mut jobs = Jobs::new(StateRepository::default());
        jobs.register(
            ANALYZE_DOCUMENT_KIND,
            Arc::new(move |_: &JobRecord| {
                *seen_handler.lock().expect("lock") = current_lane();
                Ok(())
            }),
        );
        jobs.enqueue(ANALYZE_DOCUMENT_KIND, "d", true)
            .expect("enqueue");
        jobs.run_next().expect("run").expect("ran");
        assert_eq!(*seen.lock().expect("lock"), Some(Lane::Documents));
        assert_eq!(current_lane(), None, "restored after the handler");
    }
}
