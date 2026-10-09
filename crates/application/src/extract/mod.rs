//! Extraction of Decision Candidates from a persisted capture.

mod assessment;
mod connection_test;
mod fake;
mod relevance;
mod validation;

use integration_contracts::capture::artifact_fingerprint;

use crate::clock::now_rfc3339;

pub use assessment::{
    fail_provider_setup, failure_detail, input_hash, policy_snapshot, record_skipped_assessment,
    AssessmentOutcome, AssessmentRecord, AssessmentStore, ComponentTally, ProviderSetupError,
    RunContext, ERROR_CODE_CONSENT, ERROR_CODE_KEYSTORE, ERROR_CODE_PROFILE,
    ERROR_CODE_PROVIDER_CONFIG, ERROR_CODE_SECRET, FAILURE_DETAIL_MAX_CHARS,
};
pub use connection_test::{
    connection_test_evidence, run_connection_test, ConnectionTestReport, CONNECTION_TEST_CAPTURE_ID,
};
pub use fake::FakeCandidateExtractor;
pub use relevance::{filter_relevant, RelevanceSignal};

use assessment::{capture_not_found, record_assessment};
use validation::validate_proposal;

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

/// A candidate proposed by an extractor, before persistence.
#[derive(Debug, Clone, PartialEq)]
pub struct CandidateProposal {
    /// Optional model classification; legacy omission is Unknown.
    pub nature: crate::review_exception::CandidateNature,
    /// Explicit qualifications with literal artifact support.
    pub qualifiers: Vec<crate::qualifiers::KnowledgeQualifier>,
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
    /// Decision, project rule or implementation detail (details are dropped).
    pub kind: CandidateKind,
    /// How much it matters for the project, in `[0.0, 1.0]`.
    pub significance: f64,
    /// Significance criteria the extractor ticked ([`SIGNIFICANCE_CRITERIA`]).
    pub criteria: Vec<String>,
    /// Map components the extractor says it applies to, each with a quote
    /// from its own text. Empty when the project has no map or it is unsure.
    pub components: Vec<ProposedComponent>,
}

/// A component of the project map as the extractor named it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposedComponent {
    /// The component's name, as it was listed to the extractor.
    pub name: String,
    /// A quote copied from the question, choice or rationale.
    pub quote: String,
}

/// A live component of the project map, listed to the extractor by name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MapComponent {
    /// The entity id.
    pub entity_id: String,
    /// The name the extractor writes back.
    pub name: String,
    /// The keys of the name and of its aliases (`entity_key`): a written name
    /// matches any.
    pub keys: Vec<String>,
    /// What the component does, short, for the extractor to recognize it by
    /// something other than its name. Empty when the map has none.
    pub description: String,
}

/// A component a candidate applies to, checked against the map and against
/// the candidate's own text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateComponent {
    /// The entity id.
    pub entity_id: String,
    /// The quote that shows the candidate says so.
    pub quote: String,
}

/// What a proposal is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CandidateKind {
    /// A durable engineering decision.
    #[default]
    Decision,
    /// A rule the project (or one component) must follow; becomes a claim.
    Rule,
    /// How something was implemented; never persisted.
    Detail,
}

impl CandidateKind {
    /// Persisted literal.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Decision => "decision",
            Self::Rule => "rule",
            Self::Detail => "detail",
        }
    }

    /// Parses a literal; unknown values read as a decision.
    pub fn parse(value: &str) -> Self {
        match value {
            "rule" => Self::Rule,
            "detail" => Self::Detail,
            _ => Self::Decision,
        }
    }
}

/// The significance test, adapted from Olaf Zimmermann's architectural
/// significance criteria: what makes a choice worth remembering.
pub const SIGNIFICANCE_CRITERIA: &[&str] = &[
    "cross_cutting",
    "data_or_contract",
    "security_or_privacy",
    "external_dependency",
    "hard_to_reverse",
    "first_of_a_kind",
    "past_problem",
    "constrains_future_work",
];

/// Candidates below this significance stay out of the review queue by
/// default (they are kept, in a collapsed "low relevance" group).
pub const MIN_SIGNIFICANCE: f64 = 0.5;

/// What the project already knows and what the user accepted or rejected
/// before, sent with the capture so the extractor skips repeats and learns
/// the user's taste (bounded, never raw captures).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExtractionBackground {
    /// Decisions in force and rules already recorded, as `question → choice`.
    pub known: Vec<String>,
    /// Candidates the user confirmed recently, as `question → choice`.
    pub confirmed: Vec<String>,
    /// Candidates the user rejected recently, as `question → choice`.
    pub rejected: Vec<String>,
    /// The live components of the project map, so the extractor can say which
    /// ones a candidate applies to (see [`resolve_components`]).
    pub components: Vec<MapComponent>,
}

/// A row to persist in `decision_candidates`.
#[derive(Debug, Clone, PartialEq)]
pub struct DecisionCandidateRecord {
    /// Serialized explicit qualification array.
    pub qualifiers: String,
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
    /// `decision` or `rule`.
    pub kind: String,
    /// Significance in `[0.0, 1.0]`.
    pub significance: f64,
    /// Serialized significance criteria array.
    pub criteria: String,
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
    /// The provider asked to slow down (HTTP 429); the job is requeued.
    RateLimited {
        /// Wait the provider asked for (`Retry-After`), if it said.
        retry_after: Option<std::time::Duration>,
    },
    /// The provider timed out or failed transiently on every attempt of
    /// this call; the job is requeued with backoff, never dropped.
    Unavailable {
        /// Attempts made within this call.
        attempts: u32,
    },
}

impl std::fmt::Display for ExtractError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Storage(message) => write!(formatter, "falha de armazenamento: {message}"),
            Self::Extractor(message) => write!(formatter, "falha do extrator: {message}"),
            Self::Validation(message) => write!(formatter, "proposta inválida: {message}"),
            Self::RateLimited { .. } => {
                formatter.write_str("o provedor de IA pediu uma pausa; a análise volta para a fila")
            }
            Self::Unavailable { attempts } => write!(
                formatter,
                "o provedor de IA não respondeu após {attempts} tentativa(s); \
                 a análise volta para a fila"
            ),
        }
    }
}

impl std::error::Error for ExtractError {}

impl ExtractError {
    /// Returns a short, stable code for provenance (`assessments.error_code`).
    pub fn code(&self) -> &'static str {
        match self {
            Self::Storage(_) => "storage",
            Self::Extractor(_) => "extractor",
            Self::Validation(_) => "validation",
            Self::RateLimited { .. } => "rate_limited",
            Self::Unavailable { .. } => "unavailable",
        }
    }

    /// Whether the job should go back to the queue instead of failing.
    pub fn is_deferred(&self) -> bool {
        matches!(self, Self::RateLimited { .. } | Self::Unavailable { .. })
    }

    /// The job outcome for this error: requeue for provider pauses and
    /// timeouts, failure otherwise.
    pub fn job_failure(&self) -> crate::jobs::JobFailure {
        match self {
            Self::RateLimited { retry_after } => crate::jobs::JobFailure::Deferred {
                retry_after: *retry_after,
            },
            Self::Unavailable { .. } => crate::jobs::JobFailure::Deferred { retry_after: None },
            _ => crate::jobs::JobFailure::Failed,
        }
    }
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
    /// Extraction with live authorization; network adapters recheck each attempt.
    fn extract_authorized(
        &self,
        input: &DecisionEvidence,
        signals: &[RelevanceSignal],
        background: &ExtractionBackground,
        authorization: &dyn crate::external::Authorization,
    ) -> Result<Vec<CandidateProposal>, ExtractError> {
        authorization.check()?;
        self.extract_with(input, signals, background)
    }
    /// Proposes candidates for a relevant capture.
    fn extract(
        &self,
        input: &DecisionEvidence,
        signals: &[RelevanceSignal],
    ) -> Result<Vec<CandidateProposal>, ExtractError>;
    /// Extracts with what the project already knows; extractors that can use
    /// it override this, the others ignore it.
    fn extract_with(
        &self,
        input: &DecisionEvidence,
        signals: &[RelevanceSignal],
        background: &ExtractionBackground,
    ) -> Result<Vec<CandidateProposal>, ExtractError> {
        let _ = background;
        self.extract(input, signals)
    }
}

/// The persistence port extraction needs.
pub trait ExtractionStore {
    /// Persists classification alongside retained candidate evidence.
    fn record_nature(
        &self,
        _dedup_hash: &str,
        _nature: crate::review_exception::CandidateNature,
    ) -> Result<(), ExtractError> {
        Ok(())
    }
    /// Persists the components a candidate applies to, found by its
    /// `dedup_hash`. Stores without the table ignore them.
    fn record_components(
        &self,
        _dedup_hash: &str,
        _components: &[CandidateComponent],
    ) -> Result<(), ExtractError> {
        Ok(())
    }
    /// Loads the evidence for a capture, or `None` when it no longer exists.
    fn load_evidence(&self, capture_id: &str) -> Result<Option<DecisionEvidence>, ExtractError>;

    /// What the project already records about `files` and the user's recent
    /// confirmations and rejections. Empty by default.
    fn background(
        &self,
        _project_id: &str,
        _files: &[String],
    ) -> Result<ExtractionBackground, ExtractError> {
        Ok(ExtractionBackground::default())
    }

    /// Every entity of the project's map, retired ones included; the use case
    /// picks the components to list to the extractor. Empty by default.
    fn map_entities(
        &self,
        _project_id: &str,
    ) -> Result<Vec<crate::graph::EntityRecord>, ExtractError> {
        Ok(Vec::new())
    }

    /// Inserts candidates, ignoring rows whose `dedup_hash` already exists.
    fn insert_candidates(&self, records: &[DecisionCandidateRecord])
        -> Result<usize, ExtractError>;
}

/// Runs the two passes and persists any candidates as `pending`.
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
    let mut run_context = context.clone();
    if run_context.attempt.is_none() {
        run_context.attempt = store.assessment_attempt(context.job_id.as_deref())?;
    }
    let context = &run_context;
    let Some(evidence) = store.load_evidence(capture_id)? else {
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
            None,
            None,
        )?;
        return Ok(ExtractionReport::default());
    }

    // Background is a hint: a failure to read it never blocks extraction.
    let mut background = store
        .background(&evidence.project_id, &relevance::diff_file_list(&evidence))
        .unwrap_or_default();
    // The map's components are a hint too: without them the extractor simply
    // names none. (This crate has no logging; the failure leaves the list
    // empty, like a failed background.)
    background.components = store
        .map_entities(&evidence.project_id)
        .map(|entities| {
            crate::link_suggestions::candidate_components(&entities)
                .into_iter()
                .map(|entity| MapComponent {
                    entity_id: entity.entity_id.clone(),
                    name: entity.name.clone(),
                    keys: entity.keys().collect(),
                    description: short_description(entity),
                })
                .collect()
        })
        .unwrap_or_default();
    let proposals = match extractor.extract_with(&evidence, &signals, &background) {
        Ok(proposals) => proposals,
        // A deferred call is retried by the job; it is not a lost analysis.
        Err(error) if error.is_deferred() => return Err(error),
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
                Some(failure_detail(&error)),
                None,
            )?;
            return Err(error);
        }
    };

    // The app owns the facts about the capture: the model's file list,
    // artifact count and references are reconciled with the evidence, and a
    // proposal that still fails validation is dropped on its own instead of
    // taking the whole batch down.
    let mut validated: Vec<(CandidateProposal, Vec<RelevanceSignal>)> = Vec::new();
    let detail_count = proposals
        .iter()
        .filter(|p| p.kind == CandidateKind::Detail)
        .count();
    let durable_count = proposals.len() - detail_count;
    let mut classified_context = context.clone();
    classified_context.classification = Some((durable_count, detail_count));
    let context = &classified_context;
    let mut rejected: Option<ExtractError> = None;
    let sole = proposals
        .iter()
        .filter(|proposal| proposal.kind != CandidateKind::Detail)
        .count()
        == 1;
    for proposal in proposals {
        // Implementation details are not decisions nor rules.
        if proposal.kind == CandidateKind::Detail {
            continue;
        }
        let proposal = reconcile_with_evidence(proposal, &evidence, sole);
        match validate_proposal(&proposal, &evidence, &signals) {
            Ok(canonical) => validated.push((proposal, canonical)),
            Err(error) => {
                rejected.get_or_insert(error);
            }
        }
    }
    if validated.is_empty() {
        if let Some(error) = rejected {
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
                Some(failure_detail(&error)),
                None,
            )?;
            return Err(error);
        }
    }

    let now = now_rfc3339();
    let natures: Vec<_> = validated
        .iter()
        .map(|(p, signals)| (dedup_hash(capture_id, p, signals), p.nature))
        .collect();
    let mut tally = ComponentTally {
        listed: background.components.len(),
        ..ComponentTally::default()
    };
    let components: Vec<_> = validated
        .iter()
        .map(|(p, signals)| {
            let (resolved, each) = resolve_components(p, &background.components);
            tally += each;
            (dedup_hash(capture_id, p, signals), resolved)
        })
        .collect();
    let records: Vec<DecisionCandidateRecord> = validated
        .into_iter()
        .map(|(proposal, canonical)| {
            let signal_labels: Vec<&str> = canonical.iter().map(RelevanceSignal::as_str).collect();
            let signals_json =
                serde_json::to_string(&signal_labels).unwrap_or_else(|_| "[]".to_string());
            let evidence_refs =
                serde_json::to_string(&proposal.evidence_refs).unwrap_or_else(|_| "[]".to_string());
            DecisionCandidateRecord {
                qualifiers: serde_json::to_string(&proposal.qualifiers)
                    .unwrap_or_else(|_| "[]".into()),
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
                kind: proposal.kind.as_str().to_string(),
                significance: proposal.significance.clamp(0.0, 1.0),
                criteria: serde_json::to_string(&proposal.criteria)
                    .unwrap_or_else(|_| "[]".to_string()),
                dedup_hash: dedup_hash(capture_id, &proposal, &canonical),
                created_at: now.clone(),
                updated_at: now.clone(),
            }
        })
        .collect();

    let inserted = store.insert_candidates(&records)?;
    for (hash, nature) in natures {
        store.record_nature(&hash, nature)?;
    }
    for (hash, resolved) in components {
        if !resolved.is_empty() {
            store.record_components(&hash, &resolved)?;
        }
    }
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
        None,
        Some(tally),
    )?;
    Ok(report)
}

/// Characters of a component's description listed to the extractor: enough to
/// say what it does ("OS integration: ... tray icon and updates"), not its docs.
const MAP_DESCRIPTION_CHARS: usize = 100;

/// The description of a component as the extractor sees it: the one the model
/// reads elsewhere, cut short, on one line.
fn short_description(entity: &crate::graph::EntityRecord) -> String {
    crate::graph::short_description_for_model(entity, MAP_DESCRIPTION_CHARS)
}

/// Components a candidate can apply to at most.
pub const MAX_CANDIDATE_COMPONENTS: usize = 3;

/// The component of the map a written name points to: the name or an alias as
/// the map keys it (case, accents, hyphens, spaces and backticks aside), or,
/// failing that, the part of the name before a description or path suffix
/// (`sc-platform: OS integration...`, `sc-platform::update`,
/// `sc-platform/src/tray.rs`).
fn find_component<'m>(map: &'m [MapComponent], name: &str) -> Option<&'m MapComponent> {
    let by_key = |key: &str| {
        map.iter()
            .find(|component| component.keys.iter().any(|each| each == key))
    };
    by_key(&domain::entities::entity_key(name)).or_else(|| {
        let head = name.split(':').next()?.split('/').next()?;
        if head.len() < name.len() {
            by_key(&domain::entities::entity_key(head))
        } else {
            None
        }
    })
}

/// The components the extractor named for `proposal` that the map has and that
/// the proposal's own text backs: the name matches a listed component or one
/// of its aliases (see [`find_component`]), the quote is copied from the
/// question, choice or rationale, and each component counts once. A name the
/// map does not list, a paraphrase and a quote too short to say anything are
/// dropped. The tally says how many fell where; its `listed` stays 0, because
/// the size of the map is not known per proposal: the caller sets it once.
pub(crate) fn resolve_components(
    proposal: &CandidateProposal,
    map: &[MapComponent],
) -> (Vec<CandidateComponent>, ComponentTally) {
    let texts = crate::link_suggestions::normalized_texts(&[
        proposal.question.as_str(),
        proposal.choice.as_str(),
        proposal.rationale.as_str(),
    ]);
    let mut tally = ComponentTally {
        proposed: proposal.components.len(),
        ..ComponentTally::default()
    };
    let mut resolved: Vec<CandidateComponent> = Vec::new();
    for named in &proposal.components {
        if resolved.len() >= MAX_CANDIDATE_COMPONENTS {
            break;
        }
        let Some(component) = find_component(map, &named.name) else {
            tally.unknown += 1;
            continue;
        };
        if resolved
            .iter()
            .any(|known| known.entity_id == component.entity_id)
        {
            continue;
        }
        if !crate::link_suggestions::quote_matches(&texts, &named.quote) {
            tally.unquoted += 1;
            continue;
        }
        resolved.push(CandidateComponent {
            entity_id: component.entity_id.clone(),
            quote: named.quote.trim().to_string(),
        });
    }
    tally.kept = resolved.len();
    (resolved, tally)
}

/// Replaces what the extractor cannot know better than the app: the diff
/// summary is recomputed from the evidence, and evidence references are kept
/// only when they name a real artifact (a reference that merely contains an
/// id, such as `artifact <id>`, counts). With no usable reference left, the
/// whole capture is cited.
///
/// The files and dependencies of a proposal come from the artifacts it
/// cites, not from the whole capture: two decisions of one turn must not
/// inherit each other's files. When the capture yields a single proposal
/// that cites no file-bearing artifact (only the conversation), the
/// capture's diffs and documents are its evidence.
pub(crate) fn reconcile_with_evidence(
    mut proposal: CandidateProposal,
    evidence: &DecisionEvidence,
    sole: bool,
) -> CandidateProposal {
    let mut references: Vec<String> = Vec::new();
    for reference in &proposal.evidence_refs {
        let reference = reference.trim();
        let found = evidence.artifacts.iter().find(|artifact| {
            artifact.artifact_id == reference || reference.contains(&artifact.artifact_id)
        });
        if let Some(artifact) = found {
            if !references.contains(&artifact.artifact_id) {
                references.push(artifact.artifact_id.clone());
            }
        }
    }
    if references.is_empty() {
        references = evidence
            .artifacts
            .iter()
            .map(|artifact| artifact.artifact_id.clone())
            .collect();
    }
    let bears_files =
        |artifact: &EvidenceArtifact| matches!(artifact.kind.as_str(), "diff_hunk" | "document");
    let cites_files = evidence
        .artifacts
        .iter()
        .any(|artifact| references.contains(&artifact.artifact_id) && bears_files(artifact));
    if sole && !cites_files {
        for artifact in evidence
            .artifacts
            .iter()
            .filter(|artifact| bears_files(artifact))
        {
            if !references.contains(&artifact.artifact_id) {
                references.push(artifact.artifact_id.clone());
            }
        }
    }
    let cited = DecisionEvidence {
        artifacts: evidence
            .artifacts
            .iter()
            .filter(|artifact| references.contains(&artifact.artifact_id))
            .cloned()
            .collect(),
        ..evidence.clone()
    };
    proposal.diff_summary = relevance::diff_summary(&cited);
    proposal.evidence_refs = references;
    proposal.qualifiers = crate::qualifiers::retain_supported(proposal.qualifiers, evidence);
    proposal
}

/// Sorts artifacts deterministically and applies the defensive bounds.
pub(crate) fn normalize(mut evidence: DecisionEvidence) -> DecisionEvidence {
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

#[cfg(test)]
mod tests {
    use super::{
        filter_relevant, reconcile_with_evidence, resolve_components, truncate_content,
        CandidateKind, CandidateProposal, ComponentTally, DecisionEvidence, EvidenceArtifact,
        MapComponent, ProposedComponent, RelevanceSignal,
    };

    fn proposal(refs: &[&str]) -> CandidateProposal {
        CandidateProposal {
            nature: crate::review_exception::CandidateNature::Unknown,
            qualifiers: Vec::new(),
            question: "q".into(),
            choice: "c".into(),
            rationale: "r".into(),
            confidence: 0.9,
            confidence_reason: "x".into(),
            signals: Vec::new(),
            evidence_refs: refs.iter().map(|reference| reference.to_string()).collect(),
            diff_summary: String::new(),
            kind: CandidateKind::Decision,
            significance: 0.8,
            criteria: Vec::new(),
            components: Vec::new(),
        }
    }

    #[test]
    fn each_proposal_keeps_only_the_files_it_cites() {
        let capture = evidence(vec![
            artifact(
                "ui",
                "diff_hunk",
                "diff --git a/web/form.ts b/web/form.ts\n+export const x = 1;\n",
            ),
            artifact(
                "cache",
                "diff_hunk",
                "diff --git a/api/Cargo.toml b/api/Cargo.toml\n+redis = \"0.25\"\n",
            ),
            artifact(
                "talk",
                "user_text",
                "Vamos usar Redis e mudar o formulário.",
            ),
        ]);
        let ui = reconcile_with_evidence(proposal(&["ui", "talk"]), &capture, false);
        assert!(
            ui.diff_summary.contains("web/form.ts"),
            "{}",
            ui.diff_summary
        );
        assert!(
            !ui.diff_summary.contains("api/Cargo.toml"),
            "{}",
            ui.diff_summary
        );
        let cache = reconcile_with_evidence(proposal(&["cache"]), &capture, false);
        assert!(cache.diff_summary.contains("api/Cargo.toml"));
        assert!(!cache.diff_summary.contains("web/form.ts"));

        // A sole proposal that cites only the conversation keeps the diffs.
        let sole = reconcile_with_evidence(proposal(&["talk"]), &capture, true);
        assert!(sole.diff_summary.contains("web/form.ts"));
        assert!(sole.evidence_refs.contains(&"cache".to_string()));
        // Several proposals: citing only the conversation brings no files.
        let shared = reconcile_with_evidence(proposal(&["talk"]), &capture, false);
        assert!(!shared.diff_summary.contains("web/form.ts"));
    }

    #[test]
    fn a_proposal_citing_part_of_the_capture_passes_validation() {
        // A Claude Code turn carries many tool summaries; a decision cites a few.
        let capture = evidence(vec![
            artifact("talk", "user_text", "Vamos usar Redis para o cache."),
            artifact("tool-1", "tool_summary", r#"{"tool":"Read","status":"ok"}"#),
            artifact("tool-2", "tool_summary", r#"{"tool":"Bash","status":"ok"}"#),
        ]);
        let mut cited = proposal(&["talk"]);
        cited.signals = vec![RelevanceSignal::DependencyAdded];
        let cited = reconcile_with_evidence(cited, &capture, false);
        assert_eq!(cited.evidence_refs, ["talk"]);
        super::validate_proposal(&cited, &capture, &[RelevanceSignal::DependencyAdded])
            .expect("a partial citation is valid");
    }

    #[test]
    fn a_misquoted_qualifier_is_dropped_and_the_decision_survives() {
        use crate::qualifiers::{KnowledgeQualifier, QualifierKind};
        let capture = evidence(vec![artifact(
            "doc",
            "document",
            "# ADR
Status: accepted
Applies to the Claude Code adapter only.",
        )]);
        let qualifier = |text: &str, artifact_id: &str| KnowledgeQualifier {
            kind: QualifierKind::Scope,
            text: text.to_string(),
            artifact_id: Some(artifact_id.to_string()),
        };
        let mut cited = proposal(&["doc"]);
        cited.signals = vec![RelevanceSignal::PublicContract];
        cited.qualifiers = vec![
            qualifier("Applies to the Claude Code adapter only.", "doc"),
            // The model paraphrased: not a literal excerpt.
            qualifier("Applies to the adapter of Claude Code.", "doc"),
            // The artifact is named inside a longer reference.
            qualifier("Status: accepted", "artifact doc"),
            // Unknown artifact, and no artifact at all.
            qualifier("Status: accepted", "other"),
            KnowledgeQualifier {
                kind: QualifierKind::Validation,
                text: "Status: accepted".to_string(),
                artifact_id: None,
            },
        ];
        let cited = reconcile_with_evidence(cited, &capture, true);
        let kept: Vec<_> = cited
            .qualifiers
            .iter()
            .map(|item| (item.text.as_str(), item.artifact_id.as_deref()))
            .collect();
        assert_eq!(
            kept,
            [
                ("Applies to the Claude Code adapter only.", Some("doc")),
                ("Status: accepted", Some("doc")),
            ]
        );
        super::validate_proposal(&cited, &capture, &[RelevanceSignal::PublicContract])
            .expect("the decision is still valid");
    }

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

    fn map() -> Vec<MapComponent> {
        ["Storage SQLite", "Núcleo", "outbox", "ui"]
            .iter()
            .enumerate()
            .map(|(n, name)| MapComponent {
                entity_id: format!("e{n}"),
                name: (*name).into(),
                keys: vec![domain::entities::entity_key(name)],
                ..MapComponent::default()
            })
            .collect()
    }

    fn named(name: &str, quote: &str) -> ProposedComponent {
        ProposedComponent {
            name: name.into(),
            quote: quote.into(),
        }
    }

    fn resolved_in(
        map: &[MapComponent],
        components: Vec<ProposedComponent>,
    ) -> (Vec<String>, ComponentTally) {
        let mut candidate = proposal(&[]);
        candidate.question = "Onde o núcleo grava as capturas?".into();
        candidate.choice = "Gravar cada captura na outbox antes de responder.".into();
        candidate.rationale = "Evita perder eventos se o processo cair.".into();
        candidate.components = components;
        let (resolved, tally) = resolve_components(&candidate, map);
        let ids = resolved
            .into_iter()
            .map(|component| component.entity_id)
            .collect();
        (ids, tally)
    }

    fn resolved(components: Vec<ProposedComponent>) -> Vec<String> {
        resolved_in(&map(), components).0
    }

    #[test]
    fn a_component_counts_when_the_map_lists_it_and_the_text_backs_the_quote() {
        // Case does not matter for the name; case and accents do not matter for
        // the quote.
        assert_eq!(
            resolved(vec![
                named("NÚCLEO", "O NUCLEO grava as capturas"),
                named("OUTBOX", "gravar cada captura na outbox"),
            ]),
            vec!["e1", "e2"]
        );
        // A name the map does not list.
        assert!(resolved(vec![named("billing", "Gravar cada captura na outbox")]).is_empty());
        // A paraphrase, and a quote too short to say anything.
        assert!(resolved(vec![named("outbox", "gravar capturas numa fila")]).is_empty());
        assert!(resolved(vec![named("outbox", "outbox")]).is_empty());
        // Once per component, at most three.
        assert_eq!(
            resolved(vec![
                named("outbox", "Gravar cada captura na outbox"),
                named("Outbox", "antes de responder"),
            ]),
            vec!["e2"]
        );
        let many = ["Storage SQLite", "Núcleo", "outbox", "ui"]
            .map(|name| named(name, "Evita perder eventos se o processo cair"));
        assert_eq!(resolved(many.to_vec()).len(), 3);
    }

    #[test]
    fn an_alias_and_a_path_suffix_name_the_component_and_the_tally_says_where_names_fell() {
        let platform = MapComponent {
            entity_id: "e-platform".into(),
            name: "sc-platform".into(),
            keys: vec![
                domain::entities::entity_key("sc-platform"),
                domain::entities::entity_key("Tray"),
            ],
            ..MapComponent::default()
        };
        let map = [platform];
        let quote = "Gravar cada captura na outbox";
        for name in [
            "sc-platform",
            "SC_Platform",
            "`sc-platform`",
            "Tray",
            "Tray: the tray icon",
            "sc-platform::update",
            "sc-platform/src/tray.rs",
        ] {
            let (ids, _) = resolved_in(&map, vec![named(name, quote)]);
            assert_eq!(ids, ["e-platform"], "{name}");
        }
        let (ids, tally) = resolved_in(
            &map,
            vec![
                named("sc-platform crate", quote),
                named("sc-platform", "um trecho que o texto não tem"),
                named("sc-platform", quote),
                named("tray", quote),
            ],
        );
        assert_eq!(ids, ["e-platform"]);
        assert_eq!(
            tally,
            ComponentTally {
                listed: 0,
                proposed: 4,
                kept: 1,
                unknown: 1,
                unquoted: 1
            },
            "the repeated name is only proposed"
        );
    }

    #[test]
    fn a_name_copied_with_its_description_resolves() {
        let map: Vec<MapComponent> = ["sc-core", "sc-platform", "CI", "cloudrs"]
            .iter()
            .map(|name| MapComponent {
                entity_id: format!("e-{name}"),
                name: (*name).into(),
                keys: vec![domain::entities::entity_key(name)],
                ..MapComponent::default()
            })
            .collect();
        let quote = "Gravar cada captura na outbox";
        for (copied, id) in [
            (
                "sc-core: cloudrs application core: state, rules and the bridge to SoundCloud",
                "e-sc-core",
            ),
            (
                "sc-platform: cloudrs OS integration: keychain, tray icon and updates",
                "e-sc-platform",
            ),
            (
                "CI: Continuous integration and release (.github/workflows/**)",
                "e-CI",
            ),
            (
                "cloudrs: Native SoundCloud client for the desktop",
                "e-cloudrs",
            ),
        ] {
            let (ids, tally) = resolved_in(&map, vec![named(copied, quote)]);
            assert_eq!(ids, [id], "{copied}");
            assert_eq!((tally.kept, tally.unknown), (1, 0), "{copied}");
        }
        assert!(resolved_in(&map, vec![named("billing: something", quote)])
            .0
            .is_empty());
        assert!(resolved_in(&map, vec![named("sc-platform crate", quote)])
            .0
            .is_empty());
    }
}
