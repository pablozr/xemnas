//! Project overview over SQLite: what the model receives, what survives its
//! answer, persistence and staleness.

mod support;

use std::cell::RefCell;

use application::analysis::ExtractorFactory;
use application::claims::{Claims, NewClaim};
use application::extract::{
    CandidateExtractor, CandidateProposal, DecisionEvidence, ExtractError, RelevanceSignal,
};
use application::graph::{KnowledgeGraph, LinkRequest, NewEntity};
use application::injection::short_ref;
use application::overview::{JsonValue, OverviewError, ProjectOverviews, StructuredModel};
use application::profile::{
    build_preview, grant_consent, offline_default_profile, AiProfile, AiSettings, ProfileError,
    ProfileKind, ProfileStore, SecretStore,
};
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

/// A local model on loopback with consent: needs no key.
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

struct Scripted(String);

impl CandidateExtractor for Scripted {
    fn extract(
        &self,
        _: &DecisionEvidence,
        _: &[RelevanceSignal],
    ) -> Result<Vec<CandidateProposal>, ExtractError> {
        Ok(Vec::new())
    }
}

impl StructuredModel for Scripted {
    fn complete(
        &self,
        system: &str,
        user: &str,
        schema_name: &str,
        _: &JsonValue,
    ) -> Result<String, ExtractError> {
        assert_eq!(schema_name, "project_overview");
        assert!(system.contains("never see the code"));
        assert!(
            user.contains("Qual banco usar?"),
            "decisions go to the model"
        );
        assert!(user.contains("Erros em português"), "rules go to the model");
        assert!(user.contains("\"storage\""), "the map goes to the model");
        Ok(self.0.clone())
    }
}

struct Factory(String);

impl ExtractorFactory for Factory {
    type Extractor = Scripted;
    fn external(&self, _: &AiProfile, _: String) -> Result<Scripted, ExtractError> {
        Ok(Scripted(self.0.clone()))
    }
}

#[test]
fn an_overview_cites_the_records_and_knows_when_it_is_stale() {
    let test = support::open("overview", &["p1"]);
    let decision = support::decision(&test.store, "p1", "db", "Qual banco usar?", "SQLite");
    let claim = Claims::new(test.store.clone())
        .create(NewClaim {
            project_id: "p1".into(),
            kind: ClaimKind::Convention,
            statement: "Erros em português".into(),
            valid_from: Some("2020-01-01".into()),
            valid_until: None,
            source_decision_id: None,
        })
        .expect("claim");
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

    let d = format!("D:{}", short_ref(&decision));
    let r = format!("R:{}", short_ref(&claim.claim_id));
    let answer = format!(
        r#"{{"summary":[{{"text":"App local com banco embutido.","refs":["{d}"]}},
              {{"text":"Sem fonte.","refs":[]}}],
            "flows":[{{"title":"Gravar uma decisão","description":"x","steps":[
              {{"title":"Persistir","text":"SQLite","component":"storage","refs":["{d}"]}},
              {{"title":"Responder","text":"mensagem","component":null,"refs":["{r}"]}}]}}]}}"#
    );
    let overviews = ProjectOverviews::new(
        test.store.clone(),
        AiSettings::new(Profiles(RefCell::new(consented())), NoSecrets),
        Factory(answer),
    );

    assert_eq!(overviews.current("p1").expect("none yet"), None);
    let view = overviews.generate("p1").expect("generate");
    assert_eq!(view.overview.summary.len(), 1, "uncited text is dropped");
    assert_eq!(
        view.overview.flows[0].steps[0].entity_id.as_deref(),
        Some(storage.as_str())
    );
    assert_eq!(view.overview.decisions, 1);
    assert_eq!(view.overview.rules, 1);

    let stored = overviews.current("p1").expect("current").expect("stored");
    assert_eq!(stored.overview, view.overview);
    assert_eq!(stored.new_decisions, 0);

    // The page carries the decision, tied to the container it affects, and
    // the rule.
    let knowledge = overviews
        .knowledge("p1", &view.overview.architecture)
        .expect("knowledge");
    assert_eq!(knowledge.decisions.len(), 1);
    assert_eq!(knowledge.decisions[0].label, d);
    assert_eq!(knowledge.decisions[0].choice, "SQLite");
    assert_eq!(knowledge.decisions[0].containers, vec![storage.clone()]);
    assert_eq!(knowledge.rules[0].label, r);
    let page = overviews.page("p1", "xemnas").expect("page");
    assert!(page.contains(&format!("\"label\":\"{d}\"")));
    assert!(
        !page.contains("App local com banco embutido."),
        "no summary"
    );

    // Decisions confirmed later make it stale (confirmation time is after
    // the generation second).
    std::thread::sleep(std::time::Duration::from_millis(1_100));
    support::decision(
        &test.store,
        "p1",
        "cache",
        "Como fazer cache?",
        "Em memória",
    );
    assert_eq!(
        overviews
            .current("p1")
            .expect("current")
            .expect("stored")
            .new_decisions,
        1
    );

    let offline = ProjectOverviews::new(
        test.store.clone(),
        AiSettings::new(Profiles(RefCell::new(offline_default_profile())), NoSecrets),
        Factory(String::new()),
    );
    assert_eq!(
        offline.generate("p1").map(|_| ()),
        Err(OverviewError::ProviderOff)
    );
}

#[test]
fn nothing_recorded_means_nothing_to_summarize() {
    let test = support::open("overview-empty", &["p1"]);
    let overviews = ProjectOverviews::new(
        test.store.clone(),
        AiSettings::new(Profiles(RefCell::new(consented())), NoSecrets),
        Factory(String::new()),
    );
    assert_eq!(
        overviews.generate("p1").map(|_| ()),
        Err(OverviewError::NothingRecorded)
    );
}
