//! Automatic review against the real database: the rules settle the plain
//! cases for free, the AI is asked once for the rest and never too often, and
//! everything it decides is in a ledger.

mod support;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use application::adoption::{Adoption, AdoptionApi};
use application::analysis::ExtractorFactory;
use application::auto_approval::{
    ApprovalStore, Approvals, ApprovalsApi, By, ItemKind, Mode, ReviewError, Verdict, CALLS_PER_DAY,
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
fn a_lone_ambiguous_item_waits_for_company_up_to_two_hours() {
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
            "2026-03-01T09:30:00Z",
        )])
        .expect("pending");
    let early = review
        .run_at(PROJECT, "2026-03-01T10:00:00Z")
        .expect("early");
    assert!(!early.asked, "30 minutes old and alone: it waits");
    assert_eq!(judge.calls.load(Ordering::SeqCst), 0);
    let late = review
        .run_at(PROJECT, "2026-03-01T11:31:00Z")
        .expect("late");
    assert!(late.asked, "two hours old: it is asked");
    assert_eq!(status_of(&test.store, "lone"), CandidateStatus::Accepted);
}

#[test]
fn calls_are_spaced_capped_per_day_and_not_retried_at_once_after_a_failure() {
    let test = support::open("review-limits", &[PROJECT]);
    let judge = Judge::failing();
    let review = review_with(&test.store, consented(), &judge);
    review.set_mode(Mode::Automatic).expect("on");
    let rows: Vec<_> = (0..4)
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
    assert_eq!(judge.calls.load(Ordering::SeqCst), 1);
    let soon = review
        .run_at(PROJECT, "2026-03-01T10:10:00Z")
        .expect("soon");
    assert!(!soon.asked, "20 minutes keep apart even after a failure");
    assert_eq!(judge.calls.load(Ordering::SeqCst), 1);
    for at in 0..4 {
        assert_eq!(
            status_of(&test.store, &format!("amb-{at}")),
            CandidateStatus::Pending
        );
    }

    // The daily cap: six calls in the last day and nothing more is asked.
    for hour in 0..CALLS_PER_DAY {
        test.store
            .record_review_call(&format!("2026-03-01T{:02}:00:00Z", 11 + hour), 3, true)
            .expect("call");
    }
    let capped = review
        .run_at(PROJECT, "2026-03-01T20:00:00Z")
        .expect("capped");
    assert!(!capped.asked);
    assert_eq!(judge.calls.load(Ordering::SeqCst), 1);
}

#[test]
fn without_an_enabled_provider_only_the_rules_act() {
    let test = support::open("review-offline", &[PROJECT]);
    let judge = Judge::answering("{}");
    let review = review_with(&test.store, offline_default_profile(), &judge);
    review.set_mode(Mode::Automatic).expect("on");
    assert!(!review.status().expect("status").judge);
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
