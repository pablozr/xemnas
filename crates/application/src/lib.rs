//! Application layer: use-case orchestration and the ports infrastructure implements.
//!
//! Per ARCH-001 this crate depends only on [`domain`] and defines the
//! interfaces (ports) it needs, so infrastructure can implement them without
//! the domain knowing about storage or transport. It holds the Project and Job
//! use cases and the port traits their repositories implement.
#![warn(missing_docs)]

pub mod captures;
mod clock;
pub mod decisions;
pub mod diagnostics;
pub mod export;
pub mod extract;
pub mod inbox;
pub mod integration;
pub mod jobs;
pub mod outbox;
pub mod paths;
pub mod profile;
pub mod projects;
pub mod redact;

pub use captures::{
    CaptureApi, CaptureArtifactRecord, CaptureCheckpointRecord, CaptureError, CaptureIngest,
    CaptureReceiptRecord, CaptureRepository, CaptureWrite, IngestError, IngestOutcome, Receipt,
};
pub use extract::{
    connection_test_evidence, fail_provider_setup, filter_relevant, input_hash, policy_snapshot,
    record_skipped_assessment, run_connection_test, run_extraction, truncate_content,
    AssessmentOutcome, AssessmentRecord, AssessmentStore, CandidateExtractor, CandidateProposal,
    ConnectionTestReport, DecisionCandidateRecord, DecisionEvidence, EvidenceArtifact,
    ExtractError, ExtractionReport, ExtractionStore, FakeCandidateExtractor, ProviderSetupError,
    RelevanceSignal, RunContext, ERROR_CODE_CONSENT, ERROR_CODE_KEYSTORE, ERROR_CODE_PROFILE,
    ERROR_CODE_PROVIDER_CONFIG, ERROR_CODE_SECRET, MAX_EVIDENCE_ARTIFACTS,
    MAX_EVIDENCE_CONTENT_BYTES,
};
// The Inbox port plumbing (`InboxStore`, `InboxQuery`, `Cursor`,
// `StoredCandidate`, `ValidatedEdits`) stays in `application::inbox` for the
// storage implementation and test fakes; only the front-facing surface is
// re-exported at the crate root.
pub use decisions::{
    sanitize_match_query, DecisionDetail, DecisionEdits, DecisionFilter, DecisionPage,
    DecisionRevision, DecisionSearchHit, DecisionStatus, DecisionSummary, Decisions,
    DecisionsError, SearchQuery, MAX_ARRAY_ITEMS, MAX_ARRAY_ITEM_CHARS,
};
pub use diagnostics::{
    AiProfileDiagnostic, AssessmentDiagnostic, Diagnostics, DiagnosticsCounts, DiagnosticsDocument,
    DiagnosticsError, DiagnosticsMetrics, Distribution, JobDiagnostic, LossMetrics, NoiseMetrics,
    OutboxCounts, ReceiptDiagnostic, RuntimeDiagnostic, SchemaInfo,
};
pub use export::{Export, ExportDocument, ExportError, ExportFormat, ExportResult};

// Re-exported so integration tests in other crates can build a Capture Envelope
// (and its metadata map) without adding a new dependency to those crates.
pub use inbox::{
    ArtifactView, CandidateDetail, CandidateEdits, CandidateStatus, CandidateSummary,
    ConfirmOutcome, DiffSummary, Inbox, InboxError, InboxFilter, InboxPage, DEFAULT_PAGE_LIMIT,
    MAX_BATCH_IDS, MAX_CHOICE_CHARS, MAX_PAGE_LIMIT, MAX_QUESTION_CHARS, MAX_RATIONALE_CHARS,
};
pub use integration::{
    AdapterStatus, CheckKind, CheckOutcome, Compatibility, Integration, IntegrationCheck,
    IntegrationEnvironment, IntegrationError, IntegrationState, IntegrationStatus,
    LocalApiEndpoint, OutboxStatus,
};
pub use integration_contracts::capture::{
    artifact_fingerprint, ArtifactKind, CaptureEnvelope, CaptureSource, ProjectRef, SourceArtifact,
};
pub use jobs::{
    install_panic_sanitizer, job_panic_is_sanitized, write_sanitized_panic_report, JobError,
    JobEvent, JobFailure, JobOutcome, JobRecord, JobRepository, JobState, Jobs, RecoveryReport,
    WorkerHandle, ANALYZE_CAPTURE_KIND, INTERRUPTED_NON_IDEMPOTENT,
};
pub use outbox::{
    drain, drain_with, retry_stalled, DrainPolicy, DrainReport, OutboxError,
    DEFAULT_MAX_TRANSIENT_ATTEMPTS, DEFAULT_REJECTED_RETENTION,
};
pub use paths::AppPaths;
pub use profile::{
    build_preview, choose_extractor, grant_consent, offline_default_profile, preview_hash,
    revoke_consent, AiProfile, AiSettings, AiStatus, ConsentPreview, ConsentRecord,
    ExtractorChoice, FileProfileStore, PreviewCategory, ProfileError, ProfileKind, ProfileStore,
    SecretStore, DEFAULT_MAX_INPUT_CHARS, PREVIEW_CATEGORIES,
};
pub use projects::{
    canonicalize_location, ProjectError, ProjectRecord, ProjectRepository, Projects,
};
pub use serde_json;

/// Returns a stable identifier for this crate, used by wiring smoke tests.
pub fn crate_name() -> &'static str {
    "application"
}

#[cfg(test)]
mod tests {
    use super::crate_name;

    #[test]
    fn smoke() {
        assert_eq!(crate_name(), "application");
    }

    #[test]
    fn depends_on_domain() {
        assert_eq!(domain::crate_name(), "domain");
    }
}
