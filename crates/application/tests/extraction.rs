//! Extraction integration tests: durable fixtures yield pending candidates,
//! trivial fixtures yield none, reprocessing never duplicates, and an extractor
//! failure writes nothing.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use application::extract::{
    connection_test_evidence, fail_provider_setup, filter_relevant, input_hash,
    record_skipped_assessment, run_connection_test, run_extraction, AssessmentOutcome,
    AssessmentRecord, AssessmentStore, CandidateExtractor, CandidateKind, CandidateProposal,
    DecisionCandidateRecord, DecisionEvidence, EvidenceArtifact, ExtractError, ExtractionStore,
    FakeCandidateExtractor, ProviderSetupError, RelevanceSignal, RunContext,
};
use integration_contracts::capture::CaptureEnvelope;

const DURABLE: &[&str] = &[
    "durable-migration.json",
    "durable-dependency.json",
    "durable-migration-with-formatting-note.json",
];
const TRIVIAL: &[&str] = &["trivial-formatting.json", "trivial-test-explained.json"];
const TRIVIAL_INCIDENTAL: &[&str] = &[
    "trivial-incidental-terms.json",
    "trivial-test-only-token.json",
    "trivial-two-directories.json",
    "trivial-sql-comment-only.json",
    "trivial-sql-whitespace.json",
    "trivial-prose-strong-text.json",
    "trivial-manifest-comment.json",
];

fn fixture_path(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("extraction")
        .join(name)
}

fn evidence(name: &str) -> DecisionEvidence {
    let path = fixture_path(name);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    let envelope: CaptureEnvelope = serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()));
    DecisionEvidence {
        capture_id: envelope.capture_id.clone(),
        project_id: "project-1".to_string(),
        adapter: Some(envelope.source.adapter.clone()),
        session_id: Some(envelope.source.session_id.clone()),
        observed_at: Some(envelope.observed_at.clone()),
        artifacts: envelope
            .artifacts
            .iter()
            .map(|artifact| EvidenceArtifact {
                artifact_id: artifact.artifact_id.clone(),
                kind: artifact.kind.as_str().to_string(),
                content: artifact.content.clone(),
                metadata: serde_json::to_string(&artifact.metadata)
                    .unwrap_or_else(|_| "{}".to_string()),
            })
            .collect(),
    }
}

/// Conversation-only evidence: text artifacts with no `diff_hunk`, as in a turn
/// that records a decision which was not implemented (MVP-SPEC AD-08).
fn conversation_evidence(parts: &[(&str, &str)]) -> DecisionEvidence {
    DecisionEvidence {
        capture_id: "capture-conversation".to_string(),
        project_id: "project-1".to_string(),
        adapter: Some("opencode".to_string()),
        session_id: Some("session-conversation".to_string()),
        observed_at: Some("2026-01-01T00:00:00Z".to_string()),
        artifacts: parts
            .iter()
            .enumerate()
            .map(|(index, (kind, content))| EvidenceArtifact {
                artifact_id: format!("conversation-{index}"),
                kind: (*kind).to_string(),
                content: (*content).to_string(),
                metadata: "{}".to_string(),
            })
            .collect(),
    }
}

#[derive(Default)]
struct FakeStore {
    evidence: Mutex<HashMap<String, DecisionEvidence>>,
    records: Mutex<Vec<DecisionCandidateRecord>>,
    assessments: Mutex<Vec<AssessmentRecord>>,
}

impl FakeStore {
    fn with(evidence: DecisionEvidence) -> Self {
        let store = Self::default();
        store
            .evidence
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(evidence.capture_id.clone(), evidence);
        store
    }

    fn records(&self) -> Vec<DecisionCandidateRecord> {
        self.records
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    fn assessments(&self) -> Vec<AssessmentRecord> {
        self.assessments
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
}

impl ExtractionStore for FakeStore {
    fn load_evidence(&self, capture_id: &str) -> Result<Option<DecisionEvidence>, ExtractError> {
        Ok(self
            .evidence
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(capture_id)
            .cloned())
    }

    fn insert_candidates(
        &self,
        records: &[DecisionCandidateRecord],
    ) -> Result<usize, ExtractError> {
        let mut stored = self
            .records
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut inserted = 0;
        for record in records {
            if stored
                .iter()
                .any(|existing| existing.dedup_hash == record.dedup_hash)
            {
                continue;
            }
            stored.push(record.clone());
            inserted += 1;
        }
        Ok(inserted)
    }
}

impl AssessmentStore for FakeStore {
    fn record_assessment(&self, row: &AssessmentRecord) -> Result<(), ExtractError> {
        self.assessments
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(row.clone());
        Ok(())
    }
}

struct FailingExtractor;

impl CandidateExtractor for FailingExtractor {
    fn extract(
        &self,
        _input: &DecisionEvidence,
        _signals: &[RelevanceSignal],
    ) -> Result<Vec<CandidateProposal>, ExtractError> {
        Err(ExtractError::Extractor("synthetic failure".to_string()))
    }
}

#[test]
fn durable_fixtures_produce_pending_candidates() {
    for name in DURABLE {
        let evidence = evidence(name);
        let store = FakeStore::with(evidence.clone());
        let report = run_extraction(
            &store,
            &FakeCandidateExtractor,
            &evidence.capture_id,
            &RunContext::for_tests(),
        )
        .unwrap_or_else(|error| panic!("{name}: {error}"));

        assert!(report.candidates >= 1, "{name}: expected a candidate");
        assert!(report.inserted >= 1, "{name}: expected an insert");
        assert!(!report.signals.is_empty(), "{name}: expected signals");

        let records = store.records();
        assert_eq!(records.len(), report.candidates);
        for record in &records {
            assert_eq!(record.status, "pending", "{name}: only pending is written");
            assert_eq!(record.capture_id, evidence.capture_id);
            assert!(!record.dedup_hash.is_empty());
        }
    }
}

#[test]
fn trivial_fixtures_produce_no_candidates() {
    for name in TRIVIAL {
        let evidence = evidence(name);
        let store = FakeStore::with(evidence.clone());
        let report = run_extraction(
            &store,
            &FakeCandidateExtractor,
            &evidence.capture_id,
            &RunContext::for_tests(),
        )
        .unwrap_or_else(|error| panic!("{name}: {error}"));

        assert_eq!(
            report.candidates, 0,
            "{name}: trivial change must not filter"
        );
        assert_eq!(report.inserted, 0, "{name}: nothing to insert");
        assert!(store.records().is_empty(), "{name}: no candidate rows");
    }
}

#[test]
fn incidental_strong_terms_in_trivial_changes_are_vetoed() {
    for name in TRIVIAL_INCIDENTAL {
        let evidence = evidence(name);
        let store = FakeStore::with(evidence.clone());
        let report = run_extraction(
            &store,
            &FakeCandidateExtractor,
            &evidence.capture_id,
            &RunContext::for_tests(),
        )
        .unwrap_or_else(|error| panic!("{name}: {error}"));

        assert_eq!(
            report.candidates, 0,
            "{name}: veto must suppress incidental strong terms"
        );
        assert!(report.signals.is_empty(), "{name}: no signal may survive");
        assert!(store.records().is_empty(), "{name}: no candidate rows");
    }
}

#[test]
fn sql_and_prose_diff_traps_are_vetoed() {
    for name in [
        "trivial-sql-comment-only.json",
        "trivial-sql-whitespace.json",
        "trivial-prose-strong-text.json",
    ] {
        let evidence = evidence(name);
        let store = FakeStore::with(evidence.clone());
        let report = run_extraction(
            &store,
            &FakeCandidateExtractor,
            &evidence.capture_id,
            &RunContext::for_tests(),
        )
        .unwrap_or_else(|error| panic!("{name}: {error}"));

        assert_eq!(report.candidates, 0, "{name}: must be vetoed");
        assert!(report.signals.is_empty(), "{name}: no signal may survive");
        assert!(store.records().is_empty(), "{name}: no candidate rows");
    }
}

#[test]
fn a_trivial_marker_does_not_discard_a_real_ddl_change() {
    let name = "durable-migration-with-formatting-note.json";
    let evidence = evidence(name);
    let store = FakeStore::with(evidence.clone());
    let report = run_extraction(
        &store,
        &FakeCandidateExtractor,
        &evidence.capture_id,
        &RunContext::for_tests(),
    )
    .unwrap_or_else(|error| panic!("{name}: {error}"));

    assert!(
        report.candidates >= 1,
        "an incidental formatting note must not veto a real migration"
    );
    assert!(
        report.signals.contains(&RelevanceSignal::PublicContract),
        "the strong structural signal must survive"
    );
    assert!(!store.records().is_empty());
}

#[test]
fn conversation_with_a_durable_choice_is_extracted() {
    let evidence = conversation_evidence(&[
        (
            "user_text",
            "Decidimos manter o schema dos candidatos em SQLite em vez de Postgres, \
             porque o app e local e sem servidor; o trade-off e abrir mao de \
             concorrencia entre maquinas.",
        ),
        (
            "assistant_text",
            "Registrei a escolha: SQLite, com isolamento por projeto.",
        ),
    ]);
    let store = FakeStore::with(evidence.clone());
    let report = run_extraction(
        &store,
        &FakeCandidateExtractor,
        &evidence.capture_id,
        &RunContext::for_tests(),
    )
    .expect("conversation extraction");

    assert!(
        report.candidates >= 1,
        "a conversation with a durable choice must reach the extractor"
    );
    assert!(report.inserted >= 1, "the candidate must be inserted");
    assert!(
        report.signals.contains(&RelevanceSignal::PublicContract),
        "the persistence/schema choice must be signaled"
    );
    assert!(
        report
            .signals
            .contains(&RelevanceSignal::RejectsAlternative),
        "the rejected alternative must be signaled"
    );

    let records = store.records();
    assert_eq!(records.len(), report.candidates);
    assert_eq!(records[0].status, "pending");
    assert_eq!(records[0].capture_id, evidence.capture_id);
}

#[test]
fn neutral_conversation_without_a_choice_is_not_extracted() {
    let evidence = conversation_evidence(&[
        (
            "user_text",
            "Bom dia! Vamos ajustar os titulos do README e revisar a ortografia.",
        ),
        (
            "assistant_text",
            "Feito: os titulos seguem o mesmo padrao das outras secoes.",
        ),
    ]);
    let store = FakeStore::with(evidence.clone());
    let report = run_extraction(
        &store,
        &FakeCandidateExtractor,
        &evidence.capture_id,
        &RunContext::for_tests(),
    )
    .expect("neutral extraction");

    assert_eq!(report.candidates, 0, "neutral chatter is not a decision");
    assert!(report.signals.is_empty(), "neutral chatter has no signal");
    assert!(store.records().is_empty(), "no candidate rows");
}

#[test]
fn conversation_fact_with_strong_terms_but_no_choice_is_not_extracted() {
    let evidence = conversation_evidence(&[(
        "assistant_text",
        "CREATE TABLE migration endpoint token just an example sentence.",
    )]);
    let store = FakeStore::with(evidence.clone());
    let report = run_extraction(
        &store,
        &FakeCandidateExtractor,
        &evidence.capture_id,
        &RunContext::for_tests(),
    )
    .expect("fact extraction");

    assert_eq!(
        report.candidates, 0,
        "a bare mention of contract/security nouns is a fact, not a choice"
    );
    assert!(report.signals.is_empty(), "no signal may survive");
    assert!(store.records().is_empty(), "no candidate rows");
}

#[test]
fn conversation_local_detail_without_a_choice_is_not_extracted() {
    let evidence = conversation_evidence(&[(
        "user_text",
        "Ajustei o espacamento do cartao e a cor do botao; e so estetica local.",
    )]);
    let store = FakeStore::with(evidence.clone());
    let report = run_extraction(
        &store,
        &FakeCandidateExtractor,
        &evidence.capture_id,
        &RunContext::for_tests(),
    )
    .expect("local detail extraction");

    assert_eq!(report.candidates, 0, "local aesthetic detail is excluded");
    assert!(store.records().is_empty(), "no candidate rows");
}

/// A proposal that passes validation, for tests that mutate one field at a time.
fn valid_proposal(input: &DecisionEvidence, signals: &[RelevanceSignal]) -> CandidateProposal {
    CandidateProposal {
        nature: application::review_exception::CandidateNature::Unknown,
        qualifiers: Vec::new(),
        question: "q".to_string(),
        choice: "c".to_string(),
        rationale: "r".to_string(),
        confidence: 0.5,
        confidence_reason: "x".to_string(),
        signals: signals.to_vec(),
        evidence_refs: input
            .artifacts
            .iter()
            .map(|artifact| artifact.artifact_id.clone())
            .collect(),
        diff_summary: serde_json::json!({"files": [], "artifacts": input.artifacts.len()})
            .to_string(),
        kind: CandidateKind::Decision,
        significance: 0.8,
        criteria: Vec::new(),
        components: Vec::new(),
    }
}

/// Applies a mutation to a proposal so validation failures can be exercised.
struct BadExtractor {
    mutate: fn(&mut CandidateProposal),
}

impl CandidateExtractor for BadExtractor {
    fn extract(
        &self,
        input: &DecisionEvidence,
        signals: &[RelevanceSignal],
    ) -> Result<Vec<CandidateProposal>, ExtractError> {
        let mut proposal = valid_proposal(input, signals);
        (self.mutate)(&mut proposal);
        Ok(vec![proposal])
    }
}

#[test]
fn invalid_proposals_are_rejected() {
    let mutators: &[fn(&mut CandidateProposal)] = &[
        |proposal| proposal.question = "   ".to_string(),
        |proposal| proposal.choice = String::new(),
        |proposal| proposal.rationale = String::new(),
        |proposal| proposal.confidence_reason = String::new(),
        |proposal| proposal.confidence = 1.5,
        |proposal| proposal.confidence = f64::NAN,
        |proposal| proposal.signals.clear(),
        |proposal| proposal.signals = vec![RelevanceSignal::DelegatedToAgent],
    ];

    for mutate in mutators {
        let evidence = evidence(DURABLE[0]);
        let store = FakeStore::with(evidence.clone());
        let result = run_extraction(
            &store,
            &BadExtractor { mutate: *mutate },
            &evidence.capture_id,
            &RunContext::for_tests(),
        );
        assert!(
            matches!(result, Err(ExtractError::Validation(_))),
            "an invalid proposal is not persisted"
        );
        assert!(store.records().is_empty(), "nothing may be persisted");
    }
}

#[test]
fn facts_about_the_capture_come_from_the_evidence_not_the_model() {
    let mutators: &[fn(&mut CandidateProposal)] = &[
        |proposal| proposal.evidence_refs = vec!["artifact-missing".to_string()],
        |proposal| proposal.evidence_refs.clear(),
        |proposal| proposal.diff_summary = "{ not json".to_string(),
        |proposal| proposal.diff_summary = "[]".to_string(),
        |proposal| {
            proposal.diff_summary = serde_json::json!({"files": [1], "artifacts": 0}).to_string()
        },
        |proposal| {
            proposal.diff_summary = serde_json::json!({"files": [], "artifacts": 999}).to_string()
        },
    ];

    for mutate in mutators {
        let evidence = evidence(DURABLE[0]);
        let store = FakeStore::with(evidence.clone());
        let report = run_extraction(
            &store,
            &BadExtractor { mutate: *mutate },
            &evidence.capture_id,
            &RunContext::for_tests(),
        )
        .expect("the app reconciles what it knows");
        assert_eq!(report.inserted, 1);
        let record = &store.records()[0];
        let summary: serde_json::Value =
            serde_json::from_str(&record.diff_summary).expect("summary json");
        assert_eq!(
            summary["artifacts"],
            serde_json::json!(evidence.artifacts.len())
        );
        let refs: Vec<String> = serde_json::from_str(&record.evidence_refs).expect("refs json");
        assert!(!refs.is_empty());
        assert!(refs.iter().all(|reference| evidence
            .artifacts
            .iter()
            .any(|a| &a.artifact_id == reference)));
    }
}

/// Returns one valid proposal and one without a question.
struct MixedExtractor;

impl CandidateExtractor for MixedExtractor {
    fn extract(
        &self,
        input: &DecisionEvidence,
        signals: &[RelevanceSignal],
    ) -> Result<Vec<CandidateProposal>, ExtractError> {
        let good = valid_proposal(input, signals);
        let mut bad = valid_proposal(input, signals);
        bad.question = String::new();
        Ok(vec![bad, good])
    }
}

#[test]
fn one_bad_proposal_does_not_drop_the_good_ones() {
    let evidence = evidence(DURABLE[0]);
    let store = FakeStore::with(evidence.clone());
    let report = run_extraction(
        &store,
        &MixedExtractor,
        &evidence.capture_id,
        &RunContext::for_tests(),
    )
    .expect("the valid proposal survives");
    assert_eq!(report.inserted, 1);
    assert_eq!(store.records().len(), 1);
}

/// Returns the filter signals reordered by the given function.
struct OrderedExtractor {
    order: fn(&[RelevanceSignal]) -> Vec<RelevanceSignal>,
}

impl CandidateExtractor for OrderedExtractor {
    fn extract(
        &self,
        input: &DecisionEvidence,
        signals: &[RelevanceSignal],
    ) -> Result<Vec<CandidateProposal>, ExtractError> {
        let mut proposal = valid_proposal(input, signals);
        proposal.signals = (self.order)(signals);
        Ok(vec![proposal])
    }
}

#[test]
fn signals_order_does_not_change_the_dedup_hash() {
    let evidence = evidence(DURABLE[1]);
    let first_store = FakeStore::with(evidence.clone());
    let second_store = FakeStore::with(evidence.clone());

    run_extraction(
        &first_store,
        &OrderedExtractor {
            order: |signals| signals.to_vec(),
        },
        &evidence.capture_id,
        &RunContext::for_tests(),
    )
    .expect("first run");
    run_extraction(
        &second_store,
        &OrderedExtractor {
            order: |signals| {
                let mut reversed = signals.to_vec();
                reversed.reverse();
                reversed
            },
        },
        &evidence.capture_id,
        &RunContext::for_tests(),
    )
    .expect("second run");

    let first = first_store.records();
    let second = second_store.records();
    assert_eq!(first.len(), 1);
    assert_eq!(second.len(), 1);
    assert_eq!(
        first[0].dedup_hash, second[0].dedup_hash,
        "canonical signals must give the same hash regardless of order"
    );
    assert_eq!(
        first[0].signals, second[0].signals,
        "canonical signals column must match"
    );
}

#[test]
fn reprocessing_does_not_duplicate_candidates() {
    let evidence = evidence(DURABLE[0]);
    let store = FakeStore::with(evidence.clone());

    let first = run_extraction(
        &store,
        &FakeCandidateExtractor,
        &evidence.capture_id,
        &RunContext::for_tests(),
    )
    .expect("first extraction");
    let second = run_extraction(
        &store,
        &FakeCandidateExtractor,
        &evidence.capture_id,
        &RunContext::for_tests(),
    )
    .expect("second extraction");

    assert_eq!(first.inserted, first.candidates);
    assert_eq!(second.candidates, first.candidates);
    assert_eq!(
        second.inserted, 0,
        "the dedup hash prevents a second insert"
    );
    assert_eq!(store.records().len(), first.candidates);
}

#[test]
fn extractor_failure_writes_nothing() {
    let evidence = evidence(DURABLE[0]);
    let store = FakeStore::with(evidence.clone());

    let result = run_extraction(
        &store,
        &FailingExtractor,
        &evidence.capture_id,
        &RunContext::for_tests(),
    );
    assert!(matches!(result, Err(ExtractError::Extractor(_))));
    assert!(
        store.records().is_empty(),
        "no partial candidate may be written"
    );
}

#[test]
fn proposals_are_reproducible_across_runs() {
    let evidence = evidence(DURABLE[1]);
    let first_store = FakeStore::with(evidence.clone());
    let second_store = FakeStore::with(evidence.clone());

    run_extraction(
        &first_store,
        &FakeCandidateExtractor,
        &evidence.capture_id,
        &RunContext::for_tests(),
    )
    .expect("first run");
    run_extraction(
        &second_store,
        &FakeCandidateExtractor,
        &evidence.capture_id,
        &RunContext::for_tests(),
    )
    .expect("second run");

    let first = first_store.records();
    let second = second_store.records();
    assert_eq!(first.len(), second.len());

    for (left, right) in first.iter().zip(second.iter()) {
        assert_eq!(left.question, right.question);
        assert_eq!(left.choice, right.choice);
        assert_eq!(left.rationale, right.rationale);
        assert_eq!(left.confidence, right.confidence);
        assert_eq!(left.confidence_reason, right.confidence_reason);
        assert_eq!(left.signals, right.signals);
        assert_eq!(left.evidence_refs, right.evidence_refs);
        assert_eq!(left.diff_summary, right.diff_summary);
        assert_eq!(left.dedup_hash, right.dedup_hash);
        assert_eq!(left.status, right.status);
    }
}

/// A fully populated context, so the provenance columns can be checked.
fn detailed_context() -> RunContext {
    RunContext {
        attempt: None,
        classification: None,
        profile_id: "profile-7".to_string(),
        adapter: "openai-compatible".to_string(),
        model: Some("gpt-test".to_string()),
        policy_json: "{\"kind\":\"open_ai_compatible\",\"enabled\":true}".to_string(),
        consent_preview_hash: Some("hash-9".to_string()),
        job_id: Some("job-3".to_string()),
    }
}

#[test]
fn assessment_records_ok_with_counts_and_context() {
    let evidence = evidence(DURABLE[0]);
    let store = FakeStore::with(evidence.clone());
    let context = detailed_context();

    let report = run_extraction(
        &store,
        &FakeCandidateExtractor,
        &evidence.capture_id,
        &context,
    )
    .expect("extraction succeeds");

    let rows = store.assessments();
    assert_eq!(rows.len(), 1, "one provenance row per run");
    let row = &rows[0];
    assert_eq!(row.outcome, AssessmentOutcome::Ok);
    assert_eq!(row.candidates, report.candidates as i64);
    assert_eq!(row.inserted, report.inserted as i64);
    assert_eq!(row.profile_id, "profile-7");
    assert_eq!(row.adapter, "openai-compatible");
    assert_eq!(row.model.as_deref(), Some("gpt-test"));
    assert_eq!(row.policy, context.policy_json);
    assert_eq!(row.consent_preview_hash.as_deref(), Some("hash-9"));
    assert_eq!(row.job_id.as_deref(), Some("job-3"));
    assert_eq!(row.error_code, None);
    assert_eq!(row.capture_id, evidence.capture_id);
    assert_eq!(
        row.input_hash,
        input_hash(&evidence.capture_id, &evidence),
        "the stored hash must match a pure recomputation"
    );
    assert!(!row.started_at.is_empty() && !row.finished_at.is_empty());
}

#[test]
fn assessment_records_empty_when_the_filter_is_empty() {
    let evidence = evidence(TRIVIAL[0]);
    let store = FakeStore::with(evidence.clone());

    let report = run_extraction(
        &store,
        &FakeCandidateExtractor,
        &evidence.capture_id,
        &RunContext::for_tests(),
    )
    .expect("trivial extraction succeeds");

    assert_eq!(report.candidates, 0);
    let rows = store.assessments();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].outcome, AssessmentOutcome::Empty);
    assert_eq!(rows[0].candidates, 0);
    assert_eq!(rows[0].inserted, 0);
    assert_eq!(rows[0].error_code, None);
}

#[test]
fn assessment_records_failed_for_extractor_and_validation() {
    let evidence = evidence(DURABLE[0]);

    let failing = FakeStore::with(evidence.clone());
    let result = run_extraction(
        &failing,
        &FailingExtractor,
        &evidence.capture_id,
        &RunContext::for_tests(),
    );
    assert!(matches!(result, Err(ExtractError::Extractor(_))));
    let rows = failing.assessments();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].outcome, AssessmentOutcome::Failed);
    assert_eq!(rows[0].error_code.as_deref(), Some("extractor"));
    assert_eq!((rows[0].candidates, rows[0].inserted), (0, 0));

    let invalid = FakeStore::with(evidence.clone());
    let result = run_extraction(
        &invalid,
        &BadExtractor {
            mutate: |proposal| proposal.confidence = 1.5,
        },
        &evidence.capture_id,
        &RunContext::for_tests(),
    );
    assert!(matches!(result, Err(ExtractError::Validation(_))));
    let rows = invalid.assessments();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].outcome, AssessmentOutcome::Failed);
    assert_eq!(rows[0].error_code.as_deref(), Some("validation"));
    assert!(invalid.records().is_empty(), "no partial candidate batch");
}

#[test]
fn input_hash_is_deterministic_sensitive_and_order_insensitive() {
    let mut evidence = evidence(DURABLE[1]);
    let first = input_hash(&evidence.capture_id, &evidence);
    assert_eq!(first, input_hash(&evidence.capture_id, &evidence));

    evidence.artifacts.reverse();
    assert_eq!(
        first,
        input_hash(&evidence.capture_id, &evidence),
        "artifact order must not matter"
    );

    let mut changed = evidence.clone();
    changed.artifacts[0].content.push_str("\n+synthetic change");
    assert_ne!(first, input_hash(&changed.capture_id, &changed));

    assert_ne!(first, input_hash("another-capture", &evidence));
}

#[test]
fn extract_error_codes_are_stable() {
    assert_eq!(ExtractError::Extractor("x".to_string()).code(), "extractor");
    assert_eq!(ExtractError::Storage("x".to_string()).code(), "storage");
    assert_eq!(
        ExtractError::Validation("x".to_string()).code(),
        "validation"
    );
}

#[test]
fn skipped_assessment_records_consent_and_the_same_input_hash() {
    let evidence = evidence(DURABLE[0]);
    let store = FakeStore::with(evidence.clone());

    record_skipped_assessment(&store, &evidence.capture_id, &RunContext::for_tests())
        .expect("skipped assessment is recorded");

    let rows = store.assessments();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].outcome, AssessmentOutcome::Skipped);
    assert_eq!(rows[0].error_code.as_deref(), Some("consent"));
    assert_eq!((rows[0].candidates, rows[0].inserted), (0, 0));
    assert_eq!(
        rows[0].input_hash,
        input_hash(&evidence.capture_id, &evidence),
        "the skipped row must carry the same input hash as a run would"
    );
    assert!(store.records().is_empty(), "skipped writes no candidate");
}

#[test]
fn missing_capture_is_a_storage_error_without_assessment() {
    let absent = FakeStore::default();

    let result = run_extraction(
        &absent,
        &FakeCandidateExtractor,
        "capture-gone",
        &RunContext::for_tests(),
    );
    let error = result.expect_err("a missing capture must fail the run");
    assert_eq!(error.code(), "storage");
    assert!(
        absent.assessments().is_empty(),
        "no assessment row may be written for a missing capture"
    );
    assert!(absent.records().is_empty());

    let skipped = FakeStore::default();
    let result = record_skipped_assessment(&skipped, "capture-gone", &RunContext::for_tests());
    let error = result.expect_err("a missing capture must fail the skipped path too");
    assert_eq!(error.code(), "storage");
    assert!(skipped.assessments().is_empty());
}

#[test]
fn provider_setup_failures_are_recorded_with_stable_codes() {
    for (variant, expected) in [
        (ProviderSetupError::MissingSecret, "secret"),
        (ProviderSetupError::Keystore, "keystore"),
        (ProviderSetupError::InvalidConfig, "provider_config"),
    ] {
        assert_eq!(variant.code(), expected);

        let evidence = evidence(DURABLE[0]);
        let store = FakeStore::with(evidence.clone());
        let failure = fail_provider_setup(
            &store,
            &evidence.capture_id,
            &RunContext::for_tests(),
            variant,
        )
        .unwrap_or_else(|error| panic!("{expected}: {error}"));

        assert_eq!(failure, application::jobs::JobFailure::Failed);
        let rows = store.assessments();
        assert_eq!(rows.len(), 1, "{expected}: exactly one provenance row");
        assert_eq!(rows[0].outcome, AssessmentOutcome::Failed);
        assert_eq!(rows[0].error_code.as_deref(), Some(expected));
        assert_eq!((rows[0].candidates, rows[0].inserted), (0, 0));
        assert_eq!(
            rows[0].input_hash,
            input_hash(&evidence.capture_id, &evidence),
            "{expected}: the hash must match a real run"
        );
        assert!(store.records().is_empty(), "{expected}: no candidate rows");
    }
}

#[test]
fn profile_unavailable_failure_records_a_safe_fallback_context() {
    assert_eq!(ProviderSetupError::ProfileUnavailable.code(), "profile");

    let evidence = evidence(DURABLE[0]);
    let store = FakeStore::with(evidence.clone());
    let context = detailed_context();
    let failure = fail_provider_setup(
        &store,
        &evidence.capture_id,
        &context,
        ProviderSetupError::ProfileUnavailable,
    )
    .expect("an unreadable profile is still recorded");

    assert_eq!(failure, application::jobs::JobFailure::Failed);
    let rows = store.assessments();
    assert_eq!(rows.len(), 1, "exactly one provenance row");
    assert_eq!(rows[0].outcome, AssessmentOutcome::Failed);
    assert_eq!(rows[0].error_code.as_deref(), Some("profile"));
    assert_eq!(rows[0].profile_id, "unavailable");
    assert_eq!(rows[0].adapter, "unknown");
    assert_eq!(rows[0].model, None);
    assert_eq!(rows[0].policy, "{\"profile\":\"unavailable\"}");
    assert_eq!(rows[0].consent_preview_hash, None);
    assert_eq!(rows[0].job_id.as_deref(), Some("job-3"));
    assert_eq!((rows[0].candidates, rows[0].inserted), (0, 0));
    assert_eq!(
        rows[0].input_hash,
        input_hash(&evidence.capture_id, &evidence),
        "the hash must match a real run"
    );
    assert!(store.records().is_empty(), "no candidate rows");
}

#[test]
fn connection_test_accepts_a_valid_structured_answer() {
    let evidence = connection_test_evidence();
    assert!(
        !filter_relevant(&evidence).is_empty(),
        "the synthetic evidence must dispatch the extractor like a real turn"
    );
    let report = run_connection_test(&FakeCandidateExtractor).expect("connection test");
    assert_eq!(report.proposals, 1);
}

#[test]
fn connection_test_reports_provider_and_contract_failures() {
    assert!(matches!(
        run_connection_test(&FailingExtractor),
        Err(ExtractError::Extractor(_))
    ));
    let invalid = BadExtractor {
        mutate: |proposal| proposal.evidence_refs = vec!["unknown-ref".to_string()],
    };
    assert_eq!(
        run_connection_test(&invalid).map_err(|error| error.code()),
        Err("validation")
    );
}

#[test]
fn connection_test_evidence_is_synthetic() {
    let evidence = connection_test_evidence();
    assert_eq!(evidence.capture_id, "connection-test");
    assert!(evidence.adapter.is_none() && evidence.session_id.is_none());
    for artifact in &evidence.artifacts {
        assert!(
            artifact.content.contains("sintético")
                || artifact.content.contains("exemplo")
                || artifact.content.contains("Exemplo"),
            "only fixed fictitious content is sent"
        );
    }
}

/// Returns an implementation detail and a decision.
struct DetailAndDecision;

impl CandidateExtractor for DetailAndDecision {
    fn extract(
        &self,
        input: &DecisionEvidence,
        signals: &[RelevanceSignal],
    ) -> Result<Vec<CandidateProposal>, ExtractError> {
        let mut detail = valid_proposal(input, signals);
        detail.kind = CandidateKind::Detail;
        detail.question = "Como validar o campo?".into();
        let mut decision = valid_proposal(input, signals);
        decision.significance = 0.9;
        decision.criteria = vec!["data_or_contract".into()];
        Ok(vec![detail, decision])
    }
}

#[test]
fn implementation_details_are_never_persisted() {
    let evidence = evidence(DURABLE[0]);
    let store = FakeStore::with(evidence.clone());
    let report = run_extraction(
        &store,
        &DetailAndDecision,
        &evidence.capture_id,
        &RunContext::for_tests(),
    )
    .expect("extraction");
    assert_eq!(report.inserted, 1);
    let record = &store.records()[0];
    assert_eq!(record.kind, "decision");
    assert!((record.significance - 0.9).abs() < f64::EPSILON);
    assert_eq!(record.criteria, "[\"data_or_contract\"]");
}
