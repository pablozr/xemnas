//! Provider connection test with fixed synthetic evidence.

use super::validation::validate_proposal;
use super::{
    filter_relevant, normalize, CandidateExtractor, DecisionEvidence, EvidenceArtifact,
    ExtractError,
};

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
