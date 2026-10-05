use super::*;
use crate::extract::{
    CandidateExtractor, CandidateProposal, DecisionEvidence, ExtractError, RelevanceSignal,
};
use crate::profile::{
    build_preview, grant_consent, offline_default_profile, AiProfile, ProfileError, ProfileKind,
};
use std::sync::Mutex;

const NOW: &str = "2026-10-02T00:00:00Z";

#[test]
fn array_assignments_are_redacted_after_json_decoding_and_citations_match() {
    let mut d = decision("array", DecisionStatus::Accepted);
    let payload = serde_json::to_string(&vec![
        "api_key=secret123\nlinha café 🚀",
        "password=hidden456\nsegunda",
    ])
    .unwrap();
    d.assumptions = payload.clone();
    d.scope = payload.clone();
    d.consequences = payload.clone();
    d.reconsider_when = payload;
    let fields = decision_fields(&d);
    for name in ["assumptions", "scope", "consequences", "reconsider_when"] {
        let f = fields.iter().find(|f| f.field == name).unwrap();
        assert!(!f.text.contains("secret123"));
        assert!(!f.text.contains("hidden456"));
        assert!(f.text.contains("\nlinha café 🚀"));
        let e = evidence(f, "linha café 🚀").unwrap();
        assert_eq!(&e.text[e.start_byte..e.end_byte], e.quote);
        assert_eq!(e.text, f.text);
    }
    let sent = serde_json::to_string(&fields).unwrap();
    assert!(!sent.contains("secret123"));
    assert!(!sent.contains("hidden456"));
}

#[test]
fn singleton_tension_is_discarded_but_insufficient_information_is_allowed() {
    let unit = Unit {
        ids: vec!["rule".into()],
        selection: "unlinked rule".into(),
        fields: vec![field(
            ReviewSourceKind::Claim,
            "rule",
            "statement",
            "Regra café",
        )],
    };
    let mut report = inspect_snapshot(&snapshot(), NOW.into()).unwrap();
    for kind in ["PossibleTension", "InsufficientInformation"] {
        let answer =
            json!({"findings":[{"kind":kind,"explanation":"explicação","question":"pergunta?",
            "evidence":[{"id":"rule","field":"statement","quote":"Regra"},
                {"id":"rule","field":"statement","quote":"café"}]}]})
            .to_string();
        parse(&answer, &unit, &mut report).unwrap();
    }
    assert_eq!(report.coverage.discarded_findings, 1);
    assert_eq!(
        report.findings.last().unwrap().kind,
        FindingKind::InsufficientInformation
    );
}

#[test]
fn oversize_units_do_not_consume_the_twelve_actual_calls() {
    let mut s = snapshot();
    s.decisions.clear();
    s.relations.clear();
    s.claim_suggestions.clear();
    s.claims.clear();
    for i in 0..16 {
        let from = format!("pair-{i:02}-new");
        let to = format!("pair-{i:02}-old");
        let mut newer = decision(&from, DecisionStatus::Accepted);
        if i < 2 {
            newer.question = "texto comum ".repeat(MAX_UNIT_BYTES);
        }
        s.decisions.push(newer);
        s.decisions.push(decision(&to, DecisionStatus::Superseded));
        s.relations.push(RelationRow {
            from,
            to,
            kind: "supersedes".into(),
            created_at: NOW.into(),
        });
    }
    let m = model(vec![]);
    let calls = m.inputs.clone();
    let report = reviewer(s, Some(profile()), m)
        .review("p", ReviewCancellation::default())
        .unwrap();
    assert_eq!(calls.lock().unwrap().len(), MAX_CALLS);
    assert_eq!(
        report
            .coverage
            .semantic_units
            .iter()
            .filter(|u| u.outcome == ReviewUnitOutcome::Reviewed)
            .count(),
        12
    );
    assert!(report.coverage.semantic_units[..2]
        .iter()
        .all(|u| u.outcome == ReviewUnitOutcome::Omitted));
    assert!(report.coverage.semantic_units[14..]
        .iter()
        .all(|u| u.outcome == ReviewUnitOutcome::Omitted));
}

#[test]
fn part_of_expansion_is_two_live_hops_independent_of_edge_order() {
    use domain::entities::{EdgeOrigin, EntityKind};
    let mut s = snapshot();
    s.relations.clear();
    s.claims.clear();
    s.claim_suggestions.clear();
    s.decisions = vec![
        decision("left", DecisionStatus::Accepted),
        decision("right", DecisionStatus::Accepted),
    ];
    for id in ["a", "b", "c", "d"] {
        s.entities.push(EntityRecord {
            entity_id: id.into(),
            project_id: "p".into(),
            kind: EntityKind::Component,
            name: id.into(),
            key: id.into(),
            description: "".into(),
            patterns: vec![],
            aliases: vec![],
            created_at: NOW.into(),
            retired_at: None,
        });
    }
    for (source, target, kind) in [
        ("left", "a", NodeKind::Decision),
        ("right", "d", NodeKind::Decision),
        ("a", "b", NodeKind::Entity),
        ("b", "c", NodeKind::Entity),
        ("c", "d", NodeKind::Entity),
    ] {
        s.edges.push(EdgeRecord {
            edge_id: format!("{source}-{target}"),
            project_id: "p".into(),
            kind: if kind == NodeKind::Decision {
                EdgeKind::AppliesTo
            } else {
                EdgeKind::PartOf
            },
            source_kind: kind,
            source_id: source.into(),
            entity_id: target.into(),
            origin: EdgeOrigin::Human,
            reason: "".into(),
            created_at: NOW.into(),
            confirmed_at: Some(NOW.into()),
            invalidated_at: None,
        });
    }
    // FTS fake returns no IDs from this snapshot, isolating entity selection.
    for reverse in [false, true] {
        if reverse {
            s.edges.reverse();
        }
        let mut report = inspect_snapshot(&s, NOW.into()).unwrap();
        let r = reviewer(s.clone(), None, model(vec![]));
        assert!(
            r.units(&s, &mut report).unwrap().is_empty(),
            "third hop must not match"
        );
        let right = s.edges.iter_mut().find(|e| e.source_id == "right").unwrap();
        right.entity_id = "c".into();
        let mut report = inspect_snapshot(&s, NOW.into()).unwrap();
        assert_eq!(
            r.units(&s, &mut report).unwrap().len(),
            1,
            "second hop must match"
        );
        s.entities
            .iter_mut()
            .find(|e| e.entity_id == "b")
            .unwrap()
            .retired_at = Some(NOW.into());
        let mut report = inspect_snapshot(&s, NOW.into()).unwrap();
        assert!(
            r.units(&s, &mut report).unwrap().is_empty(),
            "retired intermediary must not expand"
        );
        s.entities
            .iter_mut()
            .find(|e| e.entity_id == "b")
            .unwrap()
            .retired_at = None;
        s.edges
            .iter_mut()
            .find(|e| e.source_id == "right")
            .unwrap()
            .entity_id = "d".into();
    }
}
fn decision(id: &str, status: DecisionStatus) -> StoredDecision {
    StoredDecision {
        qualifiers: "[]".into(),
        decision_id: id.into(),
        candidate_id: format!("candidate-{id}"),
        project_id: "p".into(),
        project_location: "synthetic".into(),
        capture_id: None,
        status,
        question: format!("Escolha {id}?"),
        choice: "ação café 🚀".into(),
        rationale: "motivo registrado".into(),
        assumptions: "[]".into(),
        reconsider_when: "[]".into(),
        scope: "[]".into(),
        consequences: "[]".into(),
        version: 1,
        created_at: NOW.into(),
        confirmed_at: NOW.into(),
        updated_at: NOW.into(),
    }
}
fn snapshot() -> ReviewSnapshot {
    ReviewSnapshot {
        project_id: "p".into(),
        decisions: vec![
            decision("old", DecisionStatus::Superseded),
            decision("new", DecisionStatus::Accepted),
            decision("dependent", DecisionStatus::Accepted),
        ],
        claims: vec![ClaimRecord {
            source_version: None,
            inherited_scope: "[]".into(),
            qualifiers: "[]".into(),
            claim_id: "rule".into(),
            project_id: "p".into(),
            kind: domain::claims::ClaimKind::Constraint,
            statement: "Não apagar histórico".into(),
            valid_from: NOW.into(),
            valid_until: None,
            source_decision_id: Some("old".into()),
            created_at: NOW.into(),
            updated_at: NOW.into(),
        }],
        relations: vec![
            RelationRow {
                from: "new".into(),
                to: "old".into(),
                kind: "supersedes".into(),
                created_at: NOW.into(),
            },
            RelationRow {
                from: "dependent".into(),
                to: "old".into(),
                kind: "depends_on".into(),
                created_at: NOW.into(),
            },
        ],
        entities: vec![],
        edges: vec![],
        relation_suggestions: vec![],
        claim_suggestions: vec![ClaimSuggestionRecord {
            inherited_scope: "[]".into(),
            source_version: Some(1),
            qualifiers: "[]".into(),
            suggestion_id: "pending".into(),
            project_id: "p".into(),
            decision_id: "old".into(),
            kind: domain::claims::ClaimKind::Constraint,
            statement: "Proposta não confirmada".into(),
            quote: "ação café".into(),
            created_at: NOW.into(),
        }],
    }
}

#[test]
fn inherited_scope_is_validated_for_claims_and_pending_suggestions() {
    let mut s = snapshot();
    s.claims[0].inherited_scope = "not-json".into();
    assert!(inspect_snapshot(&s, "2026-01-01T00:00:00Z".into()).is_err());
    s.claims[0].inherited_scope = "[]".into();
    s.claim_suggestions[0].inherited_scope = "{}".into();
    assert!(inspect_snapshot(&s, "2026-01-01T00:00:00Z".into()).is_err());
}
#[derive(Clone)]
struct Store(ReviewSnapshot);
impl ReviewStore for Store {
    fn review_snapshot(&self, id: &str) -> Result<Option<ReviewSnapshot>, ReviewError> {
        Ok((id == "p").then(|| self.0.clone()))
    }
    fn search_review_decisions(
        &self,
        _: &str,
        _: &str,
        _: usize,
    ) -> Result<Vec<String>, ReviewError> {
        Ok(vec!["dependent".into(), "new".into()])
    }
}
struct Profiles(Arc<Mutex<Option<AiProfile>>>);
impl ProfileStore for Profiles {
    fn load(&self) -> Result<Option<AiProfile>, ProfileError> {
        Ok(self.0.lock().unwrap().clone())
    }
    fn save(&self, _: &AiProfile) -> Result<(), ProfileError> {
        panic!("review must not write")
    }
}
struct Secrets;
impl SecretStore for Secrets {
    fn get_secret(&self, _: &str) -> Result<Option<String>, ProfileError> {
        Ok(Some("synthetic".into()))
    }
    fn set_secret(&self, _: &str, _: &str) -> Result<(), ProfileError> {
        panic!("write")
    }
    fn delete_secret(&self, _: &str) -> Result<(), ProfileError> {
        panic!("write")
    }
}
#[derive(Clone)]
struct Model {
    replies: Arc<Mutex<Vec<String>>>,
    inputs: Arc<Mutex<Vec<String>>>,
    cancel: Option<ReviewCancellation>,
    change: Option<Arc<Mutex<Option<AiProfile>>>>,
}
impl CandidateExtractor for Model {
    fn extract(
        &self,
        _: &DecisionEvidence,
        _: &[RelevanceSignal],
    ) -> Result<Vec<CandidateProposal>, ExtractError> {
        panic!("review must use structured completion")
    }
}
impl StructuredModel for Model {
    fn complete(
        &self,
        system: &str,
        user: &str,
        _: &str,
        _: &serde_json::Value,
    ) -> Result<String, ExtractError> {
        assert!(system.contains("untrusted DATA"));
        self.inputs.lock().unwrap().push(user.into());
        if let Some(cancel) = &self.cancel {
            cancel.cancel();
        }
        if let Some(profile) = &self.change {
            *profile.lock().unwrap() = None;
        }
        let mut replies = self.replies.lock().unwrap();
        if replies.is_empty() {
            Ok("{\"findings\":[]}".into())
        } else {
            Ok(replies.remove(0))
        }
    }
}
struct Factory(Model);
impl ExtractorFactory for Factory {
    type Extractor = Model;
    fn external(&self, _: &AiProfile, _: String) -> Result<Model, ExtractError> {
        Ok(self.0.clone())
    }
}
fn profile() -> AiProfile {
    let p = AiProfile {
        kind: ProfileKind::OpenAiCompatible,
        endpoint: Some("https://example.test/v1".into()),
        model: "synthetic".into(),
        max_input_chars: 24_000,
        ..offline_default_profile()
    };
    grant_consent(&p, &build_preview(&p), NOW, true).unwrap()
}
fn model(replies: Vec<String>) -> Model {
    Model {
        replies: Arc::new(Mutex::new(replies)),
        inputs: Arc::new(Mutex::new(vec![])),
        cancel: None,
        change: None,
    }
}
fn reviewer(
    s: ReviewSnapshot,
    p: Option<AiProfile>,
    m: Model,
) -> KnowledgeReviewer<Store, Profiles, Secrets, Factory> {
    KnowledgeReviewer::new(
        Store(s),
        AiSettings::new(Profiles(Arc::new(Mutex::new(p))), Secrets),
        Factory(m),
    )
}
#[test]
fn deterministic_direction_raw_pending_and_validity() {
    let s = snapshot();
    let r = inspect_snapshot(&s, NOW.into()).unwrap();
    assert!(r.coverage.deterministic_complete);
    assert_eq!(r.coverage.inventory.current_decisions, 2);
    assert_eq!(r.coverage.inventory.superseded_decisions, 1);
    assert_eq!(r.coverage.inventory.valid_rules, 1);
    assert_eq!(
        r.findings.iter().map(|f| f.kind).collect::<Vec<_>>(),
        vec![
            FindingKind::DependencyOnSuperseded,
            FindingKind::RuleFromSuperseded,
            FindingKind::PendingProposalFromSuperseded
        ]
    );
    let mut s = s;
    s.claims[0].valid_until = Some("2026-10-03T00:00:00Z".into());
    assert_eq!(
        inspect_snapshot(&s, "2026-10-03T00:00:00Z".into())
            .unwrap()
            .coverage
            .inventory
            .valid_rules,
        0
    );
    s.relations[1].from = "old".into();
    s.relations[1].to = "dependent".into();
    assert!(!inspect_snapshot(&s, NOW.into())
        .unwrap()
        .findings
        .iter()
        .any(|f| f.kind == FindingKind::DependencyOnSuperseded));
}
#[test]
fn symmetric_conflicts_deduplicated_and_corruption_fails() {
    let mut s = snapshot();
    for (a, b) in [("new", "dependent"), ("dependent", "new")] {
        s.relations.push(RelationRow {
            from: a.into(),
            to: b.into(),
            kind: "conflicts_with".into(),
            created_at: NOW.into(),
        });
    }
    assert_eq!(
        inspect_snapshot(&s, NOW.into())
            .unwrap()
            .findings
            .iter()
            .filter(|f| f.kind == FindingKind::RecordedConflict)
            .count(),
        1
    );
    s.decisions[0].project_id = "other".into();
    assert!(matches!(validate(&s), Err(ReviewError::InvalidData(_))));
    s.decisions[0].project_id = "p".into();
    s.claims[0].valid_from = "bad".into();
    assert!(validate(&s).is_err());
}
#[test]
fn exact_unicode_allowlisted_fields_and_redaction() {
    let d = decision("a", DecisionStatus::Accepted);
    let mut fields = decision_fields(&d);
    fields.push(field(
        ReviewSourceKind::Decision,
        "b",
        "choice",
        "outro café",
    ));
    let unit = Unit {
        ids: vec!["a".into(), "b".into()],
        selection: "pair".into(),
        fields,
    };
    let mut r = inspect_snapshot(&snapshot(), NOW.into()).unwrap();
    let answer = json!({"findings":[{"kind":"PossibleTension","explanation":"api_key=secret123",
        "question":"Confirma?","evidence":[{"id":"a","field":"choice","quote":"café 🚀"},
        {"id":"b","field":"choice","quote":"café"}]}]})
    .to_string();
    parse(&answer, &unit, &mut r).unwrap();
    let f = r.findings.last().unwrap();
    assert!(!f.explanation.contains("secret123"));
    for e in &f.evidence {
        assert_eq!(&e.text[e.start_byte..e.end_byte], e.quote);
    }
    for (id, field, quote) in [
        ("a", "choice", "CAFÉ"),
        ("unknown", "choice", "café"),
        ("a", "not_sent", "café"),
        ("a", "choice", "cafe"),
    ] {
        let invalid = json!({"findings":[{"kind":"InsufficientInformation","explanation":"x",
            "question":"q","evidence":[{"id":id,"field":field,"quote":quote}]}]})
        .to_string();
        parse(&invalid, &unit, &mut r).unwrap();
    }
    assert_eq!(r.coverage.discarded_findings, 4);
    assert!(parse("invalid", &unit, &mut r).is_err());
}
#[test]
fn inspect_never_loads_or_calls_and_unavailable_never_seeds() {
    let m = model(vec![]);
    let inputs = m.inputs.clone();
    let reviewer = reviewer(snapshot(), None, m);
    assert_eq!(
        reviewer.inspect("p").unwrap().semantic_status,
        SemanticStatus::NotRequested
    );
    assert_eq!(
        reviewer
            .review("p", ReviewCancellation::default())
            .unwrap()
            .semantic_status,
        SemanticStatus::Unavailable
    );
    assert!(inputs.lock().unwrap().is_empty());
    assert_eq!(reviewer.settings.load().unwrap(), None);
    assert_eq!(
        reviewer.inspect("missing").unwrap_err(),
        ReviewError::ProjectNotFound
    );
}
#[test]
fn supersedes_and_unlinked_candidates_sent_without_writes() {
    let m = model(vec![]);
    let inputs = m.inputs.clone();
    let s = snapshot();
    let reviewer = reviewer(s.clone(), Some(profile()), m);
    let r = reviewer.review("p", ReviewCancellation::default()).unwrap();
    assert_eq!(r.semantic_status, SemanticStatus::Partial); // bounded selection disclosed
    assert!(r.coverage.semantic_units[0]
        .selection
        .contains("supersedes"));
    assert!(r
        .coverage
        .semantic_units
        .iter()
        .any(|u| u.selection.contains("unlinked")));
    assert!(inputs
        .lock()
        .unwrap()
        .iter()
        .all(|input| !input.contains("Proposta não confirmada")));
    assert_eq!(reviewer.store.0, s);
}
#[test]
fn malformed_partial_cancel_and_profile_change_keep_local_findings() {
    let r = reviewer(snapshot(), Some(profile()), model(vec!["bad json".into()]))
        .review("p", ReviewCancellation::default())
        .unwrap();
    assert_eq!(r.semantic_status, SemanticStatus::Partial);
    assert!(r
        .findings
        .iter()
        .any(|f| f.kind == FindingKind::DependencyOnSuperseded));
    let cancel = ReviewCancellation::default();
    let mut m = model(vec![]);
    m.cancel = Some(cancel.clone());
    let r = reviewer(snapshot(), Some(profile()), m)
        .review("p", cancel)
        .unwrap();
    assert_eq!(r.semantic_status, SemanticStatus::Cancelled);
    assert!(r
        .coverage
        .semantic_units
        .iter()
        .all(|u| u.outcome != ReviewUnitOutcome::Reviewed));
    let profiles = Arc::new(Mutex::new(Some(profile())));
    let mut m = model(vec![]);
    m.change = Some(profiles.clone());
    let inputs = m.inputs.clone();
    let reviewer = KnowledgeReviewer::new(
        Store(snapshot()),
        AiSettings::new(Profiles(profiles), Secrets),
        Factory(m),
    );
    let r = reviewer.review("p", ReviewCancellation::default()).unwrap();
    assert_eq!(r.semantic_status, SemanticStatus::Partial);
    assert_eq!(inputs.lock().unwrap().len(), 1);
}
#[test]
fn cancellation_before_calls_preserves_checks_and_hash_is_stable() {
    let cancel = ReviewCancellation::default();
    cancel.cancel();
    let r = reviewer(snapshot(), Some(profile()), model(vec![]))
        .review("p", cancel)
        .unwrap();
    assert_eq!(r.semantic_status, SemanticStatus::Cancelled);
    assert!(!r.findings.is_empty());
    let mut s = snapshot();
    let first = inspect_snapshot(&s, NOW.into()).unwrap().input_hash;
    s.decisions.reverse();
    assert_eq!(inspect_snapshot(&s, NOW.into()).unwrap().input_hash, first);
    s.decisions[0].version += 1;
    assert_ne!(inspect_snapshot(&s, NOW.into()).unwrap().input_hash, first);
}

#[test]
fn pending_graph_claim_source_and_confirmed_scope_candidates() {
    use domain::entities::{EdgeOrigin, EntityKind};
    let mut s = snapshot();
    s.entities.push(EntityRecord {
        entity_id: "component".into(),
        project_id: "p".into(),
        kind: EntityKind::Component,
        name: "backend".into(),
        key: "backend".into(),
        description: "".into(),
        patterns: vec![],
        aliases: vec![],
        created_at: NOW.into(),
        retired_at: None,
    });
    for (id, source, kind, confirmed) in [
        ("e1", "new", NodeKind::Decision, true),
        ("e2", "dependent", NodeKind::Decision, true),
        ("pending-edge", "rule", NodeKind::Claim, false),
    ] {
        s.edges.push(EdgeRecord {
            edge_id: id.into(),
            project_id: "p".into(),
            kind: EdgeKind::AppliesTo,
            source_kind: kind,
            source_id: source.into(),
            entity_id: "component".into(),
            origin: EdgeOrigin::Human,
            reason: "".into(),
            created_at: NOW.into(),
            confirmed_at: confirmed.then(|| NOW.into()),
            invalidated_at: None,
        });
    }
    let r = inspect_snapshot(&s, NOW.into()).unwrap();
    assert!(r
        .findings
        .iter()
        .any(|f| f.evidence.iter().any(|e| e.source.id == "pending-edge")));
    s.edges[2].source_id = "foreign".into();
    assert!(validate(&s).is_err());
}

#[test]
fn failed_and_completed_statuses_and_input_omission() {
    let mut s = snapshot();
    s.relations.truncate(1);
    s.decisions.retain(|d| d.decision_id != "dependent");
    let r = reviewer(s.clone(), Some(profile()), model(vec!["malformed".into()]))
        .review("p", ReviewCancellation::default())
        .unwrap();
    assert_eq!(r.semantic_status, SemanticStatus::Failed);
    let mut p = profile();
    p.max_input_chars = 10;
    p = grant_consent(&p, &build_preview(&p), NOW, true).unwrap();
    let m = model(vec![]);
    let calls = m.inputs.clone();
    let r = reviewer(s, Some(p), m)
        .review("p", ReviewCancellation::default())
        .unwrap();
    assert!(calls.lock().unwrap().is_empty());
    assert!(r
        .coverage
        .semantic_units
        .iter()
        .all(|u| u.outcome == ReviewUnitOutcome::Omitted));
    let mut empty = snapshot();
    empty.decisions.clear();
    empty.relations.clear();
    empty.claims.clear();
    empty.claim_suggestions.clear();
    let r = reviewer(empty, Some(profile()), model(vec![]))
        .review("p", ReviewCancellation::default())
        .unwrap();
    assert_eq!(r.semantic_status, SemanticStatus::Completed);
}
