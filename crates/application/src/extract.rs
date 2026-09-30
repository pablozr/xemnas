//! Offline, deterministic extraction of Decision Candidates from a capture.
//!
//! Ticket 12 implements only the cheap, pure first pass (§7.4) and a fake,
//! deterministic `CandidateExtractor` (§12): no network, no model, no provider.
//! The use case loads the persisted evidence, decides relevance with
//! [`filter_relevant`], asks the extractor for proposals and stores them as
//! `pending` candidates. It **never** creates an Engineering Decision: a human
//! confirms or rejects every candidate later (ticket 15).
//!
//! Reproducibility is a hard requirement: the same evidence yields the same
//! proposals field by field (only the row id and timestamps are generated at
//! persistence time), and re-running never duplicates a candidate because
//! [`ExtractionStore::insert_candidates`] is guarded by the unique
//! `dedup_hash`.
//!
//! # Veto rules (MVP-SPEC §6 exclusions)
//!
//! Before any positive signal is considered, [`filter_relevant`] applies cheap,
//! deterministic, case-insensitive vetoes that make the capture irrelevant:
//!
//! - **test-only diff**: every `diff --git` path is inside a test location
//!   (`tests/`, `test/`, `__tests__/`, or a `_test.`/`.test.`/`.spec.` file);
//! - **comment-only diff**: every added/removed line is a comment;
//! - **explicit trivial marker**: the text says it is formatting, whitespace,
//!   lint, a rename or a comment-only change ("formatting", "whitespace",
//!   "renamed", "no behavior change", ...).
//!
//! **Precedence.** A real structural strong signal defeats the trivial wording
//! (§6: a strong condition already justifies a candidate). Structural signals
//! are read **only from `diff_hunk` artifacts**: added (`+`) lines that are not
//! comments, in non-test files. A `.sql` path alone never counts — a real DDL
//! statement (`CREATE TABLE`/`ALTER TABLE`/`ADD COLUMN`/`DROP TABLE`/
//! `CREATE INDEX`) must be on the added line; `INSERT`/`UPDATE` are data
//! changes and do not count. A dependency added to a manifest, or a
//! security-relevant code line, also counts. With no `diff_hunk` at all there is
//! nothing durable to assess, so the capture is vetoed; incidental words like
//! "api" or "schema" in prose or in a trivial line do not count as structural.
//!
//! The envelope carries no semantic diff, so these rules are honest about their
//! limits: they are textual/structural cues, not a code diff. A capture that
//! mentions "schema" or "api" incidentally but is one of the cases above is
//! excluded, and there are fixtures that prove it.

use std::collections::BTreeSet;

use integration_contracts::capture::artifact_fingerprint;
use serde_json::{json, Value};

use crate::clock::now_rfc3339;
use crate::jobs::JobFailure;
use crate::profile::{AiProfile, ProfileKind};

/// Maximum file names accepted inside a `diff_summary`.
pub const MAX_DIFF_SUMMARY_FILES: usize = 500;

/// Maximum artifacts loaded for one capture, as a defensive bound.
pub const MAX_EVIDENCE_ARTIFACTS: usize = 200;

/// Maximum bytes kept per artifact content, as a defensive bound.
pub const MAX_EVIDENCE_CONTENT_BYTES: usize = 64 * 1024;

/// One artifact loaded as extraction evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceArtifact {
    /// Stable artifact identifier (UUID v7).
    pub artifact_id: String,
    /// Persisted `snake_case` artifact kind.
    pub kind: String,
    /// Already-redacted, already-bounded content.
    pub content: String,
    /// Serialized metadata object.
    pub metadata: String,
}

/// Everything the extractor may look at for one capture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionEvidence {
    /// Capture the evidence belongs to.
    pub capture_id: String,
    /// Project the capture was accepted for.
    pub project_id: String,
    /// Adapter that produced the capture, when a checkpoint records it.
    pub adapter: Option<String>,
    /// Adapter session identifier, when a checkpoint records it.
    pub session_id: Option<String>,
    /// RFC 3339 observation time, when recorded.
    pub observed_at: Option<String>,
    /// The capture artifacts (content already redacted at ingest).
    pub artifacts: Vec<EvidenceArtifact>,
}

/// A durable engineering choice worth proposing for human confirmation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RelevanceSignal {
    /// Touches a public contract, schema or persistence format.
    PublicContract,
    /// Touches security, privacy or compliance.
    SecurityPrivacy,
    /// Adds a durable external dependency.
    DependencyAdded,
    /// Changes more than one top-level component.
    CrossesBoundaries,
    /// Hard or expensive to revert.
    HardToRevert,
    /// Explicitly rejects a plausible alternative.
    RejectsAlternative,
    /// Conditions work that comes later.
    ConditionsFutureWork,
    /// Material blast radius if wrong.
    MaterialBlastRadius,
    /// Compares alternatives.
    AlternativesCompared,
    /// States an explicit trade-off.
    ExplicitTradeoff,
    /// Expresses disagreement or uncertainty.
    DisagreementUncertainty,
    /// Mentions a relevant cost.
    RelevantCost,
    /// Claims validity over months.
    ValidityMonths,
    /// Concerns maintenance or onboarding.
    MaintenanceOnboarding,
    /// Delegates the choice to an agent.
    DelegatedToAgent,
    /// Rests on an unproven assumption.
    UnprovenAssumption,
}

impl RelevanceSignal {
    /// Returns the persisted literal for this signal.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PublicContract => "public_contract",
            Self::SecurityPrivacy => "security_privacy",
            Self::DependencyAdded => "dependency_added",
            Self::CrossesBoundaries => "crosses_boundaries",
            Self::HardToRevert => "hard_to_revert",
            Self::RejectsAlternative => "rejects_alternative",
            Self::ConditionsFutureWork => "conditions_future_work",
            Self::MaterialBlastRadius => "material_blast_radius",
            Self::AlternativesCompared => "alternatives_compared",
            Self::ExplicitTradeoff => "explicit_tradeoff",
            Self::DisagreementUncertainty => "disagreement_uncertainty",
            Self::RelevantCost => "relevant_cost",
            Self::ValidityMonths => "validity_months",
            Self::MaintenanceOnboarding => "maintenance_onboarding",
            Self::DelegatedToAgent => "delegated_to_agent",
            Self::UnprovenAssumption => "unproven_assumption",
        }
    }

    /// Returns `true` when a single occurrence already makes the capture
    /// relevant (MVP-SPEC §6).
    pub fn is_strong(&self) -> bool {
        matches!(
            self,
            Self::PublicContract
                | Self::SecurityPrivacy
                | Self::DependencyAdded
                | Self::CrossesBoundaries
                | Self::HardToRevert
                | Self::RejectsAlternative
                | Self::ConditionsFutureWork
                | Self::MaterialBlastRadius
        )
    }
}

/// A candidate proposed by an extractor, before persistence.
#[derive(Debug, Clone, PartialEq)]
pub struct CandidateProposal {
    /// The decision question a human should answer.
    pub question: String,
    /// The proposed choice.
    pub choice: String,
    /// Why the capture suggests this choice (marked as inferred).
    pub rationale: String,
    /// Confidence in `[0.0, 1.0]`.
    pub confidence: f64,
    /// Explanation of the confidence.
    pub confidence_reason: String,
    /// Relevance signals that made the capture relevant.
    pub signals: Vec<RelevanceSignal>,
    /// Artifact ids supporting the candidate.
    pub evidence_refs: Vec<String>,
    /// JSON `{files:[...], artifacts:n}`; never raw diff content.
    pub diff_summary: String,
}

/// A row to persist in `decision_candidates`.
#[derive(Debug, Clone, PartialEq)]
pub struct DecisionCandidateRecord {
    /// Generated identifier (UUID v7).
    pub id: String,
    /// Project the candidate belongs to.
    pub project_id: String,
    /// Capture the candidate was extracted from.
    pub capture_id: String,
    /// Lifecycle status; ticket 12 only writes `pending`.
    pub status: String,
    /// The decision question.
    pub question: String,
    /// The proposed choice.
    pub choice: String,
    /// The inference rationale.
    pub rationale: String,
    /// JSON array of signal names.
    pub signals: String,
    /// Confidence in `[0.0, 1.0]`.
    pub confidence: f64,
    /// Explanation of the confidence.
    pub confidence_reason: String,
    /// JSON array of artifact ids.
    pub evidence_refs: String,
    /// JSON `{files:[...], artifacts:n}`.
    pub diff_summary: String,
    /// Unique hash used for deduplication.
    pub dedup_hash: String,
    /// RFC 3339 creation timestamp.
    pub created_at: String,
    /// RFC 3339 last-update timestamp.
    pub updated_at: String,
}

/// Failure modes of extraction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtractError {
    /// Loading evidence or writing candidates failed.
    Storage(String),
    /// The extractor itself failed; the capture stays valid.
    Extractor(String),
    /// The extractor answered, but a proposal broke the candidate contract
    /// (empty field, confidence out of range, unknown evidence ref, ...).
    Validation(String),
}

impl std::fmt::Display for ExtractError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Storage(message) => write!(formatter, "falha de armazenamento: {message}"),
            Self::Extractor(message) => write!(formatter, "falha do extrator: {message}"),
            Self::Validation(message) => write!(formatter, "proposta inválida: {message}"),
        }
    }
}

impl std::error::Error for ExtractError {}

impl ExtractError {
    /// Returns a short, stable code for provenance (`assessments.error_code`).
    ///
    /// The code is a fixed literal, never free-form text, so it is safe to store
    /// and to show (PRIV-001).
    pub fn code(&self) -> &'static str {
        match self {
            Self::Storage(_) => "storage",
            Self::Extractor(_) => "extractor",
            Self::Validation(_) => "validation",
        }
    }
}

/// Outcome recorded in the `assessments.outcome` column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssessmentOutcome {
    /// The extractor ran and produced a validated batch (possibly empty).
    Ok,
    /// The relevance filter found nothing durable to assess.
    Empty,
    /// The extractor or validation failed; the capture is untouched.
    Failed,
    /// External calls were blocked by consent; nothing was attempted.
    Skipped,
}

impl AssessmentOutcome {
    /// Returns the literal persisted in the `outcome` column.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Empty => "empty",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
        }
    }
}

/// One provenance row written to `assessments` (MVP-SPEC §12 line 645).
///
/// The row carries only metadata and hashes, never artifact content or a model
/// response body (PRIV-001).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssessmentRecord {
    /// Generated identifier (UUID v7).
    pub id: String,
    /// Capture this assessment ran for.
    pub capture_id: String,
    /// Job that triggered the run, when any.
    pub job_id: Option<String>,
    /// AI Execution Profile identifier.
    pub profile_id: String,
    /// Adapter literal: `fake` or `openai-compatible`.
    pub adapter: String,
    /// Model name for external profiles.
    pub model: Option<String>,
    /// JSON snapshot of the profile policy.
    pub policy: String,
    /// Consent preview hash for external profiles; `None` for the fake.
    pub consent_preview_hash: Option<String>,
    /// Stable hash of the assessed inputs.
    pub input_hash: String,
    /// RFC 3339 start time.
    pub started_at: String,
    /// RFC 3339 finish time.
    pub finished_at: String,
    /// Terminal outcome.
    pub outcome: AssessmentOutcome,
    /// Proposals produced.
    pub candidates: i64,
    /// Candidates inserted.
    pub inserted: i64,
    /// Short stable failure code, when the outcome is `failed` or `skipped`.
    pub error_code: Option<String>,
}

/// Persistence port for assessment provenance rows.
pub trait AssessmentStore {
    /// Inserts one assessment row.
    fn record_assessment(&self, row: &AssessmentRecord) -> Result<(), ExtractError>;
}

/// Provenance context for one extraction run.
///
/// The composition root builds it from the loaded profile and the claimed job;
/// tests use [`RunContext::for_tests`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunContext {
    /// AI Execution Profile identifier.
    pub profile_id: String,
    /// Adapter literal: `fake` or `openai-compatible`.
    pub adapter: String,
    /// Model name, when an external provider runs.
    pub model: Option<String>,
    /// Static JSON snapshot of the profile policy.
    pub policy_json: String,
    /// Consent preview hash, when the profile carries consent.
    pub consent_preview_hash: Option<String>,
    /// Job that triggered this run, when any.
    pub job_id: Option<String>,
}

impl RunContext {
    /// Builds the context for a profile and the triggering job.
    pub fn for_profile(profile: &AiProfile, job_id: Option<String>) -> Self {
        let external = profile.kind == ProfileKind::OpenAiCompatible;
        Self {
            profile_id: profile.id.clone(),
            adapter: profile_adapter(profile).to_string(),
            model: if external {
                Some(profile.model.clone())
            } else {
                None
            },
            policy_json: policy_snapshot(profile),
            consent_preview_hash: profile.consent.as_ref().map(|c| c.preview_hash.clone()),
            job_id,
        }
    }

    /// Context used by tests that do not care about provenance details.
    pub fn for_tests() -> Self {
        Self::for_profile(&crate::profile::offline_default_profile(), None)
    }

    /// Fixed context for a run whose profile could not be read.
    ///
    /// Carries only literals: an unread profile must not leak a partial
    /// identity, a model name or a consent hash into the provenance row.
    pub fn unavailable(job_id: Option<String>) -> Self {
        Self {
            profile_id: "unavailable".to_string(),
            adapter: "unknown".to_string(),
            model: None,
            policy_json: "{\"profile\":\"unavailable\"}".to_string(),
            consent_preview_hash: None,
            job_id,
        }
    }
}

/// Adapter literal recorded for a profile kind.
fn profile_adapter(profile: &AiProfile) -> &'static str {
    match profile.kind {
        ProfileKind::Fake => "fake",
        ProfileKind::OpenAiCompatible => "openai-compatible",
    }
}

/// Static JSON snapshot of the profile policy; no secrets, no content.
pub fn policy_snapshot(profile: &AiProfile) -> String {
    json!({
        "kind": match profile.kind {
            ProfileKind::Fake => "fake",
            ProfileKind::OpenAiCompatible => "open_ai_compatible",
        },
        "max_input_chars": profile.max_input_chars,
        "redaction_on_ingest": true,
        "enabled": profile.external_calls_enabled,
    })
    .to_string()
}

/// Stable, order-insensitive hash of the assessed inputs.
///
/// `sha256(capture_id || sorted "artifact_id:fingerprint" lines)`. Changing any
/// artifact's content changes the hash; reordering the artifacts does not.
pub fn input_hash(capture_id: &str, evidence: &DecisionEvidence) -> String {
    let mut lines: Vec<String> = evidence
        .artifacts
        .iter()
        .map(|artifact| {
            format!(
                "{}:{}",
                artifact.artifact_id,
                artifact_fingerprint(&artifact.content)
            )
        })
        .collect();
    lines.sort();
    let joined = format!("{capture_id}\n{}", lines.join("\n"));
    artifact_fingerprint(&joined)
}

/// Result of one extraction pass.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ExtractionReport {
    /// Proposals produced by the extractor.
    pub candidates: usize,
    /// Candidates actually inserted (deduplicated by `dedup_hash`).
    pub inserted: usize,
    /// Signals that made the capture relevant.
    pub signals: Vec<RelevanceSignal>,
}

/// The extractor port: the domain never knows a provider (§12).
pub trait CandidateExtractor {
    /// Proposes candidates for a relevant capture.
    fn extract(
        &self,
        input: &DecisionEvidence,
        signals: &[RelevanceSignal],
    ) -> Result<Vec<CandidateProposal>, ExtractError>;
}

/// The persistence port extraction needs.
pub trait ExtractionStore {
    /// Loads the evidence for a capture, or `None` when it no longer exists.
    fn load_evidence(&self, capture_id: &str) -> Result<Option<DecisionEvidence>, ExtractError>;

    /// Inserts candidates, ignoring rows whose `dedup_hash` already exists.
    ///
    /// Returns the number of rows actually inserted.
    fn insert_candidates(&self, records: &[DecisionCandidateRecord])
        -> Result<usize, ExtractError>;
}

/// Deterministic fake extractor (§12): templates fixed by the signals.
#[derive(Debug, Clone, Copy, Default)]
pub struct FakeCandidateExtractor;

impl CandidateExtractor for FakeCandidateExtractor {
    fn extract(
        &self,
        input: &DecisionEvidence,
        signals: &[RelevanceSignal],
    ) -> Result<Vec<CandidateProposal>, ExtractError> {
        if signals.is_empty() {
            return Ok(Vec::new());
        }
        let strong = signals.iter().filter(|signal| signal.is_strong()).count();
        let confidence = (0.5 + 0.1 * strong as f64).min(0.95);
        let labels: Vec<&str> = signals.iter().map(RelevanceSignal::as_str).collect();
        let joined = labels.join(", ");
        let top = labels.first().copied().unwrap_or("unknown");

        Ok(vec![CandidateProposal {
            question: format!("Qual decisão durável a captura registra sobre {top}?"),
            choice: format!("Manter a escolha sinalizada por: {joined}"),
            rationale: format!(
                "Inferido deterministicamente do envelope (sinais: {joined}). Requer confirmação humana."
            ),
            confidence,
            confidence_reason: format!(
                "{strong} sinal(is) forte(s) e {} moderado(s).",
                signals.len() - strong
            ),
            signals: signals.to_vec(),
            evidence_refs: input
                .artifacts
                .iter()
                .map(|artifact| artifact.artifact_id.clone())
                .collect(),
            diff_summary: diff_summary(input),
        }])
    }
}

/// Runs the two passes and persists any candidates as `pending`.
///
/// Never creates an Engineering Decision; an empty filter result or an extractor
/// failure leaves the capture untouched. Every terminal path writes one
/// `assessments` row (MVP-SPEC §12) before returning, except a storage failure
/// while reading or writing candidates, which propagates as an error.
pub fn run_extraction<S, E>(
    store: &S,
    extractor: &E,
    capture_id: &str,
    context: &RunContext,
) -> Result<ExtractionReport, ExtractError>
where
    S: ExtractionStore + AssessmentStore,
    E: CandidateExtractor,
{
    let started_at = now_rfc3339();
    let Some(evidence) = store.load_evidence(capture_id)? else {
        // A capture that no longer exists is a storage/state error, not a
        // successful empty analysis: the caller must see a failed job.
        return Err(capture_not_found());
    };
    let evidence = normalize(evidence);
    let hash = input_hash(capture_id, &evidence);
    let signals = filter_relevant(&evidence);
    if signals.is_empty() {
        record_assessment(
            store,
            context,
            capture_id,
            &hash,
            &started_at,
            AssessmentOutcome::Empty,
            0,
            0,
            None,
        )?;
        return Ok(ExtractionReport::default());
    }

    let proposals = match extractor.extract(&evidence, &signals) {
        Ok(proposals) => proposals,
        Err(error) => {
            record_assessment(
                store,
                context,
                capture_id,
                &hash,
                &started_at,
                AssessmentOutcome::Failed,
                0,
                0,
                Some(error.code()),
            )?;
            return Err(error);
        }
    };

    // Validate the whole batch before touching the store: a malformed proposal
    // must never leave partial rows behind. Each proposal yields its canonical
    // (validated, sorted, deduplicated) signal list.
    let mut validated: Vec<(CandidateProposal, Vec<RelevanceSignal>)> = Vec::new();
    for proposal in proposals {
        match validate_proposal(&proposal, &evidence, &signals) {
            Ok(canonical) => validated.push((proposal, canonical)),
            Err(error) => {
                record_assessment(
                    store,
                    context,
                    capture_id,
                    &hash,
                    &started_at,
                    AssessmentOutcome::Failed,
                    0,
                    0,
                    Some(error.code()),
                )?;
                return Err(error);
            }
        }
    }

    let now = now_rfc3339();
    let records: Vec<DecisionCandidateRecord> = validated
        .into_iter()
        .map(|(proposal, canonical)| {
            let signal_labels: Vec<&str> = canonical.iter().map(RelevanceSignal::as_str).collect();
            let signals_json =
                serde_json::to_string(&signal_labels).unwrap_or_else(|_| "[]".to_string());
            let evidence_refs =
                serde_json::to_string(&proposal.evidence_refs).unwrap_or_else(|_| "[]".to_string());
            DecisionCandidateRecord {
                id: uuid::Uuid::now_v7().to_string(),
                project_id: evidence.project_id.clone(),
                capture_id: capture_id.to_string(),
                status: "pending".to_string(),
                question: proposal.question.clone(),
                choice: proposal.choice.clone(),
                rationale: proposal.rationale.clone(),
                signals: signals_json,
                confidence: proposal.confidence,
                confidence_reason: proposal.confidence_reason.clone(),
                evidence_refs,
                diff_summary: proposal.diff_summary.clone(),
                dedup_hash: dedup_hash(capture_id, &proposal, &canonical),
                created_at: now.clone(),
                updated_at: now.clone(),
            }
        })
        .collect();

    let inserted = store.insert_candidates(&records)?;
    let report = ExtractionReport {
        candidates: records.len(),
        inserted,
        signals,
    };
    record_assessment(
        store,
        context,
        capture_id,
        &hash,
        &started_at,
        AssessmentOutcome::Ok,
        report.candidates as i64,
        report.inserted as i64,
        None,
    )?;
    Ok(report)
}

/// Records a `skipped` assessment without calling any extractor.
///
/// Used by the composition root when [`crate::profile::choose_extractor`]
/// returns `ExternalBlocked`: the capture stays valid, no provider is contacted,
/// and the provenance row explains why nothing ran.
pub fn record_skipped_assessment<S>(
    store: &S,
    capture_id: &str,
    context: &RunContext,
) -> Result<(), ExtractError>
where
    S: ExtractionStore + AssessmentStore,
{
    let started_at = now_rfc3339();
    let Some(evidence) = store.load_evidence(capture_id)? else {
        // Same rule as `run_extraction`: a missing capture is a storage error.
        return Err(capture_not_found());
    };
    let evidence = normalize(evidence);
    let hash = input_hash(capture_id, &evidence);
    record_assessment(
        store,
        context,
        capture_id,
        &hash,
        &started_at,
        AssessmentOutcome::Skipped,
        0,
        0,
        Some(ERROR_CODE_CONSENT),
    )
}

/// Why an external provider run could not start.
///
/// Each variant maps to a short, sanitized `assessments.error_code`; the
/// decision lives here so the composition root stays thin and testable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderSetupError {
    /// The profile has consent but no stored secret.
    MissingSecret,
    /// The OS keystore could not be read.
    Keystore,
    /// The extractor could not be constructed from the profile.
    InvalidConfig,
    /// The AI profile could not be loaded or validated at all.
    ProfileUnavailable,
}

impl ProviderSetupError {
    /// Returns the stable `error_code` literal persisted for this failure.
    pub fn code(&self) -> &'static str {
        match self {
            Self::MissingSecret => ERROR_CODE_SECRET,
            Self::Keystore => ERROR_CODE_KEYSTORE,
            Self::InvalidConfig => ERROR_CODE_PROVIDER_CONFIG,
            Self::ProfileUnavailable => ERROR_CODE_PROFILE,
        }
    }
}

/// Stable `error_code` for a missing provider secret.
pub const ERROR_CODE_SECRET: &str = "secret";
/// Stable `error_code` for a keystore read failure.
pub const ERROR_CODE_KEYSTORE: &str = "keystore";
/// Stable `error_code` for an invalid provider configuration.
pub const ERROR_CODE_PROVIDER_CONFIG: &str = "provider_config";
/// Stable `error_code` for a consent-blocked (skipped) run.
pub const ERROR_CODE_CONSENT: &str = "consent";
/// Stable `error_code` for a run whose profile could not be read.
pub const ERROR_CODE_PROFILE: &str = "profile";

/// Records a terminal `failed` assessment for a provider that never started,
/// then returns the typed job failure the caller must propagate.
///
/// Used by the composition root for the setup failures that never reach
/// [`run_extraction`] (missing secret, keystore error, invalid config, unreadable
/// profile): every execution still gets provenance. `input_hash` is computed
/// exactly as in a real run; a missing capture is a storage error and writes
/// nothing. [`ProviderSetupError::ProfileUnavailable`] forces the fixed
/// [`RunContext::unavailable`] context, so an unread profile can never leak a
/// partial identity into the row.
pub fn fail_provider_setup<S>(
    store: &S,
    capture_id: &str,
    context: &RunContext,
    error: ProviderSetupError,
) -> Result<JobFailure, ExtractError>
where
    S: ExtractionStore + AssessmentStore,
{
    let started_at = now_rfc3339();
    let Some(evidence) = store.load_evidence(capture_id)? else {
        return Err(capture_not_found());
    };
    let evidence = normalize(evidence);
    let hash = input_hash(capture_id, &evidence);
    let context = if error == ProviderSetupError::ProfileUnavailable {
        RunContext::unavailable(context.job_id.clone())
    } else {
        context.clone()
    };
    record_assessment(
        store,
        &context,
        capture_id,
        &hash,
        &started_at,
        AssessmentOutcome::Failed,
        0,
        0,
        Some(error.code()),
    )?;
    Ok(JobFailure::Failed)
}

/// Fixed storage error for a capture that no longer exists; carries no content.
fn capture_not_found() -> ExtractError {
    ExtractError::Storage("captura não encontrada".to_string())
}

/// Builds and persists one assessment row with a fresh `finished_at`.
#[allow(clippy::too_many_arguments)]
fn record_assessment<S: AssessmentStore>(
    store: &S,
    context: &RunContext,
    capture_id: &str,
    input_hash: &str,
    started_at: &str,
    outcome: AssessmentOutcome,
    candidates: i64,
    inserted: i64,
    error_code: Option<&str>,
) -> Result<(), ExtractError> {
    let row = AssessmentRecord {
        id: uuid::Uuid::now_v7().to_string(),
        capture_id: capture_id.to_string(),
        job_id: context.job_id.clone(),
        profile_id: context.profile_id.clone(),
        adapter: context.adapter.clone(),
        model: context.model.clone(),
        policy: context.policy_json.clone(),
        consent_preview_hash: context.consent_preview_hash.clone(),
        input_hash: input_hash.to_string(),
        started_at: started_at.to_string(),
        finished_at: now_rfc3339(),
        outcome,
        candidates,
        inserted,
        error_code: error_code.map(str::to_string),
    };
    store.record_assessment(&row)
}

/// Validates a proposal before anything is persisted and returns its canonical
/// signal list.
///
/// Failure rejects the whole batch; messages are fixed and carry no content.
fn validate_proposal(
    proposal: &CandidateProposal,
    evidence: &DecisionEvidence,
    detected: &[RelevanceSignal],
) -> Result<Vec<RelevanceSignal>, ExtractError> {
    if proposal.question.trim().is_empty() {
        return Err(ExtractError::Validation(
            "proposta sem pergunta".to_string(),
        ));
    }
    if proposal.choice.trim().is_empty() {
        return Err(ExtractError::Validation("proposta sem escolha".to_string()));
    }
    if proposal.rationale.trim().is_empty() {
        return Err(ExtractError::Validation(
            "proposta sem justificativa".to_string(),
        ));
    }
    if proposal.confidence_reason.trim().is_empty() {
        return Err(ExtractError::Validation(
            "proposta sem explicacao de confianca".to_string(),
        ));
    }
    if !proposal.confidence.is_finite() || !(0.0..=1.0).contains(&proposal.confidence) {
        return Err(ExtractError::Validation(
            "confianca fora do intervalo".to_string(),
        ));
    }
    if proposal.signals.is_empty() {
        return Err(ExtractError::Validation("proposta sem sinais".to_string()));
    }
    for signal in &proposal.signals {
        if !detected.iter().any(|known| known == signal) {
            return Err(ExtractError::Validation(
                "proposta com sinal nao detectado".to_string(),
            ));
        }
    }
    if proposal.evidence_refs.is_empty() {
        return Err(ExtractError::Validation(
            "proposta sem referencias de evidencia".to_string(),
        ));
    }
    for reference in &proposal.evidence_refs {
        if !evidence
            .artifacts
            .iter()
            .any(|artifact| &artifact.artifact_id == reference)
        {
            return Err(ExtractError::Validation(
                "proposta com referencia de evidencia desconhecida".to_string(),
            ));
        }
    }
    validate_diff_summary(&proposal.diff_summary, evidence.artifacts.len())?;

    let mut canonical = proposal.signals.clone();
    canonical.sort_by(|left, right| left.as_str().cmp(right.as_str()));
    canonical.dedup();
    Ok(canonical)
}

/// Validates the `{files: [...], artifacts: n}` shape of a diff summary.
fn validate_diff_summary(summary: &str, artifact_count: usize) -> Result<(), ExtractError> {
    let value: Value = serde_json::from_str(summary)
        .map_err(|_| ExtractError::Validation("diff_summary invalido".to_string()))?;
    let object = value
        .as_object()
        .ok_or_else(|| ExtractError::Validation("diff_summary nao e objeto".to_string()))?;
    let files = object
        .get("files")
        .and_then(Value::as_array)
        .ok_or_else(|| ExtractError::Validation("diff_summary sem files".to_string()))?;
    if files.len() > MAX_DIFF_SUMMARY_FILES {
        return Err(ExtractError::Validation(
            "diff_summary com arquivos demais".to_string(),
        ));
    }
    for file in files {
        if file
            .as_str()
            .map(|text| text.trim().is_empty())
            .unwrap_or(true)
        {
            return Err(ExtractError::Validation(
                "diff_summary com arquivo invalido".to_string(),
            ));
        }
    }
    let artifacts = object
        .get("artifacts")
        .and_then(Value::as_u64)
        .ok_or_else(|| ExtractError::Validation("diff_summary sem artifacts".to_string()))?;
    if artifacts as usize != artifact_count {
        return Err(ExtractError::Validation(
            "diff_summary com contagem incoerente".to_string(),
        ));
    }
    Ok(())
}

/// Sorts artifacts deterministically and applies the defensive bounds.
fn normalize(mut evidence: DecisionEvidence) -> DecisionEvidence {
    evidence
        .artifacts
        .sort_by(|left, right| left.artifact_id.cmp(&right.artifact_id));
    evidence.artifacts.truncate(MAX_EVIDENCE_ARTIFACTS);
    for artifact in &mut evidence.artifacts {
        artifact.content = truncate_content(&artifact.content, MAX_EVIDENCE_CONTENT_BYTES);
    }
    evidence
}

/// Truncates `value` to at most `max_bytes`, respecting char boundaries.
pub fn truncate_content(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_string();
    }
    let mut end = max_bytes;
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_string()
}

/// Canonical deduplication hash: `capture_id|question|choice|signals`.
///
/// `signals` is the already-canonical (sorted, deduplicated) list used for the
/// `signals` column, so the hash is stable regardless of the extractor's order.
fn dedup_hash(
    capture_id: &str,
    proposal: &CandidateProposal,
    signals: &[RelevanceSignal],
) -> String {
    let labels: Vec<&str> = signals.iter().map(RelevanceSignal::as_str).collect();
    artifact_fingerprint(&format!(
        "{}|{}|{}|{}",
        capture_id,
        proposal.question,
        proposal.choice,
        labels.join(",")
    ))
}

/// Returns `true` when the capture is one of the §6 exclusions.
///
/// A real structural strong signal in a production diff (DDL, a new dependency,
/// a security-relevant code line) takes precedence: the trivial wording is then
/// incidental and must not discard the capture. Otherwise a capture with no diff
/// evidence, or one whose only diff is trivial, is vetoed.
fn is_trivially_excluded(evidence: &DecisionEvidence, text: &str) -> bool {
    if has_structural_strong(evidence) {
        return false;
    }
    // Without a `diff_hunk` there is no change to assess; free prose that merely
    // mentions "CREATE TABLE" or "token" is not durable evidence.
    if !has_diff_hunk(evidence) {
        return true;
    }
    if TRIVIAL_MARKERS.iter().any(|marker| text.contains(marker)) {
        return true;
    }
    let files = diff_files(evidence);
    if !files.is_empty() && files.iter().all(|path| is_test_path(path)) {
        return true;
    }
    is_comment_only_diff(evidence)
}

/// Returns `true` when the evidence carries at least one `diff_hunk` artifact.
fn has_diff_hunk(evidence: &DecisionEvidence) -> bool {
    evidence
        .artifacts
        .iter()
        .any(|artifact| artifact.kind == "diff_hunk")
}

/// Returns `true` when a production `diff_hunk` carries a strong structural
/// signal.
///
/// Only `diff_hunk` artifacts are examined, only their added non-comment lines
/// count, and a `.sql` path alone is never enough: a real DDL statement must be
/// present. `INSERT`/`UPDATE` are data changes, not structural, and do not
/// count. This keeps an incidental word or a commented-out DDL from defeating
/// the trivial veto.
fn has_structural_strong(evidence: &DecisionEvidence) -> bool {
    for artifact in &evidence.artifacts {
        if artifact.kind != "diff_hunk" {
            continue;
        }
        if artifact_has_dependency_addition(artifact) {
            return true;
        }
        let mut current_is_test = false;
        for line in artifact.content.lines() {
            if let Some(rest) = line.strip_prefix("diff --git ") {
                let token = rest.split_whitespace().next().unwrap_or("");
                let path = token.strip_prefix("a/").unwrap_or(token);
                current_is_test = is_test_path(path);
                continue;
            }
            let trimmed = line.trim_start();
            let Some(rest) = trimmed.strip_prefix('+') else {
                continue;
            };
            if current_is_test || rest.starts_with("++") {
                continue;
            }
            let content = rest.trim();
            if content.is_empty() || is_comment_prefix(content) {
                continue;
            }
            let lower = content.to_ascii_lowercase();
            if contains_any_phrase(&lower, STRUCTURAL_DDL_PHRASES) {
                return true;
            }
            if contains_any_token(&lower, SECURITY_PRIVACY_TOKENS) {
                return true;
            }
        }
    }
    false
}

/// Returns `true` when the artifact adds a dependency in a manifest.
fn artifact_has_dependency_addition(artifact: &EvidenceArtifact) -> bool {
    let mut current_manifest = false;
    let mut current_is_test = false;
    for line in artifact.content.lines() {
        if let Some(rest) = line.strip_prefix("diff --git ") {
            let token = rest.split_whitespace().next().unwrap_or("");
            let path = token.strip_prefix("a/").unwrap_or(token);
            let lower = path.to_ascii_lowercase();
            current_is_test = is_test_path(path);
            current_manifest = lower.ends_with("cargo.toml") || lower.ends_with("package.json");
            continue;
        }
        if !current_manifest || current_is_test {
            continue;
        }
        let trimmed = line.trim_start();
        let Some(rest) = trimmed.strip_prefix('+') else {
            continue;
        };
        if rest.starts_with("++") {
            continue;
        }
        let addition = rest.trim();
        // Commented-out manifest entries are not dependency additions.
        if addition.is_empty() || is_comment_prefix(addition) {
            continue;
        }
        if addition.contains('=') || addition.contains(':') {
            return true;
        }
    }
    false
}

/// Returns `true` for a path inside a test location.
fn is_test_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    let file = lower.rsplit('/').next().unwrap_or(&lower);
    lower
        .split('/')
        .any(|segment| matches!(segment, "tests" | "test" | "__tests__"))
        || file.contains("_test.")
        || file.contains(".test.")
        || file.contains(".spec.")
}

/// Returns `true` when every added/removed diff line is a comment.
fn is_comment_only_diff(evidence: &DecisionEvidence) -> bool {
    let mut saw_comment = false;
    for artifact in &evidence.artifacts {
        for line in artifact.content.lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("+++") || trimmed.starts_with("---") {
                continue;
            }
            let Some(rest) = trimmed
                .strip_prefix('+')
                .or_else(|| trimmed.strip_prefix('-'))
            else {
                continue;
            };
            let content = rest.trim_start();
            if content.is_empty() {
                continue;
            }
            if is_comment_prefix(content) {
                saw_comment = true;
            } else {
                return false;
            }
        }
    }
    saw_comment
}

/// Returns `true` for the common comment prefixes.
fn is_comment_prefix(content: &str) -> bool {
    content.starts_with("//")
        || content.starts_with('#')
        || content.starts_with("/*")
        || content.starts_with('*')
        || content.starts_with("<!--")
        || content.starts_with("--")
}

/// Returns the relevance signals that pass the §6 filter, or an empty list.
pub fn filter_relevant(evidence: &DecisionEvidence) -> Vec<RelevanceSignal> {
    let text = aggregate_text(evidence);

    // Veto rules run before any positive signal: a trivial/test-only change must
    // not become relevant just because it mentions "schema", "api" or "token".
    if is_trivially_excluded(evidence, &text) {
        return Vec::new();
    }

    let mut detected = BTreeSet::new();

    if contains_any_token(&text, PUBLIC_CONTRACT_TOKENS) {
        detected.insert(RelevanceSignal::PublicContract);
    }
    if contains_any_token(&text, SECURITY_PRIVACY_TOKENS) {
        detected.insert(RelevanceSignal::SecurityPrivacy);
    }
    if has_dependency_addition(&text) {
        detected.insert(RelevanceSignal::DependencyAdded);
    }
    if top_level_dirs(evidence).len() >= 2 {
        detected.insert(RelevanceSignal::CrossesBoundaries);
    }
    if contains_any_phrase(&text, HARD_TO_REVERT) {
        detected.insert(RelevanceSignal::HardToRevert);
    }
    if contains_any_phrase(&text, REJECTS_ALTERNATIVE) {
        detected.insert(RelevanceSignal::RejectsAlternative);
    }
    if contains_any_phrase(&text, CONDITIONS_FUTURE_WORK) {
        detected.insert(RelevanceSignal::ConditionsFutureWork);
    }
    if contains_any_phrase(&text, MATERIAL_BLAST_RADIUS) {
        detected.insert(RelevanceSignal::MaterialBlastRadius);
    }
    if contains_any_phrase(&text, ALTERNATIVES_COMPARED) {
        detected.insert(RelevanceSignal::AlternativesCompared);
    }
    if contains_any_phrase(&text, EXPLICIT_TRADEOFF) {
        detected.insert(RelevanceSignal::ExplicitTradeoff);
    }
    if contains_any_phrase(&text, DISAGREEMENT_UNCERTAINTY) {
        detected.insert(RelevanceSignal::DisagreementUncertainty);
    }
    if contains_any_phrase(&text, RELEVANT_COST) {
        detected.insert(RelevanceSignal::RelevantCost);
    }
    if contains_any_phrase(&text, VALIDITY_MONTHS) {
        detected.insert(RelevanceSignal::ValidityMonths);
    }
    if contains_any_phrase(&text, MAINTENANCE_ONBOARDING) {
        detected.insert(RelevanceSignal::MaintenanceOnboarding);
    }
    if contains_any_phrase(&text, DELEGATED_TO_AGENT) {
        detected.insert(RelevanceSignal::DelegatedToAgent);
    }
    if contains_any_phrase(&text, UNPROVEN_ASSUMPTION) {
        detected.insert(RelevanceSignal::UnprovenAssumption);
    }

    let strong = detected.iter().filter(|signal| signal.is_strong()).count();
    let moderate = detected.len() - strong;
    if strong == 0 && moderate < 2 {
        return Vec::new();
    }
    detected.into_iter().collect()
}

/// Lowercased concatenation of kinds, contents and metadata, in artifact order.
fn aggregate_text(evidence: &DecisionEvidence) -> String {
    let mut text = String::new();
    for artifact in &evidence.artifacts {
        text.push_str(&artifact.kind.to_lowercase());
        text.push('\n');
        text.push_str(&artifact.content.to_lowercase());
        text.push('\n');
        text.push_str(&artifact.metadata.to_lowercase());
        text.push('\n');
    }
    text
}

/// JSON summary of the diff-shaped artifacts, never their raw content.
fn diff_summary(evidence: &DecisionEvidence) -> String {
    let files: Vec<String> = diff_files(evidence).into_iter().collect();
    json!({ "files": files, "artifacts": evidence.artifacts.len() }).to_string()
}

/// Sorted unique file paths found in `diff --git` headers.
fn diff_files(evidence: &DecisionEvidence) -> BTreeSet<String> {
    let mut files = BTreeSet::new();
    for artifact in &evidence.artifacts {
        for line in artifact.content.lines() {
            let Some(rest) = line.strip_prefix("diff --git ") else {
                continue;
            };
            for token in rest.split_whitespace() {
                let path = token
                    .strip_prefix("a/")
                    .or_else(|| token.strip_prefix("b/"))
                    .unwrap_or(token);
                if !path.is_empty() {
                    files.insert(path.to_string());
                }
            }
        }
    }
    files
}

/// Top-level path segments present in diff headers.
fn top_level_dirs(evidence: &DecisionEvidence) -> BTreeSet<String> {
    let mut dirs = BTreeSet::new();
    for file in diff_files(evidence) {
        if let Some(segment) = file.split('/').next() {
            if !segment.is_empty() {
                dirs.insert(segment.to_string());
            }
        }
    }
    dirs
}

/// Returns `true` when a manifest diff adds a dependency line.
fn has_dependency_addition(text: &str) -> bool {
    if !(text.contains("cargo.toml") || text.contains("package.json")) {
        return false;
    }
    text.lines().any(|line| {
        let trimmed = line.trim_start();
        match trimmed.strip_prefix('+') {
            Some(rest) if rest.starts_with("++") => false,
            Some(addition) => {
                let addition = addition.trim();
                // Commented-out manifest entries are not dependency additions.
                !addition.is_empty()
                    && !is_comment_prefix(addition)
                    && (addition.contains('=') || addition.contains(':'))
            }
            None => false,
        }
    })
}

/// Case-insensitive whole-word match for ASCII tokens.
fn contains_token(text: &str, token: &str) -> bool {
    let bytes = text.as_bytes();
    let mut start = 0;
    while let Some(position) = text[start..].find(token) {
        let index = start + position;
        let before_ok = index == 0 || !is_word_byte(bytes[index - 1]);
        let end = index + token.len();
        let after_ok = end >= bytes.len() || !is_word_byte(bytes[end]);
        if before_ok && after_ok {
            return true;
        }
        start = index + 1;
    }
    false
}

fn contains_any_token(text: &str, tokens: &[&str]) -> bool {
    tokens.iter().any(|token| contains_token(text, token))
}

fn contains_any_phrase(text: &str, phrases: &[&str]) -> bool {
    phrases.iter().any(|phrase| text.contains(phrase))
}

fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

const TRIVIAL_MARKERS: &[&str] = &[
    "formatting",
    "format only",
    "reformat",
    "whitespace",
    "lint",
    "renamed",
    "renaming",
    "rename",
    "comment only",
    "comment-only",
    "no behavior change",
    "no behavioural change",
    "sem mudanca de comportamento",
    "sem mudança de comportamento",
];

/// DDL/migration phrases counted only on a non-comment `+` line of a
/// production `diff_hunk`; a `.sql` path alone is never enough.
const STRUCTURAL_DDL_PHRASES: &[&str] = &[
    "create table",
    "alter table",
    "add column",
    "drop table",
    "create index",
    "migration",
    "endpoint",
];

const PUBLIC_CONTRACT_TOKENS: &[&str] = &[
    "migration",
    "migrations",
    "migracao",
    "schema",
    "endpoint",
    "api",
    "table",
    "tabela",
    "ddl",
    "contract",
    "contrato",
    "persistence",
    "persistencia",
];

const SECURITY_PRIVACY_TOKENS: &[&str] = &[
    "security",
    "seguranca",
    "auth",
    "token",
    "credential",
    "secret",
    "privacy",
    "privacidade",
    "permission",
    "permissao",
    "compliance",
    "encrypt",
    "encryption",
    "lgpd",
    "gdpr",
];

const HARD_TO_REVERT: &[&str] = &[
    "hard to reverse",
    "hard to revert",
    "irreversible",
    "irreversivel",
    "breaking change",
    "once deployed",
    "destructive",
    "drop table",
];

const REJECTS_ALTERNATIVE: &[&str] = &[
    "rather than",
    "instead of",
    "em vez de",
    "rejected the",
    "rejeitamos",
    "optamos por",
    "chose not",
];

const CONDITIONS_FUTURE_WORK: &[&str] = &[
    "future",
    "futuro",
    "follow-up",
    "follow up",
    "next step",
    "will allow",
    "prepares",
];

const MATERIAL_BLAST_RADIUS: &[&str] = &[
    "all users",
    "every request",
    "global",
    "todos os",
    "shared across",
    "core path",
    "blast radius",
];

const ALTERNATIVES_COMPARED: &[&str] = &[
    "alternative",
    "alternativa",
    "options",
    "option",
    "candidate",
];

const EXPLICIT_TRADEOFF: &[&str] = &[
    "trade-off",
    "tradeoff",
    "trade off",
    "versus",
    " vs ",
    "downside",
    "compromise",
];

const DISAGREEMENT_UNCERTAINTY: &[&str] = &[
    "unclear",
    "uncertain",
    "not sure",
    "talvez",
    "incerto",
    "ambiguous",
];

const RELEVANT_COST: &[&str] = &[
    "cost",
    "custo",
    "latency",
    "latencia",
    "overhead",
    "budget",
    "throughput",
];

const VALIDITY_MONTHS: &[&str] = &[
    "long-term",
    "long term",
    "months",
    "years",
    "durable",
    "durave",
    "permanent",
];

const MAINTENANCE_ONBOARDING: &[&str] = &[
    "maintain",
    "manutencao",
    "onboarding",
    "readability",
    "documentation",
];

const DELEGATED_TO_AGENT: &[&str] = &["agent", "agente", "llm", "copilot"];

const UNPROVEN_ASSUMPTION: &[&str] = &[
    "assume",
    "assumption",
    "premissa",
    "presume",
    "without evidence",
    "unverified",
];

/// Fixed identifier of the synthetic connection-test capture.
pub const CONNECTION_TEST_CAPTURE_ID: &str = "connection-test";

/// Result of [`run_connection_test`]: counts only, never model text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnectionTestReport {
    /// Proposals the provider returned; every one passed validation.
    pub proposals: usize,
}

/// Builds the synthetic evidence used by the provider connection test.
///
/// The content is a fixed, fictitious decision written for this purpose: it
/// never contains project data, so a test call sends nothing from the user's
/// captures (MVP-SPEC §13). It is shaped like a real turn — user text plus a
/// diff touching two components — so the relevance filter detects signals and
/// the provider exercises the same structured contract as a real run.
pub fn connection_test_evidence() -> DecisionEvidence {
    let artifact = |id: &str, kind: &str, content: &str| EvidenceArtifact {
        artifact_id: id.to_string(),
        kind: kind.to_string(),
        content: content.to_string(),
        metadata: "{}".to_string(),
    };
    DecisionEvidence {
        capture_id: CONNECTION_TEST_CAPTURE_ID.to_string(),
        project_id: CONNECTION_TEST_CAPTURE_ID.to_string(),
        adapter: None,
        session_id: None,
        observed_at: None,
        artifacts: vec![
            artifact(
                "00000000-0000-7000-8000-000000000001",
                "user_text",
                "Teste sintético de conexão: vamos persistir a fila de exemplo em SQLite \
                 em vez de Postgres, porque o aplicativo é local e sem servidor; o \
                 trade-off é abrir mão de concorrência entre máquinas.",
            ),
            artifact(
                "00000000-0000-7000-8000-000000000002",
                "diff_hunk",
                "diff --git a/storage/schema.sql b/storage/schema.sql\n\
                 +CREATE TABLE exemplo (id TEXT PRIMARY KEY);\n\
                 diff --git a/api/contrato.rs b/api/contrato.rs\n\
                 +pub struct Exemplo { pub id: String }",
            ),
        ],
    }
}

/// Runs the provider connection test ("teste com resposta estruturada", §8).
///
/// Sends [`connection_test_evidence`] through `extractor` and validates every
/// returned proposal with the same rules a real extraction applies. Nothing is
/// persisted: no assessment, candidate or decision is written. Consent is the
/// extractor's responsibility (the real provider refuses without it).
///
/// # Errors
///
/// The extractor's own error when the call fails, or
/// [`ExtractError::Validation`] when a proposal breaks the contract.
pub fn run_connection_test<E: CandidateExtractor>(
    extractor: &E,
) -> Result<ConnectionTestReport, ExtractError> {
    let evidence = normalize(connection_test_evidence());
    let signals = filter_relevant(&evidence);
    let proposals = extractor.extract(&evidence, &signals)?;
    for proposal in &proposals {
        validate_proposal(proposal, &evidence, &signals)?;
    }
    Ok(ConnectionTestReport {
        proposals: proposals.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::{
        filter_relevant, truncate_content, DecisionEvidence, EvidenceArtifact, RelevanceSignal,
    };

    fn artifact(id: &str, kind: &str, content: &str) -> EvidenceArtifact {
        EvidenceArtifact {
            artifact_id: id.to_string(),
            kind: kind.to_string(),
            content: content.to_string(),
            metadata: "{}".to_string(),
        }
    }

    fn evidence(artifacts: Vec<EvidenceArtifact>) -> DecisionEvidence {
        DecisionEvidence {
            capture_id: "capture-1".to_string(),
            project_id: "project-1".to_string(),
            adapter: Some("opencode".to_string()),
            session_id: Some("session-1".to_string()),
            observed_at: Some("2026-01-01T00:00:00Z".to_string()),
            artifacts,
        }
    }

    #[test]
    fn truncate_content_respects_char_boundaries() {
        assert_eq!(truncate_content("abc", 10), "abc");
        assert_eq!(truncate_content("abcdef", 3), "abc");
        assert_eq!(truncate_content("a\u{e9}b", 2), "a");
    }

    /// A neutral production diff, so prose-only moderation is not vetoed by the
    /// "no change to assess" rule.
    fn neutral_diff() -> EvidenceArtifact {
        artifact(
            "diff",
            "diff_hunk",
            "diff --git a/src/lib.rs b/src/lib.rs\n+ let value = 1;\n",
        )
    }

    #[test]
    fn a_strong_signal_alone_filters() {
        let signals = filter_relevant(&evidence(vec![artifact(
            "a1",
            "diff_hunk",
            "diff --git a/src/schema.rs b/src/schema.rs\n+CREATE TABLE t (id TEXT);\n",
        )]));
        assert!(signals.contains(&RelevanceSignal::PublicContract));
    }

    #[test]
    fn two_moderate_signals_filter_but_one_does_not() {
        let one = filter_relevant(&evidence(vec![
            neutral_diff(),
            artifact("a1", "user_text", "We compared an option for the helper."),
        ]));
        assert!(one.is_empty(), "one moderate signal must not filter");

        let two = filter_relevant(&evidence(vec![
            neutral_diff(),
            artifact(
                "a1",
                "user_text",
                "We compared an option and the trade-off is the extra cost.",
            ),
        ]));
        assert!(two.contains(&RelevanceSignal::AlternativesCompared));
        assert!(two.contains(&RelevanceSignal::ExplicitTradeoff));
    }

    #[test]
    fn a_trivial_change_does_not_filter() {
        let signals = filter_relevant(&evidence(vec![artifact(
            "a1",
            "diff_hunk",
            "diff --git a/src/lib.rs b/src/lib.rs\n+ let formatted = value;\n",
        )]));
        assert!(signals.is_empty());
    }
}
