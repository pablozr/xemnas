//! Links the AI proposes from an adopted decision to the map's components,
//! over SQLite with a fake model: when the job is queued, when it calls the
//! model, which answers survive and what the review sees.

mod support;

use std::cell::{Cell, RefCell};

use application::adoption::Adoption;
use application::analysis::ExtractorFactory;
use application::extract::{
    CandidateExtractor, CandidateProposal, DecisionCandidateRecord, DecisionEvidence, ExtractError,
    ExtractionStore, RelevanceSignal,
};
use application::graph::{ai_link_quote, ai_link_why, GraphStore, KnowledgeGraph, NewEntity};
use application::link_suggestions::{LinkFindError, LinkFinder, LINK_JOB_KIND};
use application::overview::{JsonValue, StructuredModel};
use application::profile::{
    build_preview, grant_consent, offline_default_profile, AiProfile, AiSettings, ProfileError,
    ProfileKind, ProfileStore, SecretStore,
};
use domain::entities::{EdgeKind, EntityKind};
use storage_sqlite::SqliteStore;

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

/// Answers with a fixed text, or a rate limit, and counts calls and requests.
struct Proposer<'a> {
    answer: Result<&'a str, ()>,
    calls: &'a Cell<usize>,
    asked: &'a RefCell<Vec<String>>,
}

impl CandidateExtractor for Proposer<'_> {
    fn extract(
        &self,
        _: &DecisionEvidence,
        _: &[RelevanceSignal],
    ) -> Result<Vec<CandidateProposal>, ExtractError> {
        Ok(Vec::new())
    }
}

impl StructuredModel for Proposer<'_> {
    fn complete(
        &self,
        _: &str,
        user: &str,
        schema_name: &str,
        _: &JsonValue,
    ) -> Result<String, ExtractError> {
        assert_eq!(schema_name, "decision_links");
        self.calls.set(self.calls.get() + 1);
        self.asked.borrow_mut().push(user.to_owned());
        self.answer
            .map(str::to_owned)
            .map_err(|()| ExtractError::RateLimited { retry_after: None })
    }
}

struct Factory<'a> {
    answer: Result<&'a str, ()>,
    calls: &'a Cell<usize>,
    asked: &'a RefCell<Vec<String>>,
}

impl<'a> ExtractorFactory for Factory<'a> {
    type Extractor = Proposer<'a>;
    fn external(&self, _: &AiProfile, _: String) -> Result<Proposer<'a>, ExtractError> {
        Ok(Proposer {
            answer: self.answer,
            calls: self.calls,
            asked: self.asked,
        })
    }
}

struct Fixture<'a> {
    calls: &'a Cell<usize>,
    asked: &'a RefCell<Vec<String>>,
}

impl<'a> Fixture<'a> {
    fn finder(
        &self,
        store: &SqliteStore,
        profile: AiProfile,
        answer: Result<&'a str, ()>,
    ) -> LinkFinder<SqliteStore, Profiles, NoSecrets, Factory<'a>> {
        LinkFinder::new(
            store.clone(),
            AiSettings::new(Profiles(RefCell::new(profile)), NoSecrets),
            Factory {
                answer,
                calls: self.calls,
                asked: self.asked,
            },
        )
    }
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

/// A decision from an ADR: its capture changed only a document.
fn adr_decision(store: &SqliteStore, key: &str, question: &str) -> String {
    support::decision_with_diff(
        store,
        "p1",
        key,
        question,
        &["docs/adr/0001-outcome.md"],
        "",
    )
}

const OUTCOME: &str = "Como o resultado de um turno é expresso?";

fn answer(component: &str, quote: &str) -> String {
    serde_json::json!({"links": [{
        "component_id": component, "quote": quote, "reason": "A decisão rege o desfecho."
    }]})
    .to_string()
}

#[test]
fn an_adr_decision_gets_a_pending_suggestion_the_review_can_see() {
    let test = support::open("links-store", &["p1"]);
    let core = component(&test.store, "engine", "packages/core/**");
    component(&test.store, "plugin", "packages/plugin/**");
    let decision = adr_decision(&test.store, "adr", OUTCOME);

    let (calls, asked) = (Cell::new(0), RefCell::new(Vec::new()));
    let fixture = Fixture {
        calls: &calls,
        asked: &asked,
    };
    // Components are numbered by name: c1 = engine, c2 = plugin. The valid
    // link quotes with other case and no accent; the others are dropped.
    let quote = answer("c1", "COMO O RESULTADO DE UM TURNO é expresso");
    let invented = answer("c2", "uma frase que a decisão nunca disse");
    let unknown = answer("c7", "Como o resultado de um turno é expresso");
    let both = serde_json::json!({"links": [
        serde_json::from_str::<serde_json::Value>(&invented).unwrap()["links"][0],
        serde_json::from_str::<serde_json::Value>(&unknown).unwrap()["links"][0],
        serde_json::from_str::<serde_json::Value>(&quote).unwrap()["links"][0],
    ]})
    .to_string();
    let finder = fixture.finder(&test.store, consented(), Ok(&both));
    assert_eq!(finder.run(&decision).expect("links"), 1);
    assert!(asked.borrow()[0].contains("- c1 | engine | paths: packages/core/**"));
    assert!(
        !asked.borrow()[0].contains(&core),
        "ids are never sent, only positions"
    );

    let graph = KnowledgeGraph::new(test.store.clone());
    let suggestions = graph.suggestions("p1").expect("suggestions");
    assert_eq!(suggestions.len(), 1);
    let suggestion = &suggestions[0];
    assert_eq!(suggestion.kind, EdgeKind::Affects);
    assert_eq!(suggestion.entity.node.id, core);
    assert_eq!(suggestion.source.node.id, decision);
    assert_eq!(
        ai_link_quote(&suggestion.reason),
        Some("COMO O RESULTADO DE UM TURNO é expresso")
    );
    assert_eq!(
        ai_link_why(&suggestion.reason),
        Some("A decisão rege o desfecho.")
    );
    let edge = &test.store.project_edges("p1").expect("edges")[0];
    assert!(edge.confirmed_at.is_none() && edge.invalidated_at.is_none());

    // The decision was asked about once: the same job again makes no call
    // and writes no duplicate.
    assert_eq!(finder.run(&decision).expect("again"), 0);
    assert_eq!(calls.get(), 1);
    assert_eq!(test.store.project_edges("p1").expect("edges").len(), 1);
}

#[test]
fn an_empty_map_or_an_offline_provider_makes_no_call() {
    let test = support::open("links-no-call", &["p1"]);
    let decision = adr_decision(&test.store, "adr", OUTCOME);
    let (calls, asked) = (Cell::new(0), RefCell::new(Vec::new()));
    let fixture = Fixture {
        calls: &calls,
        asked: &asked,
    };
    let reply = answer("c1", "Como o resultado de um turno é expresso");
    let finder = fixture.finder(&test.store, consented(), Ok(&reply));
    assert_eq!(finder.run(&decision).expect("no components"), 0);
    assert_eq!(calls.get(), 0, "an empty map completes without the AI");

    component(&test.store, "engine", "packages/core/**");
    let offline = fixture.finder(&test.store, offline_default_profile(), Ok(&reply));
    assert_eq!(offline.run(&decision).expect("offline"), 0);
    assert_eq!(calls.get(), 0);
}

#[test]
fn a_decision_tied_by_a_file_or_a_person_makes_no_call() {
    let test = support::open("links-tied", &["p1"]);
    component(&test.store, "engine", "packages/core/**");
    let by_file = support::decision_with_diff(
        &test.store,
        "p1",
        "code",
        "Onde guardar o estado?",
        &["packages/core/src/state.ts"],
        "",
    );
    let graph = KnowledgeGraph::new(test.store.clone());
    graph.refresh_suggestions("p1").expect("refresh");
    let (calls, asked) = (Cell::new(0), RefCell::new(Vec::new()));
    let fixture = Fixture {
        calls: &calls,
        asked: &asked,
    };
    let reply = answer("c1", "Onde guardar o estado");
    let finder = fixture.finder(&test.store, consented(), Ok(&reply));
    assert_eq!(finder.run(&by_file).expect("tied by file"), 0);
    assert_eq!(calls.get(), 0);
}

#[test]
fn an_edge_a_person_invalidated_is_not_suggested_again() {
    let test = support::open("links-invalidated", &["p1"]);
    let engine = component(&test.store, "engine", "packages/core/**");
    let plugin = component(&test.store, "plugin", "packages/plugin/**");
    // The text names the engine, so the map first suggests it by mention and
    // the person turns it down.
    let decision = adr_decision(&test.store, "adr", "Como o engine expressa o resultado?");
    let graph = KnowledgeGraph::new(test.store.clone());
    graph.refresh_suggestions("p1").expect("refresh");
    let mention = graph.suggestions("p1").expect("suggestions");
    assert_eq!(mention.len(), 1);
    graph.invalidate(&mention[0].edge_id).expect("rejected");

    let (calls, asked) = (Cell::new(0), RefCell::new(Vec::new()));
    let fixture = Fixture {
        calls: &calls,
        asked: &asked,
    };
    let reply = serde_json::json!({"links": [
        {"component_id": "c1", "quote": "Como o engine expressa o resultado",
         "reason": "de novo"},
        {"component_id": "c2", "quote": "Como o engine expressa o resultado",
         "reason": "outro"},
    ]})
    .to_string();
    let finder = fixture.finder(&test.store, consented(), Ok(&reply));
    assert_eq!(finder.run(&decision).expect("links"), 1);
    let pending = graph.suggestions("p1").expect("pending");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].entity.node.id, plugin);
    assert!(
        pending.iter().all(|s| s.entity.node.id != engine),
        "the rejected engine edge stays rejected"
    );
}

#[test]
fn a_rate_limit_defers_the_job() {
    let test = support::open("links-deferred", &["p1"]);
    component(&test.store, "engine", "packages/core/**");
    let decision = adr_decision(&test.store, "adr", OUTCOME);
    let (calls, asked) = (Cell::new(0), RefCell::new(Vec::new()));
    let fixture = Fixture {
        calls: &calls,
        asked: &asked,
    };
    let finder = fixture.finder(&test.store, consented(), Err(()));
    assert_eq!(
        finder.run(&decision),
        Err(LinkFindError::Deferred(None)),
        "the provider asked for a pause"
    );
    assert!(test.store.project_edges("p1").expect("edges").is_empty());
}

/// A pending candidate of `p1` whose evidence cites `files`.
fn candidate(store: &SqliteStore, id: &str, files: &[&str]) {
    let files: Vec<String> = files.iter().map(|file| format!("\"{file}\"")).collect();
    store
        .insert_candidates(&[DecisionCandidateRecord {
            qualifiers: "[]".into(),
            id: id.into(),
            project_id: "p1".into(),
            capture_id: "capture-p1".into(),
            status: "pending".into(),
            question: format!("Pergunta de {id}?"),
            choice: "Escolha.".into(),
            rationale: "Motivo.".into(),
            signals: "[\"public_contract\"]".into(),
            confidence: 0.8,
            confidence_reason: "sintético".into(),
            evidence_refs: "[\"art-p1\"]".into(),
            diff_summary: format!("{{\"files\":[{}],\"artifacts\":1}}", files.join(",")),
            dedup_hash: format!("dedup-{id}"),
            created_at: "2026-01-02T00:00:00Z".into(),
            updated_at: "2026-01-02T00:00:00Z".into(),
            kind: "decision".into(),
            significance: 0.8,
            criteria: "[]".into(),
        }])
        .expect("candidate");
}

fn queued(test: &support::TestStore, decision: &str) -> i64 {
    rusqlite::Connection::open(test.root.join("app.db"))
        .expect("raw")
        .query_row(
            "SELECT COUNT(*) FROM jobs WHERE kind = ?1 AND payload = ?2",
            [LINK_JOB_KIND, decision],
            |row| row.get(0),
        )
        .expect("jobs")
}

#[test]
fn adoption_queues_the_job_only_for_a_decision_no_file_tied_to_the_map() {
    let test = support::open("links-adoption", &["p1"]);
    component(&test.store, "engine", "packages/core/**");
    candidate(&test.store, "from-code", &["packages/core/src/lib.ts"]);
    candidate(&test.store, "from-adr", &["docs/adr/0001-outcome.md"]);
    let adoption = Adoption::new(test.store.clone());

    let preview = adoption.preview("from-code").expect("preview");
    assert_eq!(preview.links.len(), 1);
    let by_code = adoption
        .adopt("from-code", None, &preview.links, &[])
        .expect("adopt");
    assert_eq!(by_code.linked, 1);
    assert_eq!(
        queued(&test, &by_code.id),
        0,
        "tied by a file: no AI needed"
    );

    let by_adr = adoption.adopt("from-adr", None, &[], &[]).expect("adopt");
    assert_eq!(by_adr.linked, 0);
    assert_eq!(queued(&test, &by_adr.id), 1, "nothing tied it to the map");
}
