//! Suggested relations over SQLite: candidates from the map and words, the
//! judge's answer checked against the decisions' text, confirmation into a
//! real relation, rejection that never returns.

mod support;

use std::cell::RefCell;

use application::analysis::ExtractorFactory;
use application::extract::{
    CandidateExtractor, CandidateProposal, DecisionEvidence, ExtractError, RelevanceSignal,
};
use application::graph::{KnowledgeGraph, LinkRequest, NewEntity};
use application::injection::short_ref;
use application::overview::{JsonValue, StructuredModel};
use application::profile::{
    build_preview, grant_consent, offline_default_profile, AiProfile, AiSettings, ProfileError,
    ProfileKind, ProfileStore, SecretStore,
};
use application::relation_suggestions::{RelationFinder, RelationSuggestions};
use application::relations::DecisionRelations;
use domain::entities::{EdgeKind, EntityKind, NodeKind};
use domain::relations::RelationKind;

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

struct Judge(String);

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
        assert_eq!(schema_name, "decision_relations");
        assert!(
            user.contains("Qual banco usar?"),
            "the shared component brings it"
        );
        Ok(self.0.clone())
    }
}

struct Factory(String);

impl ExtractorFactory for Factory {
    type Extractor = Judge;
    fn external(&self, _: &AiProfile, _: String) -> Result<Judge, ExtractError> {
        Ok(Judge(self.0.clone()))
    }
}

#[test]
fn a_judged_relation_waits_for_confirmation_and_a_rejected_one_never_returns() {
    let test = support::open("relation-suggestions", &["p1"]);
    let older = support::decision(&test.store, "p1", "db", "Qual banco usar?", "SQLite");
    let newer = support::decision(
        &test.store,
        "p1",
        "queue",
        "Onde guardar a fila?",
        "Na mesma base das decisões",
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
    for decision in [&older, &newer] {
        graph
            .link(LinkRequest {
                kind: EdgeKind::Affects,
                source_kind: NodeKind::Decision,
                source_id: decision.clone(),
                entity_id: storage.clone(),
            })
            .expect("link");
    }
    // The support decisions' rationale is "motivo de <key>".
    let answer = format!(
        r#"{{"relations":[{{"earlier":"D:{}","relation":"depends_on",
            "direction":"new_to_earlier","quote":"Na mesma base das decisões",
            "reason":"A fila usa o banco escolhido."}}]}}"#,
        short_ref(&older)
    );
    let finder = RelationFinder::new(
        test.store.clone(),
        AiSettings::new(Profiles(RefCell::new(consented())), NoSecrets),
        Factory(answer.clone()),
    );
    assert_eq!(finder.run(&newer).expect("judge"), 1);
    assert_eq!(
        finder.run(&newer).expect("again"),
        0,
        "never suggested twice"
    );

    let suggestions = RelationSuggestions::new(test.store.clone());
    let pending = suggestions.pending("p1").expect("pending");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].record.kind, RelationKind::DependsOn);
    assert_eq!(pending[0].from_question, "Onde guardar a fila?");
    assert_eq!(pending[0].to_question, "Qual banco usar?");
    assert!(
        DecisionRelations::new(test.store.clone())
            .of(&newer)
            .expect("none yet")
            .is_empty(),
        "a suggestion is not a relation"
    );

    suggestions
        .confirm(&pending[0].record.suggestion_id)
        .expect("confirm");
    let relations = DecisionRelations::new(test.store.clone())
        .of(&newer)
        .expect("relations");
    assert_eq!(relations.len(), 1);
    assert_eq!(relations[0].kind, RelationKind::DependsOn);
    assert!(suggestions.pending("p1").expect("resolved").is_empty());

    // A rejected suggestion does not come back.
    let third = support::decision(
        &test.store,
        "p1",
        "cache",
        "Como cachear?",
        "Na mesma base das decisões",
    );
    graph
        .link(LinkRequest {
            kind: EdgeKind::Affects,
            source_kind: NodeKind::Decision,
            source_id: third.clone(),
            entity_id: storage.clone(),
        })
        .expect("link third");
    let finder = RelationFinder::new(
        test.store.clone(),
        AiSettings::new(Profiles(RefCell::new(consented())), NoSecrets),
        Factory(answer),
    );
    assert_eq!(finder.run(&third).expect("judge third"), 1);
    let id = suggestions.pending("p1").expect("p")[0]
        .record
        .suggestion_id
        .clone();
    suggestions.reject(&id).expect("reject");
    assert_eq!(finder.run(&third).expect("again"), 0);
    assert!(suggestions.pending("p1").expect("empty").is_empty());

    // Without an enabled provider nothing is judged.
    let offline = RelationFinder::new(
        test.store.clone(),
        AiSettings::new(Profiles(RefCell::new(offline_default_profile())), NoSecrets),
        Factory(String::new()),
    );
    assert_eq!(offline.run(&newer).expect("offline"), 0);
}
