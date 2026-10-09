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

/// The extractor under test: one decision (its text, then the components it
/// names).
struct Scripted(&'static Text, Vec<ProposedComponent>);

/// The question, choice and rationale of the scripted decision.
struct Text {
    question: &'static str,
    choice: &'static str,
    rationale: &'static str,
}

const QUEUE: Text = Text {
    question: QUESTION,
    choice: CHOICE,
    rationale: RATIONALE,
};

/// The updater of the real project: nothing in it names `sc-platform`.
const UPDATER: Text = Text {
    question: "How should the updater discover releases and select its update channel?",
    choice: "Poll the release feed daily and let the user pick stable or beta.",
    rationale: "The tray menu needs to offer updates without restarting the app.",
};

impl CandidateExtractor for Scripted {
    fn extract(
        &self,
        input: &DecisionEvidence,
        signals: &[RelevanceSignal],
    ) -> Result<Vec<CandidateProposal>, ExtractError> {
        Ok(vec![CandidateProposal {
            nature: application::review_exception::CandidateNature::Unknown,
            qualifiers: Vec::new(),
            question: self.0.question.into(),
            choice: self.0.choice.into(),
            rationale: self.0.rationale.into(),
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
            components: self.1.clone(),
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
    /// The names of the components the extractor's links point at.
    extracted: Vec<String>,
    link_calls: usize,
    /// `components_listed/proposed/kept/unknown/unquoted` of the assessment.
    tally: [Option<i64>; 5],
}

/// A scenario: the decision, the map and what the extractor named.
struct Case {
    text: &'static Text,
    /// Components of the map: name and aliases.
    map: Vec<(&'static str, Vec<String>)>,
    components: Vec<ProposedComponent>,
    edits: Option<CandidateEdits>,
}

/// Extracts the ADR of a project whose map has two components, adopts the
/// candidate as the rules would, and runs the link job it may have queued.
fn run(tag: &str, components: Vec<ProposedComponent>, edits: Option<CandidateEdits>) -> Outcome {
    run_case(
        tag,
        Case {
            text: &QUEUE,
            map: vec![("outbox", Vec::new()), ("billing", Vec::new())],
            components,
            edits,
        },
    )
}

fn run_case(tag: &str, case: Case) -> Outcome {
    let test = support::open(tag, &[]);
    let repo = test.root.join("repo");
    std::fs::create_dir_all(repo.join("docs/adr")).expect("docs");
    std::fs::write(
        repo.join("docs/adr/0001-fila.md"),
        format!("# Fila\n\n## Decisão\n{}\n", case.text.choice),
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
    let names: Vec<(String, String)> = case
        .map
        .iter()
        .map(|(name, aliases)| {
            let id = graph
                .create_entity(NewEntity {
                    project_id: "docs-p".into(),
                    kind: Some(EntityKind::Component),
                    name: (*name).into(),
                    patterns: vec![format!("crates/{name}/**")],
                    aliases: aliases.clone(),
                    ..NewEntity::default()
                })
                .expect("component")
                .entity_id;
            (id, (*name).to_string())
        })
        .collect();

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
        &Scripted(case.text, case.components),
        &capture,
        &RunContext::for_tests(),
    )
    .expect("extract");
    assert_eq!(report.inserted, 1);
    let candidate: String = connection
        .query_row("SELECT id FROM decision_candidates", [], |row| row.get(0))
        .expect("candidate");

    let adopted = Adoption::new(test.store.clone())
        .adopt_as(&candidate, case.edits, &[], &[], EdgeActor::Rules)
        .expect("adopt");
    let edges = test.store.project_edges("docs-p").expect("edges");
    let extracted = edges
        .iter()
        .filter(|edge| {
            edge.source_id == adopted.id
                && edge.is_pending()
                && ai_link_why(&edge.reason) == Some(EXTRACTED_LINK_WHY)
        })
        .map(|edge| {
            names
                .iter()
                .find(|(id, _)| *id == edge.entity_id)
                .map(|(_, name)| name.clone())
                .expect("a component of the map")
        })
        .collect::<Vec<_>>();

    let model = Model::default();
    let finder = LinkFinder::new(test.store.clone(), settings(), Factory(model.clone()));
    for result in finder.run_many(&[adopted.id.as_str()]) {
        result.expect("link job");
    }
    let link_calls = *model.0.lock().expect("calls");
    let tally = connection
        .query_row(
            "SELECT components_listed, components_proposed, components_kept, \
             components_unknown, components_unquoted FROM assessments",
            [],
            |row| {
                Ok([
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ])
            },
        )
        .expect("assessment");
    Outcome {
        extracted,
        link_calls,
        tally,
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
    assert_eq!(
        with.extracted,
        ["outbox"],
        "one pending link of the AI's kind"
    );
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

const UPDATE_QUOTE: &str = "discover releases and select its update channel";

/// The real project's map: the platform part holds the updater and has an
/// alias, and the CI sits beside it.
fn updater_case(name: &str) -> Case {
    Case {
        text: &UPDATER,
        map: vec![
            ("sc-platform", vec!["desktop-shell".to_string()]),
            ("CI", Vec::new()),
        ],
        components: vec![ProposedComponent {
            name: name.into(),
            quote: UPDATE_QUOTE.into(),
        }],
        edits: None,
    }
}

#[test]
fn a_decision_that_never_names_the_component_still_links_to_it_by_what_it_does() {
    let outcome = run_case("extracted-updater", updater_case("sc-platform"));
    assert_eq!(outcome.extracted, ["sc-platform"]);
    assert_eq!(
        outcome.link_calls, 0,
        "the proposer has nothing left to ask"
    );
    // The map listed two components; one name proposed and kept.
    assert_eq!(
        outcome.tally,
        [Some(2), Some(1), Some(1), Some(0), Some(0)],
        "listed, proposed, kept, unknown, unquoted"
    );
    println!(
        "gate extracted links: kept={} proposed={}",
        outcome.tally[2].unwrap_or_default(),
        outcome.tally[1].unwrap_or_default()
    );
}

#[test]
fn spelling_variations_of_a_name_resolve_and_a_suffixed_word_does_not() {
    // Case, hyphen, underscore, space and backticks never mattered (the key
    // ignores them); an alias, a path suffix and a description copied after
    // the name resolve too.
    for (index, name) in [
        "SC-Platform",
        "sc_platform",
        "sc platform",
        "`sc-platform`",
        "sc-platform::update",
        "Desktop Shell",
        "sc-platform: OS integration: window chrome, tray icon and updates",
        "Desktop Shell: the desktop shell",
    ]
    .into_iter()
    .enumerate()
    {
        let outcome = run_case(&format!("extracted-name-{index}"), updater_case(name));
        assert_eq!(outcome.extracted, ["sc-platform"], "{name}");
        assert_eq!(outcome.tally[3], Some(0), "{name}");
    }
    let outcome = run_case("extracted-name-crate", updater_case("sc-platform crate"));
    assert!(outcome.extracted.is_empty());
    assert_eq!(outcome.link_calls, 1, "the proposer is asked instead");
    assert_eq!(
        outcome.tally,
        [Some(2), Some(1), Some(0), Some(1), Some(0)],
        "a name the map does not list counts as unknown"
    );
}

#[test]
fn a_quote_the_text_does_not_back_is_counted_apart_from_an_unknown_name() {
    let mut case = updater_case("sc-platform");
    case.components[0].quote = "a stretch the decision never wrote".into();
    let outcome = run_case("extracted-unquoted", case);
    assert!(outcome.extracted.is_empty());
    assert_eq!(outcome.tally, [Some(2), Some(1), Some(0), Some(0), Some(1)]);
}
