//! Cost gate of the links the extractor names: when it says, with a verbatim
//! quote, which components a candidate applies to, the adoption turns that
//! into suggestions of the AI's kind and the separate link call has nothing
//! left to ask (zero calls); without it, one call. The model is a fake that
//! counts calls; extraction, adoption, graph and jobs are the real ones.

mod support;

use std::sync::{Arc, Mutex};

use application::adoption::Adoption;
use application::analysis::ExtractorFactory;
use application::documents::{Documents, DOCUMENT_ADAPTER};
use application::extract::{
    run_extraction, CandidateExtractor, CandidateKind, CandidateProposal, DecisionEvidence,
    ExtractError, ProposedComponent, RelevanceSignal, RunContext,
};
use application::graph::{ai_link_why, GraphStore, KnowledgeGraph, NewEntity, EXTRACTED_LINK_WHY};
use application::inbox::CandidateEdits;
use application::link_suggestions::LinkFinder;
use application::overview::{JsonValue, StructuredModel};
use application::profile::{
    build_preview, grant_consent, offline_default_profile, AiProfile, AiSettings, ProfileError,
    ProfileKind, ProfileStore, SecretStore,
};
use application::projects::{ProjectRecord, ProjectRepository};
use domain::entities::{EdgeActor, EntityKind};
use rusqlite::Connection;

struct Profiles(Mutex<AiProfile>);

impl ProfileStore for Profiles {
    fn load(&self) -> Result<Option<AiProfile>, ProfileError> {
        Ok(Some(self.0.lock().expect("profile").clone()))
    }
    fn save(&self, profile: &AiProfile) -> Result<(), ProfileError> {
        *self.0.lock().expect("profile") = profile.clone();
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

fn settings() -> AiSettings<Profiles, NoSecrets> {
    let profile = AiProfile {
        kind: ProfileKind::OpenAiCompatible,
        model: "local".into(),
        endpoint: Some("http://127.0.0.1:9/v1".into()),
        ..offline_default_profile()
    };
    let profile = grant_consent(
        &profile,
        &build_preview(&profile),
        "2026-01-01T00:00:00Z",
        true,
    )
    .expect("consent");
    AiSettings::new(Profiles(Mutex::new(profile)), NoSecrets)
}

const QUESTION: &str = "Onde guardar as gravações?";
const CHOICE: &str = "Gravar cada captura numa fila local antes de responder.";
const RATIONALE: &str = "Evita perder eventos se o processo cair.";

/// The extractor under test: one decision that may name components.
struct Scripted(Vec<ProposedComponent>);

impl CandidateExtractor for Scripted {
    fn extract(
        &self,
        input: &DecisionEvidence,
        signals: &[RelevanceSignal],
    ) -> Result<Vec<CandidateProposal>, ExtractError> {
        Ok(vec![CandidateProposal {
            nature: application::review_exception::CandidateNature::Unknown,
            qualifiers: Vec::new(),
            question: QUESTION.into(),
            choice: CHOICE.into(),
            rationale: RATIONALE.into(),
            confidence: 0.9,
            confidence_reason: "O ADR registra a escolha.".into(),
            signals: signals.to_vec(),
            evidence_refs: input
                .artifacts
                .iter()
                .map(|artifact| artifact.artifact_id.clone())
                .collect(),
            diff_summary: String::new(),
            kind: CandidateKind::Decision,
            significance: 0.8,
            criteria: vec!["data_or_contract".into()],
            components: self.0.clone(),
        }])
    }
}

/// The link proposer's model: counts its calls and links to the only
/// component by quoting the choice.
#[derive(Clone, Default)]
struct Model(Arc<Mutex<usize>>);

impl StructuredModel for Model {
    fn complete(
        &self,
        _: &str,
        user: &str,
        schema_name: &str,
        _: &JsonValue,
    ) -> Result<String, ExtractError> {
        assert_eq!(schema_name, "decision_links");
        *self.0.lock().expect("calls") += 1;
        let choice = user
            .lines()
            .find_map(|line| line.strip_prefix("Choice: "))
            .expect("choice");
        Ok(serde_json::json!({"decisions": [{
            "id": "1",
            "links": [{"component_id": "c1", "quote": choice, "reason": "Rege o componente."}]
        }]})
        .to_string())
    }
}

impl CandidateExtractor for Model {
    fn extract(
        &self,
        _: &DecisionEvidence,
        _: &[RelevanceSignal],
    ) -> Result<Vec<CandidateProposal>, ExtractError> {
        Ok(Vec::new())
    }
}

#[derive(Clone)]
struct Factory(Model);

impl ExtractorFactory for Factory {
    type Extractor = Model;
    fn external(&self, _: &AiProfile, _: String) -> Result<Model, ExtractError> {
        Ok(self.0.clone())
    }
}

/// What a run left: the links the extractor's components became and the calls
/// the separate link proposer made afterwards.
struct Outcome {
    extracted: Vec<String>,
    link_calls: usize,
}

/// Extracts the ADR of a project whose map has two components, adopts the
/// candidate as the rules would, and runs the link job it may have queued.
fn run(tag: &str, components: Vec<ProposedComponent>, edits: Option<CandidateEdits>) -> Outcome {
    let test = support::open(tag, &[]);
    let repo = test.root.join("repo");
    std::fs::create_dir_all(repo.join("docs/adr")).expect("docs");
    std::fs::write(
        repo.join("docs/adr/0001-fila.md"),
        format!("# Fila\n\n## Decisão\n{CHOICE}\n"),
    )
    .expect("adr");
    test.store
        .insert(&ProjectRecord::new(
            "docs-p".into(),
            repo.to_string_lossy().replace('\\', "/"),
            "2026-01-01T00:00:00Z".into(),
        ))
        .expect("project");
    let graph = KnowledgeGraph::new(test.store.clone());
    let make = |name: &str| {
        graph
            .create_entity(NewEntity {
                project_id: "docs-p".into(),
                kind: Some(EntityKind::Component),
                name: name.into(),
                patterns: vec![format!("crates/{name}/**")],
                ..NewEntity::default()
            })
            .expect("component")
            .entity_id
    };
    let outbox = make("outbox");
    make("billing");

    let documents = Documents::new(test.store.clone());
    documents.index("docs-p").expect("index");
    documents.propose("docs-p", 10).expect("propose");
    let connection = Connection::open(test.root.join("app.db")).expect("raw");
    let capture: String = connection
        .query_row(
            "SELECT capture_id FROM adapter_checkpoints WHERE adapter = ?1",
            [DOCUMENT_ADAPTER],
            |row| row.get(0),
        )
        .expect("capture");
    let report = run_extraction(
        &test.store,
        &Scripted(components),
        &capture,
        &RunContext::for_tests(),
    )
    .expect("extract");
    assert_eq!(report.inserted, 1);
    let candidate: String = connection
        .query_row("SELECT id FROM decision_candidates", [], |row| row.get(0))
        .expect("candidate");

    let adopted = Adoption::new(test.store.clone())
        .adopt_as(&candidate, edits, &[], &[], EdgeActor::Rules)
        .expect("adopt");
    let edges = test.store.project_edges("docs-p").expect("edges");
    let extracted = edges
        .iter()
        .filter(|edge| {
            edge.source_id == adopted.id
                && edge.is_pending()
                && ai_link_why(&edge.reason) == Some(EXTRACTED_LINK_WHY)
        })
        .map(|edge| edge.entity_id.clone())
        .collect::<Vec<_>>();
    assert!(extracted.iter().all(|id| *id == outbox));

    let model = Model::default();
    let finder = LinkFinder::new(test.store.clone(), settings(), Factory(model.clone()));
    for result in finder.run_many(&[adopted.id.as_str()]) {
        result.expect("link job");
    }
    let link_calls = *model.0.lock().expect("calls");
    Outcome {
        extracted,
        link_calls,
    }
}

fn outbox_by_quote() -> Vec<ProposedComponent> {
    vec![ProposedComponent {
        name: "Outbox".into(),
        quote: "Gravar cada captura numa fila local".into(),
    }]
}

#[test]
fn the_extractors_components_spare_the_separate_link_call() {
    let with = run("extracted-with", outbox_by_quote(), None);
    assert_eq!(with.extracted.len(), 1, "one pending link of the AI's kind");
    assert_eq!(with.link_calls, 0, "nothing left to ask");

    let without = run("extracted-without", Vec::new(), None);
    assert!(without.extracted.is_empty());
    assert_eq!(without.link_calls, 1, "the proposer is asked once");

    println!(
        "gate extracted links: link_calls_with_extractor={} link_calls_without={} \
         extracted_links={}",
        with.link_calls,
        without.link_calls,
        with.extracted.len()
    );
}

#[test]
fn an_edit_that_removes_the_quote_removes_the_link() {
    let edits = CandidateEdits {
        qualifiers: Vec::new(),
        question: QUESTION.into(),
        choice: "Gravar tudo direto no disco.".into(),
        rationale: RATIONALE.into(),
    };
    let edited = run("extracted-edited", outbox_by_quote(), Some(edits));
    assert!(edited.extracted.is_empty(), "the quote is not in the text");
    assert_eq!(edited.link_calls, 1, "so the proposer is asked");
}

#[test]
fn a_name_the_map_lacks_or_a_paraphrase_never_becomes_a_link() {
    let components = vec![
        ProposedComponent {
            name: "queue-service".into(),
            quote: "Gravar cada captura numa fila local".into(),
        },
        ProposedComponent {
            name: "outbox".into(),
            quote: "gravar capturas em uma fila".into(),
        },
    ];
    let outcome = run("extracted-invalid", components, None);
    assert!(outcome.extracted.is_empty());
    assert_eq!(outcome.link_calls, 1);
}
