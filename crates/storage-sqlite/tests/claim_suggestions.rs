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
        Ok(self.0.clone())
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
