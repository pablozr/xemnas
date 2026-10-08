//! Automatic review against the real database: the rules settle the plain
//! cases for free, the AI is asked once for the rest and never too often, and
//! everything it decides is in a ledger.

mod support;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use application::adoption::{Adoption, AdoptionApi};
use application::analysis::ExtractorFactory;
use application::auto_approval::{
    Approvals, ApprovalsApi, By, ItemKind, Mode, ReviewError, Verdict, BATCH,
};
use application::extract::{
    CandidateExtractor, CandidateProposal, DecisionCandidateRecord, DecisionEvidence, ExtractError,
    ExtractionStore, RelevanceSignal,
};
use application::inbox::{CandidateStatus, InboxStore};
use application::overview::{JsonValue, StructuredModel};
use application::profile::{
    build_preview, grant_consent, offline_default_profile, AiProfile, AiSettings, ProfileError,
    ProfileKind, ProfileStore, SecretStore,
};
use storage_sqlite::SqliteStore;

const PROJECT: &str = "p1";
const NOW: &str = "2026-03-01T10:00:00Z";

struct Profiles(Mutex<AiProfile>);

impl ProfileStore for Profiles {
    fn load(&self) -> Result<Option<AiProfile>, ProfileError> {
        Ok(Some(self.0.lock().expect("lock").clone()))
    }
    fn save(&self, profile: &AiProfile) -> Result<(), ProfileError> {
        *self.0.lock().expect("lock") = profile.clone();
        Ok(())
    }
}

struct NoSecrets;

impl SecretStore for NoSecrets {
    fn set_secret(&self, _: &str, _: &str) -> Result<(), ProfileError> {
        Ok(())
    }
    fn get_secret(&self, _: &str) -> Result<Option<String>, ProfileError> {
        Ok(None)
    }
    fn delete_secret(&self, _: &str) -> Result<(), ProfileError> {
        Ok(())
    }
}

fn consented() -> AiProfile {
    let profile = AiProfile {
        kind: ProfileKind::OpenAiCompatible,
        model: "local".into(),
        endpoint: Some("http://127.0.0.1:9/v1".into()),
        ..offline_default_profile()
    };
    grant_consent(
        &profile,
        &build_preview(&profile),
        "2026-01-01T00:00:00Z",
        true,
    )
    .expect("consent")
}

/// What the fake AI was asked, and what it answers.
#[derive(Clone)]
struct Judge {
    answer: Arc<Mutex<Result<String, String>>>,
    calls: Arc<AtomicUsize>,
    asked: Arc<Mutex<Vec<String>>>,
}

impl Judge {
    fn answering(answer: &str) -> Self {
        Self {
            answer: Arc::new(Mutex::new(Ok(answer.to_owned()))),
            calls: Arc::new(AtomicUsize::new(0)),
            asked: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn failing() -> Self {
        let judge = Self::answering("");
        *judge.answer.lock().expect("lock") = Err("fora do ar".into());
        judge
    }
}

impl CandidateExtractor for Judge {
    fn extract(
        &self,
        _: &DecisionEvidence,
        _: &[RelevanceSignal],
    ) -> Result<Vec<CandidateProposal>, ExtractError> {
        Ok(Vec::new())
    }
}

impl StructuredModel for Judge {
    fn complete(
        &self,
        _: &str,
        user: &str,
        schema_name: &str,
        _: &JsonValue,
    ) -> Result<String, ExtractError> {
        assert_eq!(schema_name, "approval_verdicts");
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.asked.lock().expect("lock").push(user.to_owned());
        self.answer
            .lock()
            .expect("lock")
            .clone()
            .map_err(|_| ExtractError::Extractor("fora do ar".into()))
    }
}

struct Factory(Judge);

impl ExtractorFactory for Factory {
    type Extractor = Judge;
    fn external(&self, _: &AiProfile, _: String) -> Result<Judge, ExtractError> {
        Ok(self.0.clone())
    }
}

type Review = Approvals<SqliteStore, Profiles, NoSecrets, Factory>;

fn review_with(store: &SqliteStore, profile: AiProfile, judge: &Judge) -> Review {
    let adoption: Arc<dyn AdoptionApi> = Arc::new(Adoption::new(store.clone()));
    Approvals::new(
        store.clone(),
        adoption,
        AiSettings::new(Profiles(Mutex::new(profile)), NoSecrets),
        Factory(judge.clone()),
    )
}

fn candidate(
    id: &str,
    status: &str,
    confidence: f64,
    question: &str,
    created_at: &str,
) -> DecisionCandidateRecord {
    DecisionCandidateRecord {
        id: id.to_string(),
        project_id: PROJECT.to_string(),
        capture_id: format!("capture-{PROJECT}"),
        status: status.to_string(),
        question: question.to_string(),
        choice: "escolha".to_string(),
        rationale: "motivo".to_string(),
        signals: "[\"public_contract\"]".to_string(),
        confidence,
        confidence_reason: "sintético".to_string(),
        evidence_refs: format!("[\"art-{PROJECT}\"]"),
        diff_summary: "{\"files\":[\"src/lib.rs\"],\"artifacts\":1}".to_string(),
        dedup_hash: format!("dedup-{id}"),
        created_at: created_at.to_string(),
        updated_at: created_at.to_string(),
        kind: "decision".to_string(),
        significance: 1.0,
        criteria: "[]".to_string(),
        qualifiers: "[]".into(),
    }
}

/// A decided history in which the extractor's confidence predicts what the
/// person keeps (confident ones kept, unsure ones dismissed): the only case
/// in which the free rules may accept on confidence.
fn calibrated_history() -> Vec<DecisionCandidateRecord> {
    (0..32)
        .map(|index| {
            let kept = index % 2 == 0;
            candidate(
                &format!("history-{index}"),
                if kept { "accepted" } else { "dismissed" },
                if kept { 0.92 } else { 0.35 },
                &format!("historico {index} assunto zeta{index}"),
                "2026-01-01T00:00:00Z",
            )
        })
        .collect()
}

fn old(id: &str, confidence: f64, question: &str) -> DecisionCandidateRecord {
    candidate(id, "pending", confidence, question, "2026-02-01T00:00:00Z")
}

fn status_of(store: &SqliteStore, id: &str) -> CandidateStatus {
    store.get(id).expect("get").expect("row").status
}

#[test]
fn nothing_happens_in_manual_mode() {
    let test = support::open("review-manual", &[PROJECT]);
    let judge = Judge::answering("{}");
    let review = review_with(&test.store, consented(), &judge);
    test.store
        .insert_candidates(&[old("a", 0.95, "como versionar revisoes")])
        .expect("pending");
    assert!(!review.status().expect("status").automatic);
    assert_eq!(
        review.run_at(PROJECT, NOW).expect("run"),
        Default::default()
    );
    assert_eq!(status_of(&test.store, "a"), CandidateStatus::Pending);
}

#[test]
fn the_rules_settle_the_plain_cases_and_one_batched_call_settles_the_rest() {
    let test = support::open("review-batch", &[PROJECT]);
    let judge = Judge::answering(
        r#"{"verdicts":[
          {"id":"I1","verdict":"accept","reason":"correta e útil"},
          {"id":"I2","verdict":"discard","reason":"trivial"},
          {"id":"I3","verdict":"human","reason":"muda o comportamento dos agentes"}]}"#,
    );
    let review = review_with(&test.store, consented(), &judge);
    review.set_mode(Mode::Automatic).expect("on");
    test.store
        .insert_candidates(&calibrated_history())
        .expect("history");
    test.store
        .insert_candidates(&[
            candidate(
                "seen",
                "accepted",
                0.9,
                "onde guardar credenciais externas",
                "2026-01-01T00:00:00Z",
            ),
            // Plain: confident, sourced, nothing like it recorded.
            old("plain", 0.95, "como versionar as revisoes passadas"),
            // A repeat of a recorded question.
            old("twin", 0.95, "onde guardar credenciais externas"),
            // Middling confidence: ambiguous, oldest first.
            candidate(
                "amb-1",
                "pending",
                0.60,
                "qual formato exportar relatorios",
                "2026-02-01T00:00:01Z",
            ),
            candidate(
                "amb-2",
                "pending",
                0.65,
                "quando limpar arquivos temporarios",
                "2026-02-01T00:00:02Z",
            ),
            candidate(
                "amb-3",
                "pending",
                0.70,
                "onde registrar rejeicoes antigas",
                "2026-02-01T00:00:03Z",
            ),
        ])
        .expect("pending");

    let report = review.run_at(PROJECT, NOW).expect("pass");
    assert!(report.asked);
    assert_eq!(
        (report.accepted, report.discarded, report.left),
        (2, 2, 1),
        "plain and amb-1 accepted, twin and amb-2 discarded, amb-3 for the person"
    );
    assert_eq!(
        judge.calls.load(Ordering::SeqCst),
        1,
        "one call for the three"
    );
    let asked = judge.asked.lock().expect("lock")[0].clone();
    assert!(asked.contains("qual formato exportar relatorios"));
    assert!(
        !asked.contains("como versionar as revisoes passadas"),
        "the plain one is not sent"
    );

    assert_eq!(status_of(&test.store, "plain"), CandidateStatus::Accepted);
    assert_eq!(status_of(&test.store, "twin"), CandidateStatus::Dismissed);
    assert_eq!(status_of(&test.store, "amb-1"), CandidateStatus::Accepted);
    assert_eq!(status_of(&test.store, "amb-2"), CandidateStatus::Dismissed);
    assert_eq!(status_of(&test.store, "amb-3"), CandidateStatus::Pending);

    let ledger = review.ledger(PROJECT).expect("ledger");
    assert_eq!(ledger.len(), 5);
    let entry = |id: &str| {
        ledger
            .iter()
            .find(|entry| entry.item_id == id)
            .expect("entry")
            .clone()
    };
    assert_eq!(
        (entry("plain").verdict, entry("plain").by),
        (Verdict::Accepted, By::Rules)
    );
    assert_eq!(
        (entry("twin").verdict, entry("twin").by),
        (Verdict::Discarded, By::Rules)
    );
    assert_eq!(
        (entry("amb-1").verdict, entry("amb-1").by),
        (Verdict::Accepted, By::Ai)
    );
    assert!(
        entry("plain").result_id.is_some(),
        "the decision it created"
    );
    assert_eq!(entry("amb-3").verdict, Verdict::NeedsHuman);
    assert_eq!(entry("amb-3").reason, "muda o comportamento dos agentes");

    // Nothing is asked twice, and a pass right after asks nothing.
    let again = review
        .run_at(PROJECT, "2026-03-01T10:05:00Z")
        .expect("again");
    assert!(!again.asked && !again.changed());
    assert_eq!(judge.calls.load(Ordering::SeqCst), 1);
}

#[test]
fn a_lone_ambiguous_item_is_asked_at_once() {
    let test = support::open("review-wait", &[PROJECT]);
    let judge = Judge::answering(r#"{"verdicts":[{"id":"I1","verdict":"accept","reason":"ok"}]}"#);
    let review = review_with(&test.store, consented(), &judge);
    review.set_mode(Mode::Automatic).expect("on");
    test.store
        .insert_candidates(&[candidate(
            "lone",
            "pending",
            0.6,
            "qual formato exportar relatorios",
            "2026-03-01T09:59:00Z",
        )])
        .expect("pending");
    let pass = review
        .run_at(PROJECT, "2026-03-01T10:00:00Z")
        .expect("pass");
    assert!(pass.asked, "nothing waits for company");
    assert_eq!(status_of(&test.store, "lone"), CandidateStatus::Accepted);
}

#[test]
fn a_failed_call_pauses_the_next_and_working_calls_drain_the_backlog() {
    let test = support::open("review-limits", &[PROJECT]);
    let judge = Judge::failing();
    let review = review_with(&test.store, consented(), &judge);
    review.set_mode(Mode::Automatic).expect("on");
    let rows: Vec<_> = (0..2 * BATCH + 1)
        .map(|at| {
            old(
                &format!("amb-{at}"),
                0.6,
                &format!("assunto{at} ambiguo{at} sem{at} resposta{at}"),
            )
        })
        .collect();
    test.store.insert_candidates(&rows).expect("pending");

    let first = review.run_at(PROJECT, NOW);
    assert!(
        matches!(first, Err(ReviewError::Provider(_))),
        "the failure is reported"
    );
    let soon = review
        .run_at(PROJECT, "2026-03-01T10:10:00Z")
        .expect("soon");
    assert!(!soon.asked, "a failed call pauses the next one");
    assert_eq!(judge.calls.load(Ordering::SeqCst), 1);
    assert_eq!(status_of(&test.store, "amb-0"), CandidateStatus::Pending);

    // Back up: every pass asks at once, a batch each, until nothing waits.
    *judge.answer.lock().expect("lock") = Ok(r#"{"verdicts":[]}"#.to_owned());
    let later = "2026-03-01T10:30:00Z";
    for _ in 0..3 {
        assert!(review.run_at(PROJECT, later).expect("pass").asked);
    }
    assert!(!review.run_at(PROJECT, later).expect("drained").asked);
    assert_eq!(judge.calls.load(Ordering::SeqCst), 4);
}

#[test]
fn without_a_calibrated_history_a_confident_candidate_goes_to_the_judge() {
    let test = support::open("review-uncalibrated", &[PROJECT]);
    let judge = Judge::answering(
        r#"{"verdicts":[
          {"id":"I1","verdict":"human","reason":"muda o que os agentes recebem"},
          {"id":"I2","verdict":"accept","reason":"correta e útil"},
          {"id":"I3","verdict":"discard","reason":"trivial"}]}"#,
    );
    let review = review_with(&test.store, consented(), &judge);
    review.set_mode(Mode::Automatic).expect("on");
    test.store
        .insert_candidates(&[
            old("plain", 0.97, "como versionar as revisoes passadas"),
            old("amb-1", 0.6, "qual formato exportar relatorios"),
            old("amb-2", 0.6, "quando limpar arquivos temporarios"),
        ])
        .expect("pending");
    let report = review.run_at(PROJECT, NOW).expect("pass");
    assert!(
        report.asked,
        "nothing is accepted on an uncalibrated confidence"
    );
    assert_eq!(judge.calls.load(Ordering::SeqCst), 1);
    assert_ne!(
        status_of(&test.store, "plain"),
        CandidateStatus::Accepted,
        "the judge, not the confidence, settles it"
    );
}

#[test]
fn without_an_enabled_provider_only_the_rules_act() {
    let test = support::open("review-offline", &[PROJECT]);
    let judge = Judge::answering("{}");
    let review = review_with(&test.store, offline_default_profile(), &judge);
    review.set_mode(Mode::Automatic).expect("on");
    assert!(!review.status().expect("status").judge);
    test.store
        .insert_candidates(&calibrated_history())
        .expect("history");
    test.store
        .insert_candidates(&[
            old("plain", 0.95, "como versionar as revisoes passadas"),
            old("amb-1", 0.6, "qual formato exportar relatorios"),
            old("amb-2", 0.6, "quando limpar arquivos temporarios"),
            old("amb-3", 0.6, "onde registrar rejeicoes antigas"),
        ])
        .expect("pending");
    let report = review.run_at(PROJECT, NOW).expect("pass");
    assert_eq!((report.accepted, report.asked), (1, false));
    assert_eq!(judge.calls.load(Ordering::SeqCst), 0);
    assert_eq!(status_of(&test.store, "plain"), CandidateStatus::Accepted);
    assert_eq!(status_of(&test.store, "amb-1"), CandidateStatus::Pending);
}

#[test]
fn a_discarded_candidate_can_be_put_back_and_nothing_else_can() {
    let test = support::open("review-undo", &[PROJECT]);
    let judge = Judge::answering("{}");
    let review = review_with(&test.store, consented(), &judge);
    review.set_mode(Mode::Automatic).expect("on");
    test.store
        .insert_candidates(&[
            candidate(
                "seen",
                "accepted",
                0.9,
                "onde guardar credenciais externas",
                "2026-01-01T00:00:00Z",
            ),
            old("twin", 0.95, "onde guardar credenciais externas"),
            old("plain", 0.95, "como versionar as revisoes passadas"),
        ])
        .expect("pending");
    review.run_at(PROJECT, NOW).expect("pass");
    assert_eq!(status_of(&test.store, "twin"), CandidateStatus::Dismissed);

    review.undo(ItemKind::Candidate, "twin").expect("put back");
    assert_eq!(status_of(&test.store, "twin"), CandidateStatus::Pending);
    let ledger = review.ledger(PROJECT).expect("ledger");
    assert!(ledger
        .iter()
        .find(|entry| entry.item_id == "twin")
        .is_some_and(|entry| entry.undone_at.is_some()));
    assert_eq!(
        review.undo(ItemKind::Candidate, "twin"),
        Err(ReviewError::NotUndoable),
        "already back"
    );
    assert_eq!(
        review.undo(ItemKind::Candidate, "plain"),
        Err(ReviewError::NotUndoable),
        "an accepted one is edited or ended in its own page"
    );
    assert_eq!(
        review.undo(ItemKind::Claim, "x"),
        Err(ReviewError::NotUndoable)
    );
}

#[test]
fn a_link_found_by_mention_is_asked_with_its_quote_and_a_file_link_is_accepted() {
    use application::graph::{KnowledgeGraph, NewEntity};
    use domain::entities::EntityKind;

    let test = support::open("review-links", &[PROJECT]);
    let graph = KnowledgeGraph::new(test.store.clone());
    for (name, pattern) in [("core", "crates/core/**"), ("storage", "crates/storage/**")] {
        graph
            .create_entity(NewEntity {
                project_id: PROJECT.into(),
                kind: Some(EntityKind::Component),
                name: name.into(),
                patterns: vec![pattern.into()],
                ..NewEntity::default()
            })
            .expect("component");
    }
    // From an ADR: only the document was touched, the text names the core.
    support::decision_with_diff(
        &test.store,
        PROJECT,
        "adr",
        "Como o core grava os eventos sem perder nenhum?",
        &["docs/adr/0003-outbox.md"],
        "",
    );
    support::decision_with_diff(
        &test.store,
        PROJECT,
        "code",
        "Onde guardar os arquivos temporarios?",
        &["crates/storage/src/lib.rs"],
        "",
    );
    assert_eq!(
        graph
            .refresh_suggestions(PROJECT)
            .expect("refresh")
            .new_edges,
        2
    );

    let judge = Judge::answering(
        r#"{"verdicts":[{"id":"I1","verdict":"human","reason":"menção de passagem"}]}"#,
    );
    let review = review_with(&test.store, consented(), &judge);
    review.set_mode(Mode::Automatic).expect("on");
    // Long after the suggestions were derived: a lone item does not wait.
    let report = review
        .run_at(PROJECT, "2099-01-01T00:00:00Z")
        .expect("pass");
    assert!(report.asked);
    assert_eq!(
        (report.accepted, report.left),
        (1, 1),
        "file accepted, mention asked"
    );
    let asked = judge.asked.lock().expect("lock")[0].clone();
    assert!(
        asked.contains("Citação:") && asked.contains("o core grava"),
        "{asked}"
    );
    assert!(
        !asked.contains("crates/storage"),
        "the file link is not sent: {asked}"
    );

    let ledger = review.ledger(PROJECT).expect("ledger");
    let links: Vec<_> = ledger
        .iter()
        .filter(|entry| entry.kind == ItemKind::Link)
        .map(|entry| (entry.verdict, entry.by))
        .collect();
    assert_eq!(links.len(), 2);
    assert!(links.contains(&(Verdict::Accepted, By::Rules)));
    assert!(links.contains(&(Verdict::NeedsHuman, By::Ai)));
    let pending = graph.suggestions(PROJECT).expect("suggestions");
    assert_eq!(pending.len(), 1, "the mention waits for the person");
    assert!(pending[0]
        .reason
        .starts_with(application::graph::MENTION_REASON));
}

#[test]
fn a_link_the_ai_proposed_is_judged_with_its_quote_and_reason() {
    use application::graph::{ai_link_reason, EdgeRecord, GraphStore, KnowledgeGraph, NewEntity};
    use domain::entities::{EdgeKind, EdgeOrigin, EntityKind, NodeKind};

    let test = support::open("review-ai-links", &[PROJECT]);
    let graph = KnowledgeGraph::new(test.store.clone());
    let core = graph
        .create_entity(NewEntity {
            project_id: PROJECT.into(),
            kind: Some(EntityKind::Component),
            name: "motor".into(),
            patterns: vec!["crates/core/**".into()],
            ..NewEntity::default()
        })
        .expect("component")
        .entity_id;
    let decision = support::decision_with_diff(
        &test.store,
        PROJECT,
        "adr",
        "Como o resultado de um turno é expresso?",
        &["docs/adr/0003-outcome.md"],
        "",
    );
    test.store
        .insert_edge(&EdgeRecord {
            edge_id: "ai-edge".into(),
            project_id: PROJECT.into(),
            kind: EdgeKind::Affects,
            source_kind: NodeKind::Decision,
            source_id: decision,
            entity_id: core,
            origin: EdgeOrigin::Derived,
            reason: ai_link_reason(
                "resultado de um turno",
                "A decisão rege o desfecho do motor.",
            ),
            created_at: NOW.into(),
            confirmed_at: None,
            invalidated_at: None,
            confirmed_by: None,
            invalidated_by: None,
        })
        .expect("proposed by the AI");

    let judge =
        Judge::answering(r#"{"verdicts":[{"id":"I1","verdict":"accept","reason":"rege"}]}"#);
    let review = review_with(&test.store, consented(), &judge);
    review.set_mode(Mode::Automatic).expect("on");
    let report = review
        .run_at(PROJECT, "2099-01-01T00:00:00Z")
        .expect("pass");
    assert!(
        report.asked,
        "an AI proposal is not plain: the judge is asked"
    );
    assert_eq!((report.accepted, report.left), (1, 0));
    let asked = judge.asked.lock().expect("lock")[0].clone();
    assert!(
        asked.contains("Citação proposta pela IA: resultado de um turno")
            && asked.contains("Motivo da IA: A decisão rege o desfecho do motor."),
        "{asked}"
    );
    assert!(graph.suggestions(PROJECT).expect("pending").is_empty());
    let edge = test
        .store
        .project_edges(PROJECT)
        .expect("edges")
        .into_iter()
        .find(|edge| edge.edge_id == "ai-edge")
        .expect("edge");
    assert!(edge.confirmed_at.is_some(), "the verdict confirmed it");
}

#[test]
fn an_unconfirmable_suggestion_is_left_for_the_person_and_not_asked_again() {
    use application::claim_suggestions::{ClaimSuggestionRecord, ClaimSuggestionStore};
    use application::decisions::{DecisionEdits, Decisions};
    use domain::claims::ClaimKind;

    let test = support::open("review-unconfirmable", &[PROJECT]);
    let id = support::decision(&test.store, PROJECT, "source", "Pergunta", "Escolha");
    test.store
        .insert_claim_suggestion(&ClaimSuggestionRecord {
            suggestion_id: "legacy".into(),
            project_id: PROJECT.into(),
            decision_id: id.clone(),
            kind: ClaimKind::Constraint,
            statement: "Regra antiga".into(),
            quote: "Escolha".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            qualifiers: "[]".into(),
            inherited_scope: "[]".into(),
            source_version: Some(1),
        })
        .expect("suggestion");
    Decisions::new(test.store.clone())
        .revise(
            &id,
            DecisionEdits {
                choice: Some("Outra escolha".into()),
                ..DecisionEdits::default()
            },
        )
        .expect("revise");
    // A suggestion from before the version was recorded, whose quote is gone.
    rusqlite::Connection::open(test.root.join("app.db"))
        .expect("raw")
        .execute("UPDATE claim_suggestions SET source_version = NULL", [])
        .expect("legacy row");

    let judge = Judge::answering(r#"{"verdicts":[{"id":"I1","verdict":"accept","reason":"ok"}]}"#);
    let review = review_with(&test.store, consented(), &judge);
    review.set_mode(Mode::Automatic).expect("on");
    let first = review
        .run_at(PROJECT, "2099-01-01T00:00:00Z")
        .expect("pass");
    assert!(first.asked);
    assert_eq!((first.accepted, first.left), (0, 1));
    let ledger = review.ledger(PROJECT).expect("ledger");
    let entry = ledger
        .iter()
        .find(|entry| entry.kind == ItemKind::Claim)
        .expect("recorded");
    assert_eq!(entry.verdict, Verdict::NeedsHuman);
    let calls = judge.calls.load(Ordering::SeqCst);

    review
        .run_at(PROJECT, "2099-01-02T00:00:00Z")
        .expect("second pass");
    assert_eq!(judge.calls.load(Ordering::SeqCst), calls, "not asked again");
}

fn two_conflicting(test: &support::TestStore) {
    test.store
        .insert_candidates(&[
            candidate(
                "fixed",
                "pending",
                0.6,
                "limiares fixos para o juiz",
                "2026-02-01T00:00:01Z",
            ),
            candidate(
                "bands",
                "pending",
                0.6,
                "faixas de probabilidade para o juiz",
                "2026-02-01T00:00:02Z",
            ),
        ])
        .expect("pending");
}

#[test]
fn the_judge_names_the_other_side_and_no_id_reaches_the_reason() {
    use application::auto_approval::{Conflict, ConflictKind};

    let test = support::open("review-conflict", &[PROJECT]);
    two_conflicting(&test);
    let judge = Judge::answering(
        r#"{"verdicts":[
          {"id":"I1","verdict":"human","reason":"Contradiz I2 sobre limiares","conflicts_with":"I2"},
          {"id":"I2","verdict":"human","reason":"Vale só se I9 sumir","conflicts_with":null}]}"#,
    );
    let review = review_with(&test.store, consented(), &judge);
    review.set_mode(Mode::Automatic).expect("on");
    let report = review
        .run_at(PROJECT, "2026-03-01T10:00:00Z")
        .expect("pass");
    assert_eq!((report.accepted, report.discarded, report.left), (0, 0, 2));
    assert_eq!(judge.calls.load(Ordering::SeqCst), 1, "still one call");

    let ledger = review.ledger(PROJECT).expect("ledger");
    let entry = |id: &str| ledger.iter().find(|entry| entry.item_id == id).expect(id);
    let fixed = entry("fixed");
    assert_eq!(
        fixed.conflicts_with,
        Some(Conflict {
            kind: ConflictKind::Candidate,
            id: "bands".into()
        })
    );
    assert_eq!(
        entry("bands").conflicts_with,
        Some(Conflict {
            kind: ConflictKind::Candidate,
            id: "fixed".into()
        }),
        "the other side shows the same pair"
    );
    assert!(
        fixed
            .reason
            .contains("\"faixas de probabilidade para o juiz\""),
        "{}",
        fixed.reason
    );
    assert_eq!(entry("bands").reason, "Vale só se outro item sumir");
}

#[test]
fn a_decision_in_force_that_resembles_the_candidate_is_named_to_the_judge() {
    use application::auto_approval::{Conflict, ConflictKind};

    let test = support::open("review-in-force", &[PROJECT]);
    let in_force = support::decision(
        &test.store,
        PROJECT,
        "bands",
        "faixas de probabilidade para o juiz",
        "usar três faixas",
    );
    test.store
        .insert_candidates(&[candidate(
            "fixed",
            "pending",
            0.6,
            "limiares fixos para o juiz",
            "2026-02-01T00:00:01Z",
        )])
        .expect("pending");
    let judge = Judge::answering(
        r#"{"verdicts":[{"id":"I1","verdict":"human","reason":"Contradiz D1","conflicts_with":"D1"}]}"#,
    );
    let review = review_with(&test.store, consented(), &judge);
    review.set_mode(Mode::Automatic).expect("on");
    review
        .run_at(PROJECT, "2026-03-01T10:00:00Z")
        .expect("pass");

    let asked = judge.asked.lock().expect("lock")[0].clone();
    assert!(
        asked.contains("Parecidas em vigor: D1")
            && asked.contains("## Decisões em vigor")
            && asked.contains("D1 \"faixas de probabilidade para o juiz\" -> usar três faixas"),
        "{asked}"
    );
    let ledger = review.ledger(PROJECT).expect("ledger");
    let entry = ledger
        .iter()
        .find(|entry| entry.item_id == "fixed")
        .expect("entry");
    assert_eq!(
        entry.conflicts_with,
        Some(Conflict {
            kind: ConflictKind::Decision,
            id: in_force
        })
    );
    assert!(
        entry.reason.contains("faixas de probabilidade"),
        "{}",
        entry.reason
    );
}

#[test]
fn a_doubted_ai_link_is_discarded_and_a_doubted_mention_still_waits() {
    use application::graph::{ai_link_reason, EdgeRecord, GraphStore, KnowledgeGraph, NewEntity};
    use domain::entities::{EdgeKind, EdgeOrigin, EntityKind, NodeKind};

    let test = support::open("review-doubted-links", &[PROJECT]);
    let graph = KnowledgeGraph::new(test.store.clone());
    let core = graph
        .create_entity(NewEntity {
            project_id: PROJECT.into(),
            kind: Some(EntityKind::Component),
            name: "core".into(),
            patterns: vec!["crates/core/**".into()],
            ..NewEntity::default()
        })
        .expect("component")
        .entity_id;
    support::decision_with_diff(
        &test.store,
        PROJECT,
        "mention",
        "Como o core grava os eventos sem perder nenhum?",
        &["docs/adr/0003-outbox.md"],
        "",
    );
    graph.refresh_suggestions(PROJECT).expect("refresh");
    let decision = support::decision_with_diff(
        &test.store,
        PROJECT,
        "ai",
        "Como o resultado de um turno é expresso?",
        &["docs/adr/0004-outcome.md"],
        "",
    );
    test.store
        .insert_edge(&EdgeRecord {
            edge_id: "ai-edge".into(),
            project_id: PROJECT.into(),
            kind: EdgeKind::Affects,
            source_kind: NodeKind::Decision,
            source_id: decision,
            entity_id: core,
            origin: EdgeOrigin::Derived,
            reason: ai_link_reason("Como o resultado de um turno é expresso?", "Rege o motor."),
            created_at: NOW.into(),
            confirmed_at: None,
            invalidated_at: None,
            confirmed_by: None,
            invalidated_by: None,
        })
        .expect("proposed by the AI");

    let judge = Judge::answering(
        r#"{"verdicts":[
          {"id":"I1","verdict":"human","reason":"A citação é só a pergunta da decisão"},
          {"id":"I2","verdict":"human","reason":"A citação é só a pergunta da decisão"}]}"#,
    );
    let review = review_with(&test.store, consented(), &judge);
    review.set_mode(Mode::Automatic).expect("on");
    let report = review
        .run_at(PROJECT, "2099-01-01T00:00:00Z")
        .expect("pass");
    assert_eq!((report.accepted, report.discarded, report.left), (0, 1, 1));

    let ledger = review.ledger(PROJECT).expect("ledger");
    let ai = ledger
        .iter()
        .find(|entry| entry.item_id == "ai-edge")
        .expect("ai link entry");
    assert_eq!((ai.verdict, ai.by), (Verdict::Discarded, By::Ai));
    assert_eq!(ai.reason, "A citação é só a pergunta da decisão");
    let mention = ledger
        .iter()
        .find(|entry| entry.kind == ItemKind::Link && entry.item_id != "ai-edge")
        .expect("mention entry");
    assert_eq!(mention.verdict, Verdict::NeedsHuman);
    let pending = graph.suggestions(PROJECT).expect("suggestions");
    assert_eq!(pending.len(), 1, "only the mention waits for the person");
    assert!(pending[0]
        .reason
        .starts_with(application::graph::MENTION_REASON));
    let edge = test
        .store
        .project_edges(PROJECT)
        .expect("edges")
        .into_iter()
        .find(|edge| edge.edge_id == "ai-edge")
        .expect("edge");
    assert!(edge.invalidated_at.is_some(), "the discard dropped it");
}

mod resolution {
    use super::*;
    use application::auto_approval::{ApprovalStore, Conflict, ConflictKind, Entry};
    use application::conflicts::{Resolution, SideKind};
    use application::decisions::{
        DecisionQuery, DecisionStatus, DecisionStore, Decisions, StoredDecision,
    };
    use application::relations::DecisionRelations;
    use domain::relations::RelationKind;

    fn leave(store: &SqliteStore, id: &str, other: &Conflict) -> Entry {
        let entry = Entry {
            kind: ItemKind::Candidate,
            item_id: id.into(),
            project_id: PROJECT.into(),
            verdict: Verdict::NeedsHuman,
            by: By::Ai,
            reason: "Contradiz a outra".into(),
            title: id.into(),
            result_id: None,
            created_at: NOW.into(),
            undone_at: None,
            conflicts_with: Some(other.clone()),
        };
        store.record_review(&entry).expect("ledger");
        entry
    }

    fn against(kind: ConflictKind, id: &str) -> Conflict {
        Conflict {
            kind,
            id: id.into(),
        }
    }

    fn decisions(store: &SqliteStore, status: DecisionStatus) -> Vec<StoredDecision> {
        DecisionStore::list(
            store,
            &DecisionQuery {
                project_id: Some(PROJECT.into()),
                statuses: vec![status],
                limit: 50,
                before: None,
            },
        )
        .expect("decisions")
    }

    fn from_candidate(store: &SqliteStore, candidate: &str) -> StoredDecision {
        decisions(store, DecisionStatus::Accepted)
            .into_iter()
            .find(|decision| decision.candidate_id == candidate)
            .expect("the candidate became a decision")
    }

    struct Fixture {
        test: support::TestStore,
        review: Review,
        in_force: String,
    }

    /// A decision in force and a pending candidate that contradicts it.
    fn fixture(tag: &str) -> Fixture {
        let test = support::open(tag, &[PROJECT]);
        let in_force = support::decision(
            &test.store,
            PROJECT,
            "old",
            "faixas de probabilidade para o juiz",
            "usar três faixas",
        );
        test.store
            .insert_candidates(&[candidate(
                "new",
                "pending",
                0.6,
                "limiares fixos para o juiz",
                "2026-02-01T00:00:01Z",
            )])
            .expect("pending");
        let review = review_with(&test.store, consented(), &Judge::answering("{}"));
        Fixture {
            test,
            review,
            in_force,
        }
    }

    #[test]
    fn the_view_shows_both_sides_and_the_judges_sentence() {
        let fx = fixture("conflict-view");
        let entry = leave(
            &fx.test.store,
            "new",
            &against(ConflictKind::Decision, &fx.in_force),
        );
        let view = fx.review.conflict(&entry).expect("view").expect("stands");
        assert_eq!(view.reason, "Contradiz a outra");
        assert!(view.can_replace);
        assert_eq!(
            (
                view.this.in_force,
                view.this.kind,
                view.this.question.as_str()
            ),
            (false, SideKind::Decision, "limiares fixos para o juiz")
        );
        assert_eq!(
            (view.other.in_force, view.other.choice.as_str()),
            (true, "usar três faixas")
        );
        assert!(!view.other.created_at.is_empty());
    }

    #[test]
    fn keeping_this_one_supersedes_the_decision_in_force() {
        let fx = fixture("conflict-keep-this");
        let other = against(ConflictKind::Decision, &fx.in_force);
        fx.review
            .resolve("new", &other, &Resolution::KeepThis)
            .expect("resolve");
        assert_eq!(status_of(&fx.test.store, "new"), CandidateStatus::Accepted);
        let new = from_candidate(&fx.test.store, "new");
        assert_eq!(
            decisions(&fx.test.store, DecisionStatus::Superseded)[0].decision_id,
            fx.in_force
        );
        let relations = DecisionRelations::new(fx.test.store.clone())
            .of(&new.decision_id)
            .expect("relations");
        assert!(relations
            .iter()
            .any(|relation| relation.kind == RelationKind::Supersedes
                && relation.other_id == fx.in_force));
        // Resolved: the conflict no longer stands.
        let entry = leave(&fx.test.store, "new", &other);
        assert_eq!(fx.review.conflict(&entry).expect("view"), None);
        assert_eq!(
            fx.review.resolve("new", &other, &Resolution::KeepThis),
            Err(ReviewError::Stale)
        );
    }

    #[test]
    fn keeping_the_other_one_rejects_this_and_leaves_the_decision_alone() {
        let fx = fixture("conflict-keep-other");
        let other = against(ConflictKind::Decision, &fx.in_force);
        fx.review
            .resolve("new", &other, &Resolution::KeepOther)
            .expect("resolve");
        assert_eq!(status_of(&fx.test.store, "new"), CandidateStatus::Dismissed);
        assert_eq!(decisions(&fx.test.store, DecisionStatus::Accepted).len(), 1);
        assert!(decisions(&fx.test.store, DecisionStatus::Superseded).is_empty());
    }

    #[test]
    fn keeping_both_scopes_each_and_supersedes_nothing() {
        let fx = fixture("conflict-keep-both");
        let other = against(ConflictKind::Decision, &fx.in_force);
        let both = |this: &str, other: &str| Resolution::KeepBoth {
            this_scope: this.into(),
            other_scope: other.into(),
        };
        assert!(
            fx.review
                .resolve("new", &other, &both("  ", "só no módulo A"))
                .is_err(),
            "a scope is required for each"
        );
        assert_eq!(status_of(&fx.test.store, "new"), CandidateStatus::Pending);

        fx.review
            .resolve(
                "new",
                &other,
                &both("somente   na API pública", "somente no núcleo"),
            )
            .expect("resolve");
        assert_eq!(
            status_of(&fx.test.store, "new"),
            CandidateStatus::EditedAndAccepted
        );
        assert!(decisions(&fx.test.store, DecisionStatus::Superseded).is_empty());
        let decisions_api = Decisions::new(fx.test.store.clone());
        let new = decisions_api
            .detail(&from_candidate(&fx.test.store, "new").decision_id)
            .expect("new decision");
        assert!(new
            .qualifiers
            .iter()
            .any(|qualifier| qualifier.text == "somente na API pública"));
        let old = decisions_api.detail(&fx.in_force).expect("old decision");
        assert_eq!(old.scope, vec!["somente no núcleo".to_owned()]);
    }

    #[test]
    fn two_candidates_resolve_by_adopting_one_and_rejecting_the_other() {
        let keep = |tag: &str, resolution: Resolution| {
            let test = support::open(tag, &[PROJECT]);
            two_conflicting(&test);
            let review = review_with(&test.store, consented(), &Judge::answering("{}"));
            let other = against(ConflictKind::Candidate, "bands");
            let entry = leave(&test.store, "fixed", &other);
            let view = review.conflict(&entry).expect("view").expect("stands");
            assert_eq!(view.other.question, "faixas de probabilidade para o juiz");
            assert!(!view.other.in_force);
            review
                .resolve("fixed", &other, &resolution)
                .expect("resolve");
            test
        };
        let test = keep("conflict-cands-this", Resolution::KeepThis);
        assert_eq!(status_of(&test.store, "fixed"), CandidateStatus::Accepted);
        assert_eq!(status_of(&test.store, "bands"), CandidateStatus::Dismissed);

        let test = keep("conflict-cands-other", Resolution::KeepOther);
        assert_eq!(status_of(&test.store, "fixed"), CandidateStatus::Dismissed);
        assert_eq!(status_of(&test.store, "bands"), CandidateStatus::Accepted);

        let test = keep(
            "conflict-cands-both",
            Resolution::KeepBoth {
                this_scope: "limiares onde há dado de sobra".into(),
                other_scope: "faixas onde há pouco dado".into(),
            },
        );
        assert_eq!(
            status_of(&test.store, "fixed"),
            CandidateStatus::EditedAndAccepted
        );
        assert_eq!(
            status_of(&test.store, "bands"),
            CandidateStatus::EditedAndAccepted
        );
    }

    #[test]
    fn a_candidate_the_ai_adopted_meanwhile_is_a_decision_in_force() {
        let test = support::open("conflict-adopted", &[PROJECT]);
        two_conflicting(&test);
        let review = review_with(&test.store, consented(), &Judge::answering("{}"));
        Adoption::new(test.store.clone())
            .adopt("bands", None, &[], &[])
            .expect("adopted");
        let other = against(ConflictKind::Candidate, "bands");
        let entry = leave(&test.store, "fixed", &other);
        let view = review.conflict(&entry).expect("view").expect("stands");
        assert!(view.other.in_force, "it stands now");

        review
            .resolve("fixed", &other, &Resolution::KeepThis)
            .expect("resolve");
        let superseded = decisions(&test.store, DecisionStatus::Superseded);
        assert_eq!(superseded.len(), 1);
        assert_eq!(superseded[0].candidate_id, "bands");
    }
}

fn live_rule(test: &support::TestStore, statement: &str) {
    use application::claims::{Claims, NewClaim};
    use domain::claims::ClaimKind;

    Claims::new(test.store.clone())
        .create(NewClaim {
            project_id: PROJECT.into(),
            kind: ClaimKind::Constraint,
            statement: statement.into(),
            valid_from: None,
            valid_until: None,
            source_decision_id: None,
            source_version: None,
            qualifiers: Vec::new(),
        })
        .expect("claim");
}

fn pending_rule(id: &str, choice: &str, created_at: &str) -> DecisionCandidateRecord {
    DecisionCandidateRecord {
        kind: "rule".into(),
        choice: choice.into(),
        ..candidate(id, "pending", 0.6, &format!("regra {id}"), created_at)
    }
}

#[test]
fn a_rule_that_repeats_one_in_force_or_one_of_the_batch_is_discarded_by_the_rules() {
    let test = support::open("review-rule-repeat", &[PROJECT]);
    live_rule(&test, "Cache entries must expire after ten minutes");
    test.store
        .insert_candidates(&[
            pending_rule(
                "again",
                "Expire cache entries after ten minutes",
                "2026-02-01T00:00:01Z",
            ),
            pending_rule(
                "twin-a",
                "Never retry a request that is not idempotent",
                "2026-02-01T00:00:02Z",
            ),
            pending_rule(
                "twin-b",
                "Never retry a request that is not idempotent.",
                "2026-02-01T00:00:03Z",
            ),
        ])
        .expect("pending");
    let judge = Judge::answering(r#"{"verdicts":[{"id":"I1","verdict":"human","reason":"ok"}]}"#);
    let review = review_with(&test.store, consented(), &judge);
    review.set_mode(Mode::Automatic).expect("on");
    review.run_at(PROJECT, NOW).expect("pass");

    let ledger = review.ledger(PROJECT).expect("ledger");
    let by_rules: Vec<_> = ledger
        .iter()
        .filter(|entry| entry.by == By::Rules && entry.verdict == Verdict::Discarded)
        .collect();
    assert_eq!(by_rules.len(), 2, "{ledger:?}");
    assert!(by_rules.iter().any(|entry| entry.item_id == "again"
        && entry
            .reason
            .starts_with("repete a regra em vigor: Cache entries must")));
    assert_eq!(
        ledger
            .iter()
            .filter(|entry| entry.item_id.starts_with("twin"))
            .filter(|entry| entry.by == By::Rules)
            .count(),
        1
    );
    // The survivor of the pair is the only item the judge sees.
    assert_eq!(judge.asked.lock().expect("lock").len(), 1);
    assert_eq!(status_of(&test.store, "again"), CandidateStatus::Dismissed);
}

#[test]
fn a_rule_that_only_resembles_one_in_force_goes_to_the_judge_with_it() {
    let test = support::open("review-rule-near", &[PROJECT]);
    live_rule(&test, "Cache entries expire after ten minutes");
    test.store
        .insert_candidates(&[pending_rule(
            "hours",
            "Cache entries expire after ten hours",
            "2026-02-01T00:00:01Z",
        )])
        .expect("pending");
    let judge =
        Judge::answering(r#"{"verdicts":[{"id":"I1","verdict":"human","reason":"muda o prazo"}]}"#);
    let review = review_with(&test.store, consented(), &judge);
    review.set_mode(Mode::Automatic).expect("on");
    review.run_at(PROJECT, NOW).expect("pass");

    let asked = judge.asked.lock().expect("lock")[0].clone();
    assert!(
        asked.contains("Regras parecidas em vigor: R1")
            && asked.contains("## Regras em vigor\n- R1 Cache entries expire after ten minutes"),
        "{asked}"
    );
    assert_eq!(status_of(&test.store, "hours"), CandidateStatus::Pending);
}

/// Pairs of rule statements labeled by a person: `true` when the second only
/// restates the first (PT, EN, reordered, synonyms), `false` when it is similar
/// but adds or reverses a constraint and must survive.
const RULE_PAIRS: &[(&str, &str, bool)] = &[
    (
        "Cache entries must expire after ten minutes",
        "Expire cache entries after ten minutes",
        true,
    ),
    (
        "O estado UNAVAILABLE é um estado operacional, não uma falha",
        "UNAVAILABLE é um estado operacional e não uma falha",
        true,
    ),
    (
        "Never log secrets or tokens in plain text",
        "Secrets and tokens must never be logged in plain text",
        true,
    ),
    (
        "Toda chamada externa deve ter timeout",
        "Every external call must have a timeout",
        true,
    ),
    (
        "Observe-only mode never changes the user's files",
        "Observe-only mode must never modify user files",
        true,
    ),
    (
        "Falhas de rede devem ser repetidas até três vezes",
        "Repetir falhas de rede até três vezes",
        true,
    ),
    (
        "A warning never produces FAIL",
        "An error always produces FAIL",
        false,
    ),
    (
        "Cache entries expire after ten minutes",
        "Cache entries expire after ten hours",
        false,
    ),
    (
        "Retry network failures up to three times",
        "Never retry network failures",
        false,
    ),
    (
        "Secrets must never be logged",
        "Secrets must be stored in the keychain",
        false,
    ),
    (
        "Tests must run offline",
        "Tests must run in under one minute",
        false,
    ),
    (
        "Migrations are never edited after release",
        "Migrations always run in a transaction",
        false,
    ),
];

#[test]
fn the_local_duplicate_rule_never_discards_a_different_rule_and_finds_the_plain_paraphrases() {
    use application::auto_approval::{rule_similarity, DUPLICATE_AT};

    let (mut true_positive, mut false_positive, mut paraphrases) = (0, 0, 0);
    for (a, b, same) in RULE_PAIRS {
        let likeness = rule_similarity(a, b);
        assert!((likeness - rule_similarity(b, a)).abs() < 1e-9, "{a} / {b}");
        if *same {
            paraphrases += 1;
        }
        if likeness >= DUPLICATE_AT {
            if *same {
                true_positive += 1;
            } else {
                false_positive += 1;
            }
        }
        eprintln!("{likeness:.2} {same:5} {a} / {b}");
    }
    let flagged = true_positive + false_positive;
    let precision = if flagged == 0 {
        1.0
    } else {
        f64::from(true_positive) / f64::from(flagged)
    };
    let recall = f64::from(true_positive) / f64::from(paraphrases);
    eprintln!("rule duplicate: precision {precision:.2}, recall {recall:.2}");
    assert!(precision >= 0.9, "precision {precision:.2}");
    assert!(recall >= RECALL_FLOOR, "recall {recall:.2}");
}

/// Recall measured on [`RULE_PAIRS`]; raise it when the rule improves.
const RECALL_FLOOR: f64 = 0.3;
