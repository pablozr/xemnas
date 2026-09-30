//! Application layer: use-case orchestration and the ports infrastructure implements.
#![warn(missing_docs)]

pub mod analysis;
pub mod captures;
pub mod claims;
mod clock;
pub mod context;
pub mod context_settings;
pub mod decisions;
pub mod diagnostics;
pub mod export;
pub mod extract;
pub mod inbox;
pub mod injection;
pub mod integration;
pub mod jobs;
pub mod outbox;
pub mod paths;
pub mod profile;
pub mod projects;
pub mod redact;
pub mod relations;

pub use analysis::{AnalysisOutcome, AnalyzeCapture, ExtractorFactory};
pub use captures::{
    CaptureApi, CaptureArtifactRecord, CaptureCheckpointRecord, CaptureError, CaptureIngest,
    CaptureReceiptRecord, CaptureRepository, CaptureWrite, IngestError, IngestOutcome, Receipt,
};
pub use claims::{ClaimRecord, ClaimStore, Claims, ClaimsError, NewClaim};
pub use context::{
    ContextError, ContextPack, ContextPacks, ContextProvider, ContextRequest, ContextStore,
    PackClaim, PackDecision,
};
pub use context_settings::{ContextMode, ContextSettings, ProjectContextSettings};
pub use decisions::{
    sanitize_match_query, DecisionDetail, DecisionEdits, DecisionFilter, DecisionPage,
    DecisionRevision, DecisionSearchHit, DecisionStatus, DecisionSummary, Decisions,
    DecisionsError, SearchQuery, MAX_ARRAY_ITEMS, MAX_ARRAY_ITEM_CHARS,
};
pub use diagnostics::{
    AiProfileDiagnostic, AssessmentDiagnostic, ContextMetrics, ContextModeMetrics, Diagnostics,
    DiagnosticsCounts, DiagnosticsDocument, DiagnosticsError, DiagnosticsMetrics, Distribution,
    JobDiagnostic, LossMetrics, NoiseMetrics, OutboxCounts, ReceiptDiagnostic, RuntimeDiagnostic,
    SchemaInfo,
};
pub use export::{
    preview_pack, write_pack, Export, ExportDocument, ExportError, ExportFormat, ExportResult,
    PackDocument,
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
    canonicalize_location, ProjectError, ProjectRecord, ProjectRepository, Projects, RemovalError,
    RemovalImpact,
};
pub use relations::{
    DecisionRelations, RelationDirection, RelationInsert, RelationRow, RelationStore, RelationView,
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
