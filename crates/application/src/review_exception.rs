//! Conservative review grouping; repetition never grants normative authority.

use crate::inbox::{InboxError, StoredCandidate};

/// Model classification is a proposal, never authority.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateNature {
    /// Descriptive assertion requiring typed local verification.
    Description,
    /// Unverified explanatory inference, not a normative approval request.
    Inference,
    /// Proposed norm requiring explicit review.
    Normative,
    /// Legacy or unsupported classification; remains reviewable.
    #[default]
    Unknown,
}

/// Exact local reference; not an additional external-input category.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationReference {
    /// Exact source content hash.
    pub source_sha256: String,
    /// Exact parser policy.
    pub parser_policy_version: String,
    /// Exact supporting workspace declarations.
    pub supporting_sources: Vec<crate::observations::ObservationSupport>,
    /// Exact path scope.
    pub path_scope: Vec<String>,
    /// Project owning the observation.
    pub project_id: String,
    /// Typed observation identity.
    pub observation_id: String,
    /// Exact immutable revision.
    pub version: i64,
    /// Source identity.
    pub source_id: String,
    /// Exact declared field pointer.
    pub field_pointer: String,
    /// Exact subject, not interpreted prose.
    pub subject: crate::observations::ObservationSubject,
    /// Exact declared values, not installed versions.
    pub value: crate::observations::ObservationValue,
}

impl ObservationReference {
    /// Canonical descriptive content, including every declared value and its
    /// exact source/scope/support identity. Free prose is never equivalent.
    pub fn canonical_description(&self) -> String {
        serde_json::to_string(self).expect("typed observation serialization is infallible")
    }
}

/// Pure classification result; no normative acceptance is possible here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NatureDestination {
    /// Human review remains necessary.
    ReviewRequired,
    /// Stored inference with original evidence.
    InferenceStored,
    /// Exact current descriptive record already represents the structured fact.
    DescriptionVerified,
}

/// Persisted trace without exposing or discarding original candidate evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NatureTrace {
    /// Retained candidate identity.
    pub candidate_id: String,
    /// Proposed classification, not authority.
    pub nature: CandidateNature,
    /// Local disposition.
    pub destination: NatureDestination,
}

/// Read-only classification trace port for progress and detail surfaces.
pub trait NatureTraceStore {
    /// Attaches an explicit local typed reference, never inferred from prose.
    fn bind_observation(
        &self,
        candidate_id: &str,
        reference: &ObservationReference,
    ) -> Result<NatureDestination, InboxError>;
    /// Bounded per-project trace page.
    fn nature_traces(
        &self,
        project_id: &str,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<NatureTrace>, InboxError>;
}

/// Verifies typed identity against a current, eligible local snapshot.
pub fn classify_nature(
    nature: CandidateNature,
    project_id: &str,
    reference: Option<&ObservationReference>,
    record: Option<&crate::observations::ObservationRecord>,
    eligible: bool,
) -> NatureDestination {
    if nature == CandidateNature::Inference {
        return NatureDestination::InferenceStored;
    }
    if nature == CandidateNature::Description && eligible {
        if let (Some(reference), Some(record)) = (reference, record) {
            if reference.project_id == project_id
                && record.project_id == project_id
                && reference.observation_id == record.observation_id
                && reference.version == record.version
                && reference.source_id == record.provenance.source_id
                && reference.source_sha256 == record.provenance.source_sha256
                && reference.parser_policy_version == record.provenance.parser_policy_version
                && reference.supporting_sources == record.provenance.supporting_sources
                && reference.path_scope == record.path_scope
                && reference.field_pointer == record.provenance.field_pointer
                && reference.subject == record.subject
                && reference.value == record.value
                && record.status == crate::observations::RecordStatus::Current
            {
                return NatureDestination::DescriptionVerified;
            }
        }
    }
    NatureDestination::ReviewRequired
}

/// Provenance of an action, never inferred from the API caller.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ReviewActor {
    /// Explicit human action.
    Human,
    /// Automated action.
    Automation,
    /// Laboratory action.
    Lab,
    /// No authenticated provenance supplied.
    #[default]
    Unknown,
}

/// One occurrence, retaining its own evidence and lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewMember {
    /// Candidate identifier.
    pub candidate_id: String,
    /// Original capture identifier.
    pub capture_id: String,
    /// Original evidence references, not merged into the representative.
    pub evidence_refs: Vec<String>,
    /// Lateral result; does not change candidate status.
    pub already_represented: bool,
}

/// Paged disclosure of a group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewGroup {
    /// Content and conservative scope fingerprint.
    pub fingerprint: String,
    /// Total retained occurrences.
    pub occurrence_count: usize,
    /// Requested members only.
    pub members: Vec<ReviewMember>,
}

/// Potential opportunities, not measured human work or elapsed time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewMetrics {
    /// Pending individual occurrences.
    pub pending_occurrences: usize,
    /// Pending representative opportunities.
    pub potential_review_opportunities: usize,
}

/// Storage port for paged occurrence disclosure and potential metrics.
pub trait ReviewExceptionStore {
    /// Loads a group through a selected candidate, with bounded pagination.
    fn review_group(
        &self,
        candidate_id: &str,
        offset: usize,
        limit: usize,
    ) -> Result<ReviewGroup, InboxError>;
    /// Returns potential counts for one project.
    fn review_metrics(&self, project_id: &str) -> Result<ReviewMetrics, InboxError>;
}

fn whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Stable identity: preserves case, punctuation, negation and qualifier order.
/// Empty file scope stays capture-local rather than pretending universal scope.
pub fn group_fingerprint(row: &StoredCandidate) -> Result<String, InboxError> {
    let summary: crate::inbox::DiffSummary = serde_json::from_str(&row.diff_summary)
        .map_err(|_| InboxError::InvalidData("diff_summary inválido".into()))?;
    let mut files = summary.files;
    files.sort();
    files.dedup();
    let qualifiers = crate::qualifiers::decode(&row.qualifiers).map_err(InboxError::InvalidData)?;
    let qualifiers: Vec<_> = qualifiers
        .iter()
        .map(|q| {
            // Attribution support cannot safely be equated across distinct sources.
            let attribution = (q.kind == crate::qualifiers::QualifierKind::Attribution)
                .then_some((&row.capture_id, &q.artifact_id));
            (
                q.kind,
                whitespace(&q.text),
                q.artifact_id.is_some(),
                attribution,
            )
        })
        .collect();
    let bytes = serde_json::to_string(&(
        &row.project_id,
        &row.kind,
        whitespace(&row.question),
        whitespace(&row.choice),
        whitespace(&row.rationale),
        files.clone(),
        files.is_empty().then_some(&row.capture_id),
        qualifiers,
    ))
    .map_err(|e| InboxError::InvalidData(e.to_string()))?;
    Ok(integration_contracts::capture::artifact_fingerprint(&bytes))
}

/// Exact occurrence snapshot, including provenance and lifecycle.
pub fn source_fingerprint(row: &StoredCandidate) -> String {
    let bytes = serde_json::to_string(&(
        &row.id,
        &row.project_id,
        &row.capture_id,
        row.status.as_str(),
        &row.kind,
        &row.question,
        &row.choice,
        &row.rationale,
        &row.qualifiers,
        &row.evidence_refs,
        &row.diff_summary,
        &row.updated_at,
    ))
    .expect("string tuple serialization is infallible");
    integration_contracts::capture::artifact_fingerprint(&bytes)
}

#[cfg(test)]
mod nature_tests {
    use super::*;

    #[test]
    fn unsupported_description_and_inference_never_become_verified_norms() {
        assert_eq!(
            classify_nature(CandidateNature::Description, "p", None, None, true),
            NatureDestination::ReviewRequired
        );
        assert_eq!(
            classify_nature(CandidateNature::Inference, "p", None, None, false),
            NatureDestination::InferenceStored
        );
        assert_eq!(
            classify_nature(CandidateNature::Unknown, "p", None, None, false),
            NatureDestination::ReviewRequired
        );
        assert_eq!(
            classify_nature(CandidateNature::Normative, "p", None, None, true),
            NatureDestination::ReviewRequired
        );
        assert!(serde_json::from_str::<CandidateNature>("\"approved\"").is_err());
    }
}
