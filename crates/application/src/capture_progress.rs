//! Read-only capture destination projection; an empty list means never received.

/// Safe terminal assessment explanation, never provider response text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssessmentReason {
    /// Durable candidates were produced.
    Candidates,
    /// Only implementation details were classified.
    Detail,
    /// No proposals were produced.
    Empty,
    /// Extraction failed.
    Failed,
    /// Consent blocked extraction.
    Skipped,
    /// Legacy or unrecognized reason.
    Unknown,
}

/// State of the current job, not of a previous retry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureState {
    /// Waiting for execution.
    Queued,
    /// Currently executing.
    Running,
    /// Execution completed.
    Completed,
    /// Execution failed.
    Failed,
    /// Execution skipped.
    Skipped,
    /// Execution cancelled.
    Cancelled,
    /// Insufficient or unrecognized persisted state.
    Unknown,
}

/// Candidate lifecycle counts, including hidden low-significance rows.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CandidateCounts {
    /// Visible pending candidates.
    pub pending: usize,
    /// Low-significance pending candidates.
    pub hidden: usize,
    /// Accepted or edited-and-accepted candidates.
    pub adopted: usize,
    /// Dismissed candidates.
    pub dismissed: usize,
    /// Snoozed candidates.
    pub snoozed: usize,
}

/// One receipt and the result of its current attempt, read in one snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureProgress {
    /// Owning project.
    pub project_id: String,
    /// Received capture.
    pub capture_id: String,
    /// Receipt timestamp.
    pub received_at: String,
    /// Current job identifier.
    pub job_id: Option<String>,
    /// Current attempt counter.
    pub attempts: i64,
    /// Current job state.
    pub state: CaptureState,
    /// Assessment reason of this attempt only.
    pub reason: AssessmentReason,
    /// Provider or fake adapter of this assessment.
    pub source: Option<String>,
    /// External model, when known.
    pub model: Option<String>,
    /// Durable classifications before deduplication.
    pub durable: usize,
    /// Detail classifications, never stored as candidates.
    pub detail: usize,
    /// Persisted candidate lifecycle counts.
    pub candidates: CandidateCounts,
    /// Whether backend job rules allow retry.
    pub can_retry: bool,
}

/// Snapshot persistence port, scoped by project and optionally capture.
pub trait CaptureProgressStore {
    /// Returns latest receipts; unknown/foreign captures return an empty list.
    fn capture_progress(
        &self,
        project_id: &str,
        capture_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<CaptureProgress>, String>;
}

/// Read-only application entry point.
pub struct CaptureProgressReader<S>(pub S);
impl<S: CaptureProgressStore> CaptureProgressReader<S> {
    /// Latest captures, capped to 100 receipts.
    pub fn recent(&self, project_id: &str, limit: usize) -> Result<Vec<CaptureProgress>, String> {
        self.0
            .capture_progress(project_id, None, limit.clamp(1, 100))
    }
    /// One capture only if it belongs to this project.
    pub fn capture(
        &self,
        project_id: &str,
        capture_id: &str,
    ) -> Result<Option<CaptureProgress>, String> {
        Ok(self
            .0
            .capture_progress(project_id, Some(capture_id), 1)?
            .pop())
    }
}
