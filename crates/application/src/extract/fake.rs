//! Deterministic offline extractor (MVP-SPEC §12).

use super::relevance::diff_summary;
use super::{
    CandidateExtractor, CandidateProposal, DecisionEvidence, ExtractError, RelevanceSignal,
};

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
