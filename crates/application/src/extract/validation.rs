//! Validation of extractor proposals before anything is persisted.

use serde_json::Value;

use super::{
    CandidateProposal, DecisionEvidence, ExtractError, RelevanceSignal, MAX_DIFF_SUMMARY_FILES,
};

/// Validates a proposal before anything is persisted and returns its canonical
/// signal list.
pub(super) fn validate_proposal(
    proposal: &CandidateProposal,
    evidence: &DecisionEvidence,
    detected: &[RelevanceSignal],
) -> Result<Vec<RelevanceSignal>, ExtractError> {
    crate::qualifiers::validate_extracted(&proposal.qualifiers, evidence)
        .map_err(ExtractError::Validation)?;
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
