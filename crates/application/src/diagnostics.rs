//! Sanitized operational diagnostics (MVP-SPEC §16, Gate 5).
//!
//! [`Diagnostics::export`] produces a **structural** document: versions, counts,
//! recent job/receipt/assessment metadata and the AI profile mode. It can never
//! carry substantive content — no artifact or diff text, no candidate/decision
//! question or rationale, no token or secret. The sanitization is by
//! construction: the document types have no field that could hold those values,
//! and job diagnostics expose a stable error code rather than the raw message.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::integration::outbox_status;
use crate::jobs::INTERRUPTED_NON_IDEMPOTENT;
use crate::profile::{
    consent_status, endpoint_host, AiSettings, ProfileKind, ProfileStore, SecretStore,
};

/// How many recent rows each list includes.
pub const RECENT_LIMIT: usize = 20;

/// Failure modes of the diagnostics use case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticsError {
    /// A storage query failed; the message is diagnostic only.
    Storage(String),
}

impl DiagnosticsError {
    /// Returns a short, stable code.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Storage(_) => "storage",
        }
    }
}

impl std::fmt::Display for DiagnosticsError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Storage(message) => write!(formatter, "falha ao montar o diagnóstico: {message}"),
        }
    }
}

impl std::error::Error for DiagnosticsError {}

/// Versions section.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaInfo {
    /// Application crate version.
    pub app_version: String,
    /// Highest applied migration.
    pub migrations_version: i64,
}

/// Database counts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticsCounts {
    /// Tracked projects.
    pub projects: i64,
    /// Capture receipts.
    pub captures: i64,
    /// Capture artifacts.
    pub artifacts: i64,
    /// Candidates grouped by status literal.
    pub candidates_by_status: BTreeMap<String, i64>,
    /// Engineering decisions.
    pub decisions: i64,
    /// Decision revisions.
    pub revisions: i64,
    /// Assessments grouped by outcome literal.
    pub assessments_by_outcome: BTreeMap<String, i64>,
    /// Jobs grouped by state literal.
    pub jobs_by_state: BTreeMap<String, i64>,
}

/// Outbox file counts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboxCounts {
    /// Files waiting in `pending/`.
    pub pending: i64,
    /// Files archived in `accepted/`.
    pub accepted: i64,
    /// Files archived in `rejected/`.
    pub rejected: i64,
    /// Intact items parked in `stalled/` after repeated refusals.
    #[serde(default)]
    pub stalled: i64,
}

/// One recent job, without its raw diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobDiagnostic {
    /// Job identifier.
    pub id: String,
    /// Job kind.
    pub kind: String,
    /// Persisted state literal.
    pub state: String,
    /// Attempts recorded.
    pub attempts: i64,
    /// RFC 3339 last update.
    pub updated_at: String,
    /// Stable error code, never the raw message.
    pub error_code: Option<String>,
}

/// One recent receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptDiagnostic {
    /// Capture identifier.
    pub capture_id: String,
    /// Artifact count.
    pub artifact_count: i64,
    /// RFC 3339 receipt time.
    pub received_at: String,
    /// Project identifier resolved from the receipt location, when registered.
    ///
    /// The canonical path is deliberately **not** exported: it can embed a user
    /// name or other sensitive text, and the receipt-to-project association is
    /// preserved by this opaque id instead.
    pub project_id: Option<String>,
}

/// One recent assessment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssessmentDiagnostic {
    /// Outcome literal.
    pub outcome: String,
    /// Stable error code, when any.
    pub error_code: Option<String>,
}

/// AI profile section, without config values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiProfileDiagnostic {
    /// Profile kind literal.
    pub kind: String,
    /// Provider host for external profiles, never the full URL.
    pub provider: Option<String>,
    /// Whether a secret is stored.
    pub has_secret: bool,
    /// Consent status literal.
    pub consent_status: String,
}

/// Runtime mode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeDiagnostic {
    /// Operating system.
    pub platform: String,
    /// `external` when a consented provider is active, otherwise `offline`.
    pub mode: String,
}

/// A percentile distribution over integer millisecond deltas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Distribution {
    /// Number of valid samples.
    pub samples: i64,
    /// Median in milliseconds, when there is at least one sample.
    pub p50: Option<i64>,
    /// 95th percentile in milliseconds, when there is at least one sample.
    pub p95: Option<i64>,
}

/// Noise metrics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NoiseMetrics {
    /// Decided candidates: dismissed + accepted + edited_and_accepted.
    pub decided_total: i64,
    /// `dismissed / decided_total`, rounded to 4 decimals; `None` with no decided.
    pub dismissed_ratio: Option<f64>,
}

/// Loss metrics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LossMetrics {
    /// Assessments that failed.
    pub assessments_failed: i64,
    /// Assessments skipped because consent was missing.
    pub assessments_skipped: i64,
    /// Jobs that failed.
    pub jobs_failed: i64,
    /// Outbox files in `rejected/`.
    pub outbox_rejected: i64,
}

/// Operational metrics for the dogfood (spec §780).
///
/// Aggregates only: no candidate text, no artifact content, no per-row export.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticsMetrics {
    /// Capture receipt → candidate creation latency.
    pub latency_capture_to_candidate_ms: Distribution,
    /// Candidate creation → decision confirmation time.
    pub review_time_ms: Distribution,
    /// Noise: the dismissed share of decided candidates.
    pub noise: NoiseMetrics,
    /// Losses: failed/skipped work.
    pub losses: LossMetrics,
}

/// The exported diagnostics document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticsDocument {
    /// Version information.
    pub schema: SchemaInfo,
    /// Database counts.
    pub counts: DiagnosticsCounts,
    /// Operational metrics.
    pub metrics: DiagnosticsMetrics,
    /// Outbox file counts.
    pub outbox: OutboxCounts,
    /// Recent jobs (metadata only).
    pub recent_jobs: Vec<JobDiagnostic>,
    /// Recent receipts (metadata only).
    pub recent_receipts: Vec<ReceiptDiagnostic>,
    /// Recent assessments (outcome only).
    pub recent_assessments: Vec<AssessmentDiagnostic>,
    /// AI profile shape.
    pub ai_profile: AiProfileDiagnostic,
    /// Runtime mode.
    pub runtime: RuntimeDiagnostic,
}

/// Raw rows the store returns; none of them carry artifact or candidate text
/// except `last_error`, which the use case maps to a code and never copies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobDiagnosticRow {
    /// Job identifier.
    pub id: String,
    /// Job kind.
    pub kind: String,
    /// Persisted state literal.
    pub state: String,
    /// Attempts recorded.
    pub attempts: i64,
    /// RFC 3339 last update.
    pub updated_at: String,
    /// Raw diagnostic, mapped to a code by the use case.
    pub last_error: Option<String>,
}

/// Raw recent-receipt row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiptDiagnosticRow {
    /// Capture identifier.
    pub capture_id: String,
    /// Artifact count.
    pub artifact_count: i64,
    /// RFC 3339 receipt time.
    pub received_at: String,
    /// Project identifier resolved from the receipt location, when registered.
    pub project_id: Option<String>,
}

/// Raw recent-assessment row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssessmentDiagnosticRow {
    /// Outcome literal.
    pub outcome: String,
    /// Stable error code.
    pub error_code: Option<String>,
}

/// Persistence port the diagnostics use case needs.
pub trait DiagnosticsStore {
    /// Highest applied migration version.
    fn migrations_version(&self) -> Result<i64, DiagnosticsError>;

    /// Database counts.
    fn counts(&self) -> Result<DiagnosticsCounts, DiagnosticsError>;

    /// Operational metrics; `losses.outbox_rejected` is filled by the use case.
    fn metrics(&self) -> Result<DiagnosticsMetrics, DiagnosticsError>;

    /// Recent jobs, newest first.
    fn recent_jobs(&self, limit: usize) -> Result<Vec<JobDiagnosticRow>, DiagnosticsError>;

    /// Recent receipts, newest first.
    fn recent_receipts(&self, limit: usize) -> Result<Vec<ReceiptDiagnosticRow>, DiagnosticsError>;

    /// Recent assessments, newest first.
    fn recent_assessments(
        &self,
        limit: usize,
    ) -> Result<Vec<AssessmentDiagnosticRow>, DiagnosticsError>;
}

/// Diagnostics use case over a [`DiagnosticsStore`] and the AI settings.
#[derive(Debug, Clone)]
pub struct Diagnostics<S, P, K> {
    store: S,
    settings: AiSettings<P, K>,
    outbox_dir: PathBuf,
}

impl<S, P, K> Diagnostics<S, P, K>
where
    S: DiagnosticsStore,
    P: ProfileStore,
    K: SecretStore,
{
    /// Wraps the store, the AI settings and the outbox root to count.
    pub fn new(store: S, settings: AiSettings<P, K>, outbox_dir: impl Into<PathBuf>) -> Self {
        Self {
            store,
            settings,
            outbox_dir: outbox_dir.into(),
        }
    }

    /// Builds the sanitized diagnostics document.
    pub fn export(&self) -> Result<DiagnosticsDocument, DiagnosticsError> {
        let schema = SchemaInfo {
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            migrations_version: self.store.migrations_version()?,
        };
        let counts = self.store.counts()?;
        let mut metrics = self.store.metrics()?;
        let recent_jobs = self
            .store
            .recent_jobs(RECENT_LIMIT)?
            .into_iter()
            .map(|row| JobDiagnostic {
                id: row.id,
                kind: row.kind,
                state: row.state,
                attempts: row.attempts,
                updated_at: row.updated_at,
                error_code: job_error_code(row.last_error.as_deref()).map(str::to_string),
            })
            .collect();
        let recent_receipts = self
            .store
            .recent_receipts(RECENT_LIMIT)?
            .into_iter()
            .map(|row| ReceiptDiagnostic {
                capture_id: row.capture_id,
                artifact_count: row.artifact_count,
                received_at: row.received_at,
                project_id: row.project_id,
            })
            .collect();
        let recent_assessments = self
            .store
            .recent_assessments(RECENT_LIMIT)?
            .into_iter()
            .map(|row| AssessmentDiagnostic {
                outcome: row.outcome,
                error_code: row.error_code,
            })
            .collect();
        let status = outbox_status(&self.outbox_dir);
        let outbox = OutboxCounts {
            pending: status.pending,
            accepted: status.accepted,
            rejected: status.rejected,
            stalled: status.stalled,
        };
        metrics.losses.outbox_rejected = outbox.rejected;

        let mut ai_profile = AiProfileDiagnostic {
            kind: "unavailable".to_string(),
            provider: None,
            has_secret: false,
            consent_status: "unavailable".to_string(),
        };
        let mut mode = "offline".to_string();
        if let Ok(profile) = self.settings.load_or_seed() {
            let status = self.settings.status().ok();
            ai_profile.kind = match profile.kind {
                ProfileKind::Fake => "fake".to_string(),
                ProfileKind::OpenAiCompatible => "openai-compatible".to_string(),
            };
            ai_profile.provider = profile.endpoint.as_deref().and_then(endpoint_host);
            ai_profile.has_secret = status.map(|status| status.has_secret).unwrap_or(false);
            ai_profile.consent_status = match consent_status(&profile) {
                Ok(()) => "valid".to_string(),
                Err(reason) => reason.to_string(),
            };
            if consent_status(&profile).is_ok() {
                mode = "external".to_string();
            }
        }

        Ok(DiagnosticsDocument {
            schema,
            counts,
            metrics,
            outbox,
            recent_jobs,
            recent_receipts,
            recent_assessments,
            ai_profile,
            runtime: RuntimeDiagnostic {
                platform: std::env::consts::OS.to_string(),
                mode,
            },
        })
    }
}

/// Maps a raw job diagnostic to a stable code; the raw text never leaves here.
///
/// The jobs layer writes fixed product messages, so an unknown value still maps
/// to `failed` rather than leaking its content.
pub fn job_error_code(last_error: Option<&str>) -> Option<&'static str> {
    match last_error {
        None => None,
        Some(message) if message == INTERRUPTED_NON_IDEMPOTENT => Some("interrupted"),
        Some(_) => Some("failed"),
    }
}

#[cfg(test)]
mod tests {
    use super::endpoint_host;

    #[test]
    fn endpoint_host_keeps_only_the_hostname() {
        assert_eq!(
            endpoint_host("https://user:SECRET-MARKER-PW@exemplo.test/v1?chave=SECRET-MARKER-QS")
                .as_deref(),
            Some("exemplo.test")
        );
        assert_eq!(
            endpoint_host("https://api.example.test:8443/v1").as_deref(),
            Some("api.example.test")
        );
        assert_eq!(
            endpoint_host("http://exemplo.test#frag").as_deref(),
            Some("exemplo.test")
        );
        assert_eq!(
            endpoint_host("https://exemplo.test").as_deref(),
            Some("exemplo.test")
        );
        assert_eq!(endpoint_host("exemplo.test/v1"), None);
        assert_eq!(endpoint_host("https://user@/v1"), None);
    }
}
