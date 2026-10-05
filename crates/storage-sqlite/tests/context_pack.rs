//! Context Pack over SQLite: relevance, reference date, supersession, claims,
//! budget and request validation.

mod support;

use application::claims::{Claims, NewClaim};
use application::context::{ContextPacks, ContextProvider, ContextRequest};
use application::decisions::Decisions;
use application::relations::DecisionRelations;
use domain::claims::ClaimKind;
use domain::relations::RelationKind;
use rusqlite::Connection;

fn request(task: &str) -> ContextRequest {
    ContextRequest {
        project_id: "p1".to_string(),
        task: task.to_string(),
        as_of: None,
        budget_chars: None,
        files: Vec::new(),
    }
}

fn claim(
    test: &support::TestStore,
    kind: ClaimKind,
    statement: &str,
    until: Option<&str>,
) -> String {
    Claims::new(test.store.clone())
        .create(NewClaim {
            source_version: None,
            qualifiers: Vec::new(),
            project_id: "p1".to_string(),
            kind,
            statement: statement.to_string(),
            valid_from: Some("2020-01-01".to_string()),
            valid_until: until.map(str::to_string),
            source_decision_id: None,
        })
        .expect("create claim")
        .claim_id
}

fn ids<T>(items: &[T], id: impl Fn(&T) -> &str) -> Vec<String> {
    items.iter().map(|item| id(item).to_string()).collect()
}

#[test]
fn selects_only_relevant_decisions_in_force_with_citations() {
    let test = support::open("pack-relevance", &["p1", "p2"]);
    let cache = support::decision(
        &test.store,
        "p1",
        "cache",
        "Como fazer cache da API?",
        "Cache em memória",
    );
    let storage = support::decision(&test.store, "p1", "db", "Qual banco usar?", "SQLite local");
    support::decision(&test.store, "p2", "other", "Cache da API?", "Redis");
    DecisionRelations::new(test.store.clone())
        .relate(&cache, &storage, RelationKind::DependsOn)
        .expect("relate");

    let pack = ContextPacks::new(test.store.clone())
        .build_pack(request("adicionar cache nas respostas da API"))
        .expect("pack");

    assert_eq!(
        ids(&pack.decisions, |d| &d.decision_id),
        vec![cache.clone()]
    );
    let decision = &pack.decisions[0];
    assert_eq!(decision.version, 1);
    assert_eq!(decision.evidence, vec!["art-p1".to_string()]);
    assert_eq!(decision.depends_on, vec![storage]);
    assert_eq!(pack.omitted, 0);
    assert!(pack.used_chars > 0 && pack.used_chars <= pack.budget_chars);
}

#[test]
fn reference_date_hides_future_decisions_and_supersession() {
    let test = support::open("pack-dates", &["p1"]);
    let older = support::decision(&test.store, "p1", "old", "Qual banco?", "Postgres");
    let newer = support::decision(&test.store, "p1", "new", "Qual banco?", "SQLite");
    DecisionRelations::new(test.store.clone())
        .supersede(&newer, &older)
        .expect("supersede");
    let packs = ContextPacks::new(test.store.clone());

    let now = packs.build_pack(request("qual banco")).expect("pack");
    assert_eq!(ids(&now.decisions, |d| &d.decision_id), vec![newer.clone()]);

    let before = packs
        .build_pack(ContextRequest {
            as_of: Some("2020-01-01".to_string()),
            ..request("qual banco")
        })
        .expect("pack");
    assert!(
        before.decisions.is_empty(),
        "nothing was decided yet in 2020"
    );

    Connection::open(test.root.join("app.db"))
        .expect("raw")
        .execute(
            "UPDATE decision_relations SET created_at = '2099-01-01T00:00:00Z'",
            [],
        )
        .expect("move the supersession to the future");
    let mut in_force = ids(
        &packs
            .build_pack(request("qual banco"))
            .expect("pack")
            .decisions,
        |d| &d.decision_id,
    );
    in_force.sort();
    let mut both = vec![older, newer];
    both.sort();
    assert_eq!(
        in_force, both,
        "before the supersession date both still held"
    );
}

#[test]
fn claims_are_matched_first_then_standing_rules() {
    let test = support::open("pack-claims", &["p1"]);
    let matched = claim(
        &test,
        ClaimKind::Assumption,
        "A API recebe no máximo 10 pedidos por segundo",
        None,
    );
    let rule = claim(
        &test,
        ClaimKind::Convention,
        "Mensagens de erro em português",
        None,
    );
    claim(&test, ClaimKind::Goal, "Lançar em dezembro", None);
    claim(
        &test,
        ClaimKind::Constraint,
        "Suportar Windows 7",
        Some("2021-01-01"),
    );

    let pack = ContextPacks::new(test.store.clone())
        .build_pack(request("limite de pedidos da API"))
        .expect("pack");

    assert_eq!(ids(&pack.claims, |c| &c.claim_id), vec![matched, rule]);
    assert!(pack.claims[0].matched);
    assert!(
        !pack.claims[1].matched,
        "conventions stand even without a match"
    );
    assert!(pack.decisions.is_empty());
}

#[test]
fn budget_limits_the_selection_and_counts_what_was_left_out() {
    let test = support::open("pack-budget", &["p1"]);
    let decisions = Decisions::new(test.store.clone());
    for key in ["a", "b", "c"] {
        let id = support::decision(
            &test.store,
            "p1",
            key,
            "Como versionar a API?",
            "Prefixo /v1",
        );
        decisions
            .revise(&id, support::rationale(&"justificativa longa ".repeat(15)))
            .expect("long rationale");
    }
    let pack = ContextPacks::new(test.store.clone())
        .build_pack(ContextRequest {
            budget_chars: Some(700),
            ..request("versionar API")
        })
        .expect("pack");
    assert_eq!(pack.decisions.len(), 1);
    assert_eq!(pack.omitted, 2);
    assert!(pack.used_chars <= 700);
}

#[test]
fn invalid_requests_are_rejected() {
    let test = support::open("pack-invalid", &["p1"]);
    let packs = ContextPacks::new(test.store.clone());
    let code = |request: ContextRequest| {
        packs
            .build_pack(request)
            .map(|_| ())
            .map_err(|error| error.code())
    };
    assert_eq!(code(request("   ")), Err("invalid_request"));
    assert_eq!(
        code(ContextRequest {
            budget_chars: Some(100),
            ..request("api")
        }),
        Err("invalid_request")
    );
    assert_eq!(
        code(ContextRequest {
            as_of: Some("ontem".into()),
            ..request("api")
        }),
        Err("invalid_request")
    );
    assert_eq!(
        code(ContextRequest {
            project_id: "nope".into(),
            ..request("api")
        }),
        Err("project_not_found")
    );
    assert_eq!(
        code(request("de a o")),
        Ok(()),
        "no searchable word is an empty pack, not an error"
    );
}
