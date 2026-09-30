//! Provenance of every extraction run (`assessments` rows, MVP-SPEC §12).

use integration_contracts::capture::artifact_fingerprint;
use serde_json::json;

use super::{normalize, DecisionEvidence, ExtractError, ExtractionStore};
use crate::clock::now_rfc3339;
use crate::jobs::JobFailure;
use crate::profile::{AiProfile, ProfileKind};

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
        ProfileKind::ChatGptPlan => "chatgpt-plan",
        ProfileKind::OpenCode => "opencode",
    }
}

/// Static JSON snapshot of the profile policy; no secrets, no content.
pub fn policy_snapshot(profile: &AiProfile) -> String {
    json!({
        "kind": match profile.kind {
            ProfileKind::Fake => "fake",
            ProfileKind::OpenAiCompatible => "open_ai_compatible",
            ProfileKind::ChatGptPlan => "chat_gpt_plan",
            ProfileKind::OpenCode => "open_code",
        },
        "max_input_chars": profile.max_input_chars,
        "redaction_on_ingest": true,
        "enabled": profile.external_calls_enabled,
    })
    .to_string()
}

/// Stable, order-insensitive hash of the assessed inputs.
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

/// Records a `skipped` assessment without calling any extractor.
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
pub(super) fn capture_not_found() -> ExtractError {
    ExtractError::Storage("captura não encontrada".to_string())
}

/// Builds and persists one assessment row with a fresh `finished_at`.
#[allow(clippy::too_many_arguments)]
pub(super) fn record_assessment<S: AssessmentStore>(
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
