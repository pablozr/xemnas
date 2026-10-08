//! Rules with component scope over SQLite: a claim derived from a decision
//! follows the decision's component ties, and a standing rule tied to
//! components only reaches tasks that touch one of them.

mod support;

use application::claims::{Claims, NewClaim};
use application::context::{ContextPacks, ContextProvider, ContextRequest};
use application::graph::{GraphStore, KnowledgeGraph, LinkRequest, NewEntity};
use domain::claims::ClaimKind;
use domain::entities::{EdgeKind, EdgeOrigin, EntityKind, NodeKind};
use storage_sqlite::SqliteStore;

fn claim(store: &SqliteStore, kind: ClaimKind, statement: &str, source: Option<&str>) -> String {
    Claims::new(store.clone())
        .create(NewClaim {
            source_version: None,
            qualifiers: Vec::new(),
            project_id: "p1".into(),
            kind,
            statement: statement.into(),
            valid_from: Some("2020-01-01".into()),
            valid_until: None,
            source_decision_id: source.map(str::to_string),
        })
        .expect("claim")
        .claim_id
}

fn component(store: &SqliteStore, name: &str, pattern: &str) -> String {
    KnowledgeGraph::new(store.clone())
        .create_entity(NewEntity {
            project_id: "p1".into(),
            kind: Some(EntityKind::Component),
            name: name.into(),
            patterns: vec![pattern.into()],
            ..NewEntity::default()
        })
        .expect("component")
        .entity_id
}

fn link(store: &SqliteStore, kind: EdgeKind, source: (NodeKind, &str), entity: &str) {
    KnowledgeGraph::new(store.clone())
        .link(LinkRequest {
            kind,
            source_kind: source.0,
            source_id: source.1.into(),
            entity_id: entity.into(),
        })
        .expect("link");
}

/// Rows `applies_to` from `claim` to `entity`, as (confirmed, invalidated).
fn applies(store: &SqliteStore, claim: &str, entity: &str) -> Vec<(bool, bool)> {
    store
        .project_edges("p1")
        .expect("edges")
        .into_iter()
        .filter(|edge| {
            edge.kind == EdgeKind::AppliesTo && edge.source_id == claim && edge.entity_id == entity
        })
        .map(|edge| (edge.confirmed_at.is_some(), edge.invalidated_at.is_some()))
        .collect()
}

fn pack_claims(store: &SqliteStore, task: &str, files: &[&str]) -> Vec<String> {
    ContextPacks::new(store.clone())
        .build_pack(ContextRequest {
            project_id: "p1".into(),
            task: task.into(),
            as_of: None,
            budget_chars: Some(12_000),
            files: files.iter().map(|file| (*file).into()).collect(),
        })
        .expect("pack")
        .claims
        .into_iter()
        .map(|claim| claim.claim_id)
        .collect()
}

#[test]
fn a_confirmed_decision_link_reaches_its_derived_claims() {
    let test = support::open("scope-propagation", &["p1"]);
    let store = &test.store;
    let decision = support::decision(store, "p1", "d1", "Como gravar?", "Pela outbox");
    let derived = claim(
        store,
        ClaimKind::Constraint,
        "Gravar pela outbox",
        Some(&decision),
    );
    let loose = claim(store, ClaimKind::Constraint, "Regra solta", None);
    let outbox = component(store, "outbox", "adapters/outbox/**");
    let storage = component(store, "storage", "crates/storage/**");

    // A person's link creates the edge confirmed.
    link(
        store,
        EdgeKind::Affects,
        (NodeKind::Decision, &decision),
        &outbox,
    );
    assert_eq!(applies(store, &derived, &outbox), vec![(true, false)]);
    assert!(applies(store, &loose, &outbox).is_empty());

    // A suggestion confirmed later (AI, mention, auto review) does the same.
    let graph = KnowledgeGraph::new(store.clone());
    let pending = application::graph::EdgeRecord {
        edge_id: "pending-edge".into(),
        project_id: "p1".into(),
        kind: EdgeKind::Affects,
        source_kind: NodeKind::Decision,
        source_id: decision.clone(),
        entity_id: storage.clone(),
        origin: EdgeOrigin::Derived,
        reason: "citado no texto: \"storage\"".into(),
        created_at: "2026-01-03T00:00:00Z".into(),
        confirmed_at: None,
        invalidated_at: None,
        confirmed_by: None,
        invalidated_by: None,
    };
    store.insert_edge(&pending).expect("pending");
    assert!(applies(store, &derived, &storage).is_empty());
    graph.confirm("pending-edge").expect("confirm");
    assert_eq!(applies(store, &derived, &storage), vec![(true, false)]);
}

#[test]
fn refresh_backfills_once_and_respects_a_removed_tie() {
    let test = support::open("scope-backfill", &["p1"]);
    let store = &test.store;
    let decision = support::decision(store, "p1", "d1", "Como gravar?", "Pela outbox");
    let outbox = component(store, "outbox", "adapters/outbox/**");
    let storage = component(store, "storage", "crates/storage/**");
    // The ties come first, the claim later: nothing propagated yet.
    link(
        store,
        EdgeKind::Affects,
        (NodeKind::Decision, &decision),
        &outbox,
    );
    link(
        store,
        EdgeKind::Affects,
        (NodeKind::Decision, &decision),
        &storage,
    );
    let derived = claim(
        store,
        ClaimKind::Convention,
        "Gravar pela outbox",
        Some(&decision),
    );
    assert!(applies(store, &derived, &outbox).is_empty());

    let graph = KnowledgeGraph::new(store.clone());
    graph.refresh_suggestions("p1").expect("refresh");
    assert_eq!(applies(store, &derived, &outbox), vec![(true, false)]);
    assert_eq!(applies(store, &derived, &storage), vec![(true, false)]);
    graph.refresh_suggestions("p1").expect("refresh again");
    assert_eq!(applies(store, &derived, &outbox).len(), 1, "idempotent");

    // A person removes one tie: neither refresh nor a new confirmation
    // brings it back.
    let edge = store
        .project_edges("p1")
        .expect("edges")
        .into_iter()
        .find(|edge| edge.source_id == derived && edge.entity_id == storage)
        .expect("edge");
    graph.invalidate(&edge.edge_id).expect("invalidate");
    graph
        .refresh_suggestions("p1")
        .expect("refresh after removal");
    assert_eq!(applies(store, &derived, &storage), vec![(true, true)]);
    assert_eq!(applies(store, &derived, &outbox), vec![(true, false)]);
}

#[test]
fn a_standing_rule_follows_its_components_and_a_global_one_stays() {
    let test = support::open("scope-pack", &["p1"]);
    let store = &test.store;
    let scoped = claim(
        store,
        ClaimKind::Constraint,
        "Registrar cada lançamento com o selo do auditor",
        None,
    );
    let global = Claims::new(store.clone())
        .create(NewClaim {
            source_version: None,
            qualifiers: vec![application::qualifiers::KnowledgeQualifier {
                kind: application::qualifiers::QualifierKind::Scope,
                text: application::context::GLOBAL_SCOPE.into(),
                artifact_id: None,
            }],
            project_id: "p1".into(),
            kind: ClaimKind::Convention,
            statement: "Mensagens de erro em português".into(),
            valid_from: Some("2020-01-01".into()),
            valid_until: None,
            source_decision_id: None,
        })
        .expect("global claim")
        .claim_id;
    let ledger = component(store, "ledger", "crates/ledger/**");
    link(
        store,
        EdgeKind::AppliesTo,
        (NodeKind::Claim, &scoped),
        &ledger,
    );

    // Files inside the component.
    let by_files = pack_claims(
        store,
        "Ajustar o módulo aberto",
        &["crates/ledger/src/post.rs"],
    );
    assert!(by_files.contains(&scoped) && by_files.contains(&global));

    // Mention in the task text, no files.
    let by_mention = pack_claims(store, "Reorganizar o ledger por mês", &[]);
    assert!(by_mention.contains(&scoped) && by_mention.contains(&global));

    // Another component's file, and no files and no mention.
    let elsewhere = pack_claims(
        store,
        "Ajustar o módulo aberto",
        &["crates/storage/src/db.rs"],
    );
    assert_eq!(elsewhere, vec![global.clone()]);
    let unrelated = pack_claims(store, "Escolher a trilha sonora do jogo", &[]);
    assert_eq!(unrelated, vec![global]);
}

#[test]
fn a_decision_of_the_pack_tied_to_the_component_does_not_bring_its_rules() {
    let test = support::open("scope-decision", &["p1"]);
    let store = &test.store;
    let decision = support::decision(
        store,
        "p1",
        "d1",
        "Como fechar o balancete mensal?",
        "Conferir os totais antes de fechar",
    );
    let scoped = claim(
        store,
        ClaimKind::Constraint,
        "Registrar cada lançamento com o selo do auditor",
        None,
    );
    let ledger = component(store, "contabil", "crates/contabil/**");
    link(
        store,
        EdgeKind::AppliesTo,
        (NodeKind::Claim, &scoped),
        &ledger,
    );
    link(
        store,
        EdgeKind::Affects,
        (NodeKind::Decision, &decision),
        &ledger,
    );

    // No files and no mention of the component: the pack carries a decision
    // tied to it, but that does not touch the component, or one decision
    // would drag in every rule of its component.
    let pack = pack_claims(store, "Como fechar o balancete mensal", &[]);
    assert!(!pack.contains(&scoped), "{pack:?}");
    // Touching the component by a file brings the rule.
    let pack = pack_claims(
        store,
        "Como fechar o balancete mensal",
        &["crates/contabil/src/lib.rs"],
    );
    assert!(pack.contains(&scoped), "{pack:?}");
}
