//! Extraction of Decision Candidates from a persisted capture.

mod assessment;
mod connection_test;
mod fake;
mod relevance;
mod validation;

use integration_contracts::capture::artifact_fingerprint;

use crate::clock::now_rfc3339;

pub use assessment::{
    fail_provider_setup, input_hash, policy_snapshot, record_skipped_assessment, AssessmentOutcome,
    AssessmentRecord, AssessmentStore, ProviderSetupError, RunContext, ERROR_CODE_CONSENT,
    ERROR_CODE_KEYSTORE, ERROR_CODE_PROFILE, ERROR_CODE_PROVIDER_CONFIG, ERROR_CODE_SECRET,
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
    pub fn code(&self) -> &'static str {
        match self {
            Self::Storage(_) => "storage",
            Self::Extractor(_) => "extractor",
            Self::Validation(_) => "validation",
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
