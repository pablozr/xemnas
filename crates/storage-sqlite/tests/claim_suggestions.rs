//! Context derived from adopted decisions over SQLite: the judge's items
//! checked against the decision, confirmed into scoped rules with the
//! decision as source, rejected for good, and flagged for review when the
//! source decision is replaced.

mod support;

use std::cell::RefCell;

use application::analysis::ExtractorFactory;
use application::claim_suggestions::{ClaimFinder, ClaimSuggestions};
use application::claims::Claims;
use application::extract::{
    CandidateExtractor, CandidateProposal, DecisionEvidence, ExtractError, RelevanceSignal,
};
use application::graph::{KnowledgeGraph, LinkRequest, NewEntity};
use application::overview::{JsonValue, StructuredModel};
use application::profile::{
    build_preview, grant_consent, offline_default_profile, AiProfile, AiSettings, ProfileError,
    ProfileKind, ProfileStore, SecretStore,
};
use application::relations::DecisionRelations;
use domain::claims::ClaimKind;
use domain::entities::{EdgeKind, EntityKind, NodeKind};

struct Profiles(RefCell<AiProfile>);

#[test]
fn original_source_version_blocks_stale_generation_and_confirmation() {
    use application::claim_suggestions::{ClaimSuggestionRecord, ClaimSuggestionStore};
    use application::claims::ClaimStore;
    use application::decisions::{DecisionEdits, Decisions};
    let test = support::open("stale-suggestion-source", &["p1"]);
    let id = support::decision(&test.store, "p1", "source", "Question", "Choice");
    let record = ClaimSuggestionRecord {
        suggestion_id: "suggestion".into(),
        project_id: "p1".into(),
        decision_id: id.clone(),
        kind: ClaimKind::Constraint,
        statement: "Rule".into(),
        quote: "Choice".into(),
        created_at: "2026-01-01T00:00:00Z".into(),
        qualifiers: "[]".into(),
        inherited_scope: "[]".into(),
        source_version: Some(1),
    };
    assert!(test.store.insert_claim_suggestion(&record).unwrap());
    Decisions::new(test.store.clone())
        .revise(
            &id,
            DecisionEdits {
                choice: Some("Changed choice".into()),
                ..DecisionEdits::default()
            },
        )
        .unwrap();
    assert!(ClaimSuggestions::new(test.store.clone())
        .confirm("suggestion")
        .is_err());
    assert!(test.store.project_claims("p1").unwrap().is_empty());
    let mut stale = record;
    stale.suggestion_id = "stale-generation".into();
    assert!(!test.store.insert_claim_suggestion(&stale).unwrap());
    stale.source_version = None;
    assert!(!test.store.insert_claim_suggestion(&stale).unwrap());
    stale.source_version = Some(2);
    assert!(test.store.insert_claim_suggestion(&stale).unwrap());
    assert_eq!(
        test.store
            .pending_claim_suggestion("suggestion")
            .unwrap()
            .unwrap()
            .source_version,
        Some(2)
    );
    test.store
        .resolve_claim_suggestion("suggestion", "rejected", "2026-02-01T00:00:00Z")
        .unwrap();
    Decisions::new(test.store.clone())
        .revise(
            &id,
            DecisionEdits {
                rationale: Some("another revision".into()),
                ..DecisionEdits::default()
            },
        )
        .unwrap();
    stale.source_version = Some(3);
    assert!(!test.store.insert_claim_suggestion(&stale).unwrap());
    assert!(test
        .store
        .pending_claim_suggestion("suggestion")
        .unwrap()
        .is_none());
}

impl ProfileStore for Profiles {
    fn load(&self) -> Result<Option<AiProfile>, ProfileError> {
        Ok(Some(self.0.borrow().clone()))
    }
    fn save(&self, profile: &AiProfile) -> Result<(), ProfileError> {
        *self.0.borrow_mut() = profile.clone();
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

struct Reader(String);

impl CandidateExtractor for Reader {
    fn extract(
        &self,
        _: &DecisionEvidence,
        _: &[RelevanceSignal],
    ) -> Result<Vec<CandidateProposal>, ExtractError> {
        Ok(Vec::new())
    }
}

impl StructuredModel for Reader {
    fn complete(
        &self,
        _: &str,
        user: &str,
        schema_name: &str,
        _: &JsonValue,
    ) -> Result<String, ExtractError> {
        assert_eq!(schema_name, "decision_context");
        assert!(user.contains("Qual banco usar?"));
        Ok(support::batched_answer(&self.0))
    }
}

struct Factory(String);

impl ExtractorFactory for Factory {
    type Extractor = Reader;
    fn external(&self, _: &AiProfile, _: String) -> Result<Reader, ExtractError> {
        Ok(Reader(self.0.clone()))
    }
}

#[test]
fn derived_context_becomes_a_scoped_rule_and_is_flagged_when_its_decision_is_replaced() {
    let test = support::open("claim-suggestions", &["p1"]);
    // support::decision writes the rationale "motivo de <key>".
    let decision = support::decision(
        &test.store,
        "p1",
        "db",
        "Qual banco usar?",
        "SQLite embutido",
    );
    let graph = KnowledgeGraph::new(test.store.clone());
    let storage = graph
        .create_entity(NewEntity {
            project_id: "p1".into(),
            kind: Some(EntityKind::Component),
            name: "storage".into(),
            patterns: vec!["crates/storage/**".into()],
            ..NewEntity::default()
        })
        .expect("component")
        .entity_id;
    graph
        .link(LinkRequest {
            kind: EdgeKind::Affects,
            source_kind: NodeKind::Decision,
            source_id: decision.clone(),
            entity_id: storage.clone(),
        })
        .expect("link");

    let answer = r#"{"claims":[
        {"kind":"constraint","statement":"Os dados ficam num arquivo SQLite embutido.",
         "quote":"SQLite embutido"},
        {"kind":"assumption","statement":"Ninguém usa servidor.","quote":"texto que não existe"},
        {"kind":"convention","statement":"Banco versionado.","quote":"motivo de db"}
    ]}"#;
    let finder = ClaimFinder::new(
        test.store.clone(),
        AiSettings::new(Profiles(RefCell::new(consented())), NoSecrets),
        Factory(answer.into()),
    );
    assert_eq!(
        finder.run(&decision).expect("derive"),
        2,
        "the unquoted one is dropped"
    );
    assert_eq!(finder.run(&decision).expect("again"), 0, "never twice");

    let suggestions = ClaimSuggestions::new(test.store.clone());
    let pending = suggestions.pending("p1").expect("pending");
    assert_eq!(pending.len(), 2);
    assert_eq!(pending[0].decision_question, "Qual banco usar?");
    assert_eq!(
        pending[0].scope,
        vec![(storage.clone(), "storage".to_string())]
    );

    let constraint = pending
        .iter()
        .find(|view| view.record.kind == ClaimKind::Constraint)
        .expect("constraint");
    let claim_id = suggestions
        .confirm(&constraint.record.suggestion_id)
        .expect("confirm");
    let claims = Claims::new(test.store.clone())
        .list("p1", None)
        .expect("claims");
    let claim = claims
        .iter()
        .find(|claim| claim.claim_id == claim_id)
        .expect("created");
    assert_eq!(claim.source_decision_id.as_deref(), Some(decision.as_str()));
    let detail = graph.entity_detail(&storage, None).expect("storage");
    assert_eq!(detail.claims.len(), 1, "it applies where the decision does");

    let other = pending
        .iter()
        .find(|view| view.record.kind == ClaimKind::Convention)
        .expect("convention");
    suggestions
        .reject(&other.record.suggestion_id)
        .expect("reject");
    assert!(suggestions.pending("p1").expect("none").is_empty());
    assert_eq!(finder.run(&decision).expect("rejected stays"), 0);

    // Replacing the source decision flags the rule for review.
    assert!(suggestions.to_review("p1").expect("review").is_empty());
    let newer = support::decision(
        &test.store,
        "p1",
        "pg",
        "Qual banco usar agora?",
        "Postgres",
    );
    DecisionRelations::new(test.store.clone())
        .supersede(&newer, &decision)
        .expect("supersede");
    assert_eq!(suggestions.to_review("p1").expect("review"), vec![claim_id]);

    // Without an enabled provider nothing is derived.
    let offline = ClaimFinder::new(
        test.store.clone(),
        AiSettings::new(Profiles(RefCell::new(offline_default_profile())), NoSecrets),
        Factory(answer.into()),
    );
    assert_eq!(offline.run(&newer).expect("offline"), 0);
}

/// A pending suggestion whose `source_version` is NULL, as 0027 left the rows
/// that existed before it.
fn legacy_suggestion(tag: &str, revise: bool) -> (support::TestStore, rusqlite::Connection) {
    use application::claim_suggestions::{ClaimSuggestionRecord, ClaimSuggestionStore};
    use application::decisions::{DecisionEdits, Decisions};
    let test = support::open(tag, &["p1"]);
    let id = support::decision(&test.store, "p1", "source", "Question", "Choice");
    test.store
        .insert_claim_suggestion(&ClaimSuggestionRecord {
            suggestion_id: "legacy".into(),
            project_id: "p1".into(),
            decision_id: id.clone(),
            kind: ClaimKind::Constraint,
            statement: "Rule".into(),
            quote: "Choice".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            qualifiers: "[]".into(),
            inherited_scope: "[]".into(),
            source_version: Some(1),
        })
        .unwrap();
    if revise {
        Decisions::new(test.store.clone())
            .revise(
                &id,
                DecisionEdits {
                    choice: Some("Changed choice".into()),
                    ..DecisionEdits::default()
                },
            )
            .unwrap();
    }
    let db = rusqlite::Connection::open(test.root.join("app.db")).unwrap();
    db.execute("UPDATE claim_suggestions SET source_version = NULL", [])
        .unwrap();
    (test, db)
}

const BACKFILL: &str = include_str!("../src/migrations/0041_backfill_claim_suggestion_version.sql");

#[test]
fn backfill_makes_a_legacy_suggestion_of_an_unrevised_decision_confirmable() {
    let (test, db) = legacy_suggestion("backfill-v1", false);
    assert!(ClaimSuggestions::new(test.store.clone())
        .confirm("legacy")
        .is_err());
    db.execute_batch(BACKFILL).unwrap();
    ClaimSuggestions::new(test.store.clone())
        .confirm("legacy")
        .expect("confirmable after the backfill");
}

#[test]
fn backfill_leaves_a_revised_decision_whose_quote_is_gone_untouched() {
    let (test, db) = legacy_suggestion("backfill-v2", true);
    db.execute_batch(BACKFILL).unwrap();
    let version: Option<i64> = db
        .query_row("SELECT source_version FROM claim_suggestions", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(version, None);
    assert!(ClaimSuggestions::new(test.store.clone())
        .confirm("legacy")
        .is_err());
}

#[test]
fn a_rule_that_names_a_component_applies_only_there_and_a_pending_tie_is_not_inherited() {
    use application::graph::{EdgeRecord, GraphStore};
    use domain::entities::{EdgeActor, EdgeOrigin};
    let test = support::open("claim-scope-text", &["p1"]);
    let decision = support::decision(
        &test.store,
        "p1",
        "wal",
        "Qual banco usar?",
        "Gravar na outbox com SQLite embutido",
    );
    let graph = KnowledgeGraph::new(test.store.clone());
    let make = |name: &str, pattern: &str| {
        graph
            .create_entity(NewEntity {
                project_id: "p1".into(),
                kind: Some(EntityKind::Component),
                name: name.into(),
                patterns: vec![pattern.into()],
                ..NewEntity::default()
            })
            .expect("component")
            .entity_id
    };
    let storage = make("storage", "crates/storage/**");
    let outbox = make("outbox", "adapters/outbox/**");
    let cache = make("cache", "crates/cache/**");
    for entity in [&storage, &outbox] {
        graph
            .link(LinkRequest {
                kind: EdgeKind::Affects,
                source_kind: NodeKind::Decision,
                source_id: decision.clone(),
                entity_id: entity.clone(),
            })
            .expect("link");
    }
    // A suggestion nobody confirmed is not a tie.
    test.store
        .insert_edge(&EdgeRecord {
            edge_id: "pending-cache".into(),
            project_id: "p1".into(),
            kind: EdgeKind::Affects,
            source_kind: NodeKind::Decision,
            source_id: decision.clone(),
            entity_id: cache.clone(),
            origin: EdgeOrigin::Derived,
            reason: "citado no texto: \"cache\"".into(),
            created_at: "2026-01-03T00:00:00Z".into(),
            confirmed_at: None,
            invalidated_at: None,
            confirmed_by: None,
            invalidated_by: None,
        })
        .expect("pending");

    let answer = r#"{"claims":[
        {"kind":"constraint","statement":"A outbox grava antes de responder.",
         "quote":"Gravar na outbox"},
        {"kind":"constraint","statement":"Os dados ficam num arquivo embutido.",
         "quote":"SQLite embutido"}
    ]}"#;
    let finder = ClaimFinder::new(
        test.store.clone(),
        AiSettings::new(Profiles(RefCell::new(consented())), NoSecrets),
        Factory(answer.into()),
    );
    assert_eq!(finder.run(&decision).expect("derive"), 2);
    let suggestions = ClaimSuggestions::new(test.store.clone());
    let pending = suggestions.pending("p1").expect("pending");
    let scope_of = |text: &str| -> Vec<String> {
        let view = pending
            .iter()
            .find(|view| view.record.statement.contains(text))
            .expect("suggestion");
        let mut names: Vec<String> = view.scope.iter().map(|(_, name)| name.clone()).collect();
        names.sort();
        names
    };
    assert_eq!(scope_of("outbox"), vec!["outbox"], "it names the outbox");
    assert_eq!(
        scope_of("arquivo"),
        vec!["outbox", "storage"],
        "it names nothing: every confirmed tie, and not the pending one"
    );

    let named = pending
        .iter()
        .find(|view| view.record.statement.contains("outbox"))
        .expect("named");
    let claim_id = suggestions
        .confirm(&named.record.suggestion_id)
        .expect("confirm");
    let edges: Vec<EdgeRecord> = test
        .store
        .project_edges("p1")
        .expect("edges")
        .into_iter()
        .filter(|edge| edge.source_id == claim_id)
        .collect();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].entity_id, outbox);
    assert_eq!(edges[0].origin, EdgeOrigin::Derived);
    assert_eq!(edges[0].confirmed_by, Some(EdgeActor::Inherited));
}
