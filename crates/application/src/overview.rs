//! Project overview: a summary and the main flows of a project, written by
//! the configured AI provider from what the app already knows (decisions in
//! force, rules and the project map), never from the code.
//!
//! Every sentence and every step must cite a recorded decision or rule; the
//! uncited ones are dropped. The result is a reading, not authority: it is
//! stored with its date and regenerated only when the user asks.

use std::collections::BTreeMap;

use domain::entities::EntityKind;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::analysis::ExtractorFactory;
use crate::architecture::Architecture;
use crate::captures::CaptureRepository;
use crate::claims::ClaimStore;
use crate::clock::now_rfc3339;
use crate::decisions::{DecisionQuery, DecisionStatus, DecisionStore};
use crate::documents::{document_ref, DocumentError, DocumentStore, Documents, PROPOSE_PER_RUN};
use crate::extract::ExtractError;
use crate::graph::{GraphStore, KnowledgeGraph};
use crate::inbox::InboxStore;
use crate::injection::short_ref;
use crate::page::{PageKnowledge, MAX_PAGE_DECISIONS};
use crate::profile::{choose_extractor, AiSettings, ExtractorChoice, ProfileStore, SecretStore};
use crate::projects::ProjectRepository;
use crate::relations::RelationStore;

/// Most decisions sent to the model, newest first.
pub const MAX_DECISIONS: usize = 60;
/// Most rules sent to the model.
pub const MAX_RULES: usize = 40;
/// Most components and technologies sent to the model.
pub const MAX_ENTITIES: usize = 40;
/// Most flows kept from the answer.
pub const MAX_FLOWS: usize = 6;
/// Most steps kept per flow.
pub const MAX_STEPS: usize = 8;
/// Most summary paragraphs kept.
pub const MAX_PARAGRAPHS: usize = 4;
/// Longest text kept per field.
const MAX_TEXT_CHARS: usize = 900;
/// Longest `via` label kept, in characters.
const MAX_VIA_CHARS: usize = 40;
/// Documents sent at most.
pub const MAX_DOCUMENTS: usize = 40;
/// Characters of documentation sent at most.
pub const MAX_DOCUMENT_CHARS: usize = 16_000;

/// JSON value type of [`StructuredModel`] schemas, re-exported so adapters
/// need no JSON crate of their own.
pub use serde_json::Value as JsonValue;

/// A model that answers one prompt with JSON under a schema.
pub trait StructuredModel {
    /// Completes with live authorization. Adapters must recheck before every retry.
    fn complete_authorized(
        &self,
        system: &str,
        user: &str,
        schema_name: &str,
        schema: &serde_json::Value,
        authorization: &dyn crate::external::Authorization,
    ) -> Result<String, ExtractError> {
        authorization.check()?;
        self.complete(system, user, schema_name, schema)
    }
    /// The model's JSON text for `system` instructions and `user` content.
    fn complete(
        &self,
        system: &str,
        user: &str,
        schema_name: &str,
        schema: &serde_json::Value,
    ) -> Result<String, ExtractError>;
}

/// Why an overview could not be produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverviewError {
    /// The storage backend failed; diagnostic only.
    Storage(String),
    /// The project does not exist.
    ProjectNotFound,
    /// Nothing is recorded yet: no decision and no rule.
    NothingRecorded,
    /// No external provider is enabled (local heuristic or no consent).
    ProviderOff,
    /// The provider failed; the message is product language.
    Provider(String),
    /// The answer had nothing citable left.
    EmptyAnswer,
}

impl OverviewError {
    /// Short, stable code.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Storage(_) => "storage",
            Self::ProjectNotFound => "project_not_found",
            Self::NothingRecorded => "nothing_recorded",
            Self::ProviderOff => "provider_off",
            Self::Provider(_) => "provider",
            Self::EmptyAnswer => "empty_answer",
        }
    }
}

impl std::fmt::Display for OverviewError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Storage(message) => write!(formatter, "falha de armazenamento: {message}"),
            Self::ProjectNotFound => formatter.write_str("projeto não encontrado"),
            Self::NothingRecorded => {
                formatter.write_str("ainda não há decisões nem regras confirmadas para resumir")
            }
            Self::ProviderOff => formatter
                .write_str("ative um provedor de IA em Configurações › IA para gerar a visão"),
            Self::Provider(message) => formatter.write_str(message),
            Self::EmptyAnswer => {
                formatter.write_str("a IA não devolveu nada com fonte; tente de novo mais tarde")
            }
        }
    }
}

impl std::error::Error for OverviewError {}

fn storage(error: impl std::fmt::Display) -> OverviewError {
    OverviewError::Storage(error.to_string())
}

/// A recorded decision or rule a sentence rests on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Citation {
    /// `decision` or `claim`.
    pub kind: String,
    /// Decision or claim id.
    pub id: String,
    /// Short label shown in the text (`D:xxxxxxxx` or `R:xxxxxxxx`).
    pub label: String,
    /// The decision question or rule statement, for readers.
    #[serde(default)]
    pub title: String,
}

/// One paragraph of the summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OverviewParagraph {
    /// The text.
    pub text: String,
    /// What it rests on.
    pub citations: Vec<Citation>,
}

/// One step of a flow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OverviewStep {
    /// Short title.
    pub title: String,
    /// What happens there.
    pub text: String,
    /// Component of the map where it happens, when the map has one.
    pub entity_id: Option<String>,
    /// Its name, for display.
    pub entity_name: Option<String>,
    /// How the previous component reaches this one: the protocol or medium
    /// (HTTP/JSON, a queue, a function call), when the model names it.
    #[serde(default)]
    pub via: Option<String>,
    /// Decisions and rules that govern it.
    pub citations: Vec<Citation>,
}

/// One main flow of the project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OverviewFlow {
    /// Title, such as "Captura até candidato".
    pub title: String,
    /// One sentence on what the flow achieves.
    pub description: String,
    /// Ordered steps.
    pub steps: Vec<OverviewStep>,
}

/// A generated overview, as stored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectOverview {
    /// Project summarized.
    pub project_id: String,
    /// RFC 3339 generation time.
    pub generated_at: String,
    /// Decisions in force it was built from.
    pub decisions: usize,
    /// Rules it was built from.
    pub rules: usize,
    /// Documents it was built from.
    #[serde(default)]
    pub documents: usize,
    /// Summary paragraphs.
    pub summary: Vec<OverviewParagraph>,
    /// Main flows.
    pub flows: Vec<OverviewFlow>,
    /// The containers and interactions the flows draw (C4 level 2), made when
    /// the overview was generated.
    #[serde(default)]
    pub architecture: Architecture,
}

impl ProjectOverview {
    /// The stored JSON form.
    pub fn to_json(&self) -> Result<String, OverviewError> {
        serde_json::to_string(self)
            .map_err(|_| OverviewError::Storage("visão impossível de gravar".into()))
    }

    /// Reads the stored JSON form.
    pub fn from_json(content: &str) -> Result<Self, OverviewError> {
        serde_json::from_str(content)
            .map_err(|_| OverviewError::Storage("visão gravada ilegível".into()))
    }
}

/// An overview with how much changed since it was generated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverviewView {
    /// The stored overview.
    pub overview: ProjectOverview,
    /// Decisions confirmed after it was generated.
    pub new_decisions: usize,
    /// Documents sent for analysis by this generation (0 when only read).
    pub queued_documents: usize,
}

/// Persistence port for overviews.
pub trait OverviewStore {
    /// The stored overview of a project, if any.
    fn overview(&self, project_id: &str) -> Result<Option<ProjectOverview>, OverviewError>;

    /// Replaces the stored overview of a project.
    fn save_overview(&self, overview: &ProjectOverview) -> Result<(), OverviewError>;
}

/// Object-safe entry point for the desktop.
pub trait OverviewApi: Send + Sync {
    /// See [`ProjectOverviews::current`].
    fn current(&self, project_id: &str) -> Result<Option<OverviewView>, OverviewError>;
    /// See [`ProjectOverviews::generate`].
    fn generate(&self, project_id: &str) -> Result<OverviewView, OverviewError>;
    /// See [`ProjectOverviews::page`].
    fn page(&self, project_id: &str, project_name: &str) -> Result<String, OverviewError>;
}

/// Instructions for the overview call.
pub const OVERVIEW_PROMPT: &str = concat!(
    "Preserve explicit scope and qualifiers in every summary. Empty scope means not informed, not the entire project. Empty qualifiers mean not informed. Artifact-less qualifiers are reviewer declarations, not verified support. Never invent authority or turn unexecuted validation into a successful check. You write the overview of one software project for its own team, using only the recorded knowledge given: decisions in force (D:...), rules (R:...) and \
the project map (components and technologies) and the project's own documentation (F:..., title, sections and opening paragraph; documents describe intent and may be outdated: when they disagree with a decision, the decision wins). You never see the code; do not invent \
components, libraries or behaviour that the records do not state.\n\
Write in the language of the records. summary: 2 to 4 short paragraphs: what the project is \
for, how it is organised, the central choices and the rules that weigh most. flows: the 3 to \
6 main flows of the system (how a request, a piece of data or a user action travels \
through it), each with 3 to 8 ordered steps; a step names the component where it happens \
(use the component name exactly as listed, or null) and what happens there; via is how the \
previous step's component reaches this one (protocol or medium, up to four words, e.g. \
HTTP/JSON, fila SQLite, chamada direta), or null when unknown.\n\
Every paragraph and every step must cite the records it rests on in refs, using the ids \
exactly as given (D:xxxxxxxx, R:xxxxxxxx or F:xxxxxxxx). Anything you cannot cite, leave out. Prefer \
fewer, well-cited flows over many vague ones.\n\
Reply with one JSON object only, matching exactly: {\"summary\":[{\"text\":string,\
\"refs\":[string]}],\"flows\":[{\"title\":string,\"description\":string,\"steps\":[{\
\"title\":string,\"text\":string,\"component\":string|null,\"via\":string|null,\"refs\":[string]}]}]}.",
    crate::plain_rules!()
);

/// Strict JSON Schema of the overview answer.
pub fn overview_schema() -> serde_json::Value {
    let refs = json!({ "type": "array", "items": { "type": "string" } });
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["summary", "flows"],
        "properties": {
            "summary": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["text", "refs"],
                    "properties": { "text": { "type": "string" }, "refs": refs }
                }
            },
            "flows": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["title", "description", "steps"],
                    "properties": {
                        "title": { "type": "string" },
                        "description": { "type": "string" },
                        "steps": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "additionalProperties": false,
                                "required": ["title", "text", "component", "via", "refs"],
                                "properties": {
                                    "title": { "type": "string" },
                                    "text": { "type": "string" },
                                    "component": { "type": ["string", "null"] },
                                    "via": { "type": ["string", "null"] },
                                    "refs": refs
                                }
                            }
                        }
                    }
                }
            }
        }
    })
}

/// What the model receives, and how its references resolve back.
#[derive(Debug, Clone, Default)]
pub struct OverviewInput {
    /// The prompt's user content.
    pub text: String,
    /// `D:xxxxxxxx` / `R:xxxxxxxx` → citation.
    pub references: BTreeMap<String, Citation>,
    /// Lowercased component name → (entity id, display name).
    pub components: BTreeMap<String, (String, String)>,
    /// Decisions included.
    pub decisions: usize,
    /// Rules included.
    pub rules: usize,
    /// Documents included.
    pub documents: usize,
}

#[derive(Deserialize)]
struct RawOverview {
    #[serde(default)]
    summary: Vec<RawParagraph>,
    #[serde(default)]
    flows: Vec<RawFlow>,
}

#[derive(Deserialize)]
struct RawParagraph {
    text: String,
    #[serde(default)]
    refs: Vec<String>,
}

#[derive(Deserialize)]
struct RawFlow {
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    steps: Vec<RawStep>,
}

#[derive(Deserialize)]
struct RawStep {
    title: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    component: Option<String>,
    #[serde(default)]
    via: Option<String>,
    #[serde(default)]
    refs: Vec<String>,
}

/// Parses the model's answer, keeping only what cites known records:
/// uncited paragraphs and steps are dropped, and a flow needs two cited steps.
pub fn parse_overview(
    text: &str,
    input: &OverviewInput,
) -> Result<(Vec<OverviewParagraph>, Vec<OverviewFlow>), OverviewError> {
    let start = text.find('{').unwrap_or(0);
    let end = text.rfind('}').map_or(text.len(), |end| end + 1);
    let raw: RawOverview = serde_json::from_str(text.get(start..end).unwrap_or(text))
        .map_err(|_| OverviewError::Provider("a IA respondeu num formato inesperado".into()))?;
    let cite = |refs: &[String]| -> Vec<Citation> {
        let mut citations: Vec<Citation> = Vec::new();
        for reference in refs {
            let key = reference.trim().to_lowercase();
            if let Some(citation) = input.references.get(&key) {
                if !citations.contains(citation) {
                    citations.push(citation.clone());
                }
            }
        }
        citations
    };
    let bounded = |text: &str| -> String { text.trim().chars().take(MAX_TEXT_CHARS).collect() };
    let summary: Vec<OverviewParagraph> = raw
        .summary
        .iter()
        .filter_map(|paragraph| {
            let citations = cite(&paragraph.refs);
            (!citations.is_empty() && !paragraph.text.trim().is_empty()).then(|| {
                OverviewParagraph {
                    text: bounded(&paragraph.text),
                    citations,
                }
            })
        })
        .take(MAX_PARAGRAPHS)
        .collect();
    let flows: Vec<OverviewFlow> = raw
        .flows
        .iter()
        .filter_map(|flow| {
            let steps: Vec<OverviewStep> = flow
                .steps
                .iter()
                .filter_map(|step| {
                    let citations = cite(&step.refs);
                    if citations.is_empty() || step.title.trim().is_empty() {
                        return None;
                    }
                    let entity = step
                        .component
                        .as_deref()
                        .and_then(|name| input.components.get(&name.trim().to_lowercase()));
                    Some(OverviewStep {
                        title: bounded(&step.title),
                        text: bounded(&step.text),
                        entity_id: entity.map(|(id, _)| id.clone()),
                        entity_name: entity.map(|(_, name)| name.clone()),
                        via: step
                            .via
                            .as_deref()
                            .map(|via| via.trim().chars().take(MAX_VIA_CHARS).collect::<String>())
                            .filter(|via| !via.is_empty()),
                        citations,
                    })
                })
                .take(MAX_STEPS)
                .collect();
            (steps.len() >= 2 && !flow.title.trim().is_empty()).then(|| OverviewFlow {
                title: bounded(&flow.title),
                description: bounded(&flow.description),
                steps,
            })
        })
        .take(MAX_FLOWS)
        .collect();
    if summary.is_empty() && flows.is_empty() {
        return Err(OverviewError::EmptyAnswer);
    }
    Ok((summary, flows))
}

/// The overview use case over the store, the AI settings and the provider
/// factory.
#[derive(Debug, Clone)]
pub struct ProjectOverviews<S, P, K, F> {
    store: S,
    settings: AiSettings<P, K>,
    factory: F,
    documents: Option<Documents<S>>,
}

impl<S, P, K, F> ProjectOverviews<S, P, K, F>
where
    S: GraphStore
        + RelationStore
        + ClaimStore
        + ProjectRepository
        + DecisionStore
        + OverviewStore
        + DocumentStore
        + CaptureRepository
        + Clone,
    P: ProfileStore,
    K: SecretStore,
    F: ExtractorFactory,
    F::Extractor: StructuredModel,
{
    /// Wraps the store, the AI settings and the provider factory.
    pub fn new(store: S, settings: AiSettings<P, K>, factory: F) -> Self {
        Self {
            store,
            settings,
            factory,
            documents: None,
        }
    }

    /// Uses `documents` (for instance one that prepares the map) when the
    /// overview re-reads and queues the project's documentation.
    #[must_use]
    pub fn with_documents(mut self, documents: Documents<S>) -> Self {
        self.documents = Some(documents);
        self
    }

    fn documents(&self) -> Documents<S> {
        self.documents
            .clone()
            .unwrap_or_else(|| Documents::new(self.store.clone()))
    }

    /// The stored overview and how many decisions were confirmed since.
    ///
    /// # Errors
    ///
    /// `storage` on failure.
    pub fn current(&self, project_id: &str) -> Result<Option<OverviewView>, OverviewError> {
        let Some(mut overview) = self.store.overview(project_id)? else {
            return Ok(None);
        };
        // An overview stored before the architecture existed gets it from
        // the current map, without being rewritten.
        if overview.architecture.is_empty() && !overview.flows.is_empty() {
            overview.architecture = self.architecture(project_id, &overview.flows);
        }
        let new_decisions = self
            .store
            .project_decisions(project_id)
            .map_err(storage)?
            .iter()
            .filter(|decision| decision.confirmed_at > overview.generated_at)
            .count();
        Ok(Some(OverviewView {
            overview,
            new_decisions,
            queued_documents: 0,
        }))
    }

    /// Asks the provider for a new overview and stores it.
    ///
    /// # Errors
    ///
    /// `project_not_found`, `nothing_recorded`, `provider_off`, `provider`,
    /// `empty_answer` or `storage`.
    pub fn generate(&self, project_id: &str) -> Result<OverviewView, OverviewError> {
        let project = ProjectRepository::get(&self.store, project_id)
            .map_err(storage)?
            .ok_or(OverviewError::ProjectNotFound)?;
        // Documentation is read again so the overview sees the current files;
        // an unreadable folder only leaves the previous index in place.
        if let Err(DocumentError::Storage(detail)) = self.documents().index(project_id) {
            return Err(OverviewError::Storage(detail));
        }
        let input = self.input(project_id, &project.location)?;
        if input.decisions == 0 && input.rules == 0 && input.documents == 0 {
            return Err(OverviewError::NothingRecorded);
        }
        let profile = self.settings.load_or_seed().map_err(storage)?;
        if choose_extractor(Some(&profile)) != ExtractorChoice::ExternalEnabled {
            return Err(OverviewError::ProviderOff);
        }
        let secret = match self.settings.secret(&profile.credential_account()) {
            Ok(Some(secret)) => secret,
            Ok(None) if !profile.credential_required() => String::new(),
            Ok(None) => return Err(OverviewError::ProviderOff),
            Err(error) => return Err(storage(error)),
        };
        let model = self
            .factory
            .authorized(&profile, secret, &self.settings)
            .map_err(|_| OverviewError::ProviderOff)?;
        let answer = model
            .complete(
                OVERVIEW_PROMPT,
                &input.text,
                "project_overview",
                &overview_schema(),
            )
            .map_err(|error| OverviewError::Provider(provider_message(&error)))?;
        let (summary, flows) = parse_overview(&answer, &input)?;
        let overview = ProjectOverview {
            project_id: project_id.to_string(),
            generated_at: now_rfc3339(),
            decisions: input.decisions,
            rules: input.rules,
            documents: input.documents,
            architecture: self.architecture(project_id, &flows),
            summary,
            flows,
        };
        self.store.save_overview(&overview)?;
        // With the provider on, new or changed documents go to the same
        // analysis as conversations: their decisions reach Revisão.
        let queued_documents = self
            .documents()
            .propose(project_id, PROPOSE_PER_RUN)
            .map_err(|error| match error {
                DocumentError::Storage(detail) => OverviewError::Storage(detail),
                other => OverviewError::Storage(other.to_string()),
            })?;
        Ok(OverviewView {
            overview,
            new_decisions: 0,
            queued_documents,
        })
    }

    /// The containers and interactions of the project's flows; empty (never
    /// an error) when the map cannot be read, so the rest of the page stays.
    fn architecture(&self, project_id: &str, flows: &[OverviewFlow]) -> Architecture {
        let graph = KnowledgeGraph::new(self.store.clone());
        match (
            graph.project_map(project_id, None),
            graph.project_graph(project_id, None),
        ) {
            (Ok(map), Ok(full)) => crate::architecture::derive(&map, &full, flows),
            _ => Architecture::default(),
        }
    }

    /// Builds the prompt content from the records of the project.
    pub fn input(&self, project_id: &str, location: &str) -> Result<OverviewInput, OverviewError> {
        let graph = KnowledgeGraph::new(self.store.clone());
        let map = graph.project_map(project_id, None).map_err(storage)?;
        let mut input = OverviewInput::default();
        for decision in self.store.project_decisions(project_id).map_err(storage)? {
            if let Some(stored) =
                DecisionStore::get(&self.store, &decision.decision_id).map_err(storage)?
            {
                crate::qualifiers::decode(&stored.qualifiers).map_err(OverviewError::Storage)?;
            }
        }
        let project_name = location.rsplit(['/', '\\']).next().unwrap_or(location);
        let mut text = format!("# Project {project_name}\n");

        // Decisions in force, newest first.
        let relations = self.store.project_relations(project_id).map_err(storage)?;
        let mut decisions = self.store.project_decisions(project_id).map_err(storage)?;
        decisions.retain(|decision| {
            !relations.iter().any(|row| {
                row.kind == domain::relations::RelationKind::Supersedes.as_str()
                    && row.to == decision.decision_id
            })
        });
        decisions.sort_by(|left, right| right.confirmed_at.cmp(&left.confirmed_at));
        decisions.truncate(MAX_DECISIONS);
        text.push_str("\n## Decisions in force\n");
        for decision in &decisions {
            let label = format!("D:{}", short_ref(&decision.decision_id));
            let rationale = DecisionStore::get(&self.store, &decision.decision_id)
                .map_err(storage)?
                .map(|stored| crate::external::limited_text(&stored.rationale, 400))
                .unwrap_or_default();
            let restrictions = DecisionStore::get(&self.store, &decision.decision_id)
                .map_err(storage)?
                .map(|stored| {
                    format!(
                        " scope:{} qualifiers:{}",
                        crate::external::protected_text(&stored.scope),
                        crate::external::protected_text(&stored.qualifiers)
                    )
                })
                .unwrap_or_default();
            text.push_str(&format!(
                "- {label} {} → {} — {}\n",
                crate::external::protected_text(&decision.question),
                crate::external::protected_text(&decision.choice),
                rationale
            ));
            input.references.insert(
                label.to_lowercase(),
                Citation {
                    kind: "decision".into(),
                    id: decision.decision_id.clone(),
                    label,
                    title: decision.question.clone(),
                },
            );
            text.push_str(&restrictions);
        }
        input.decisions = decisions.len();

        // Rules valid now.
        let now = domain::time::Timestamp::parse(&now_rfc3339());
        let claims: Vec<_> = self
            .store
            .project_claims(project_id)
            .map_err(storage)?
            .into_iter()
            .filter(|claim| now.as_ref().is_some_and(|at| claim.is_valid_at(at)))
            .take(MAX_RULES)
            .collect();
        text.push_str("\n## Rules\n");
        for claim in &claims {
            crate::qualifiers::decode(&claim.qualifiers).map_err(OverviewError::Storage)?;
            let label = format!("R:{}", short_ref(&claim.claim_id));
            text.push_str(&format!(
                "- {label} ({}) {}\n",
                claim.kind.as_str(),
                crate::external::protected_text(&claim.statement)
            ));
            text.push_str(&format!(
                " qualifiers:{}\n",
                crate::external::protected_text(&claim.qualifiers)
            ));
            text.push_str(&format!(
                " inherited_scope:{}\n",
                crate::external::protected_text(&claim.inherited_scope)
            ));
            input.references.insert(
                label.to_lowercase(),
                Citation {
                    kind: "claim".into(),
                    id: claim.claim_id.clone(),
                    label,
                    title: claim.statement.clone(),
                },
            );
        }
        input.rules = claims.len();

        // The map.
        let parents: BTreeMap<&String, &String> = map
            .part_of
            .iter()
            .map(|(child, parent)| (child, parent))
            .collect();
        let names: BTreeMap<&String, &String> = map
            .entities
            .iter()
            .map(|row| (&row.entity.entity_id, &row.entity.name))
            .collect();
        text.push_str("\n## Project map\n");
        for row in map.entities.iter().take(MAX_ENTITIES) {
            let entity = &row.entity;
            let kind = match entity.kind {
                EntityKind::Component => "component",
                EntityKind::Technology => "technology",
            };
            let detail = graph
                .entity_detail(&entity.entity_id, None)
                .map_err(storage)?;
            let refs: Vec<String> = detail
                .decisions
                .iter()
                .map(|node| format!("D:{}", short_ref(&node.node.id)))
                .chain(
                    detail
                        .claims
                        .iter()
                        .map(|node| format!("R:{}", short_ref(&node.node.id))),
                )
                .collect();
            let parent = parents
                .get(&entity.entity_id)
                .and_then(|parent| names.get(parent))
                .map(|name| format!(", part of {}", crate::external::protected_text(name)))
                .unwrap_or_default();
            text.push_str(&format!(
                "- {kind} \"{}\"{parent}{}{}: {}\n",
                crate::external::protected_text(&entity.name),
                if entity.patterns.is_empty() {
                    String::new()
                } else {
                    format!(
                        " [{}]",
                        entity
                            .patterns
                            .iter()
                            .map(|pattern| crate::external::protected_text(pattern))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                },
                if entity.description.is_empty() {
                    String::new()
                } else {
                    format!(
                        " — {}",
                        crate::external::protected_text(&entity.description)
                    )
                },
                if refs.is_empty() {
                    "no records yet".to_string()
                } else {
                    refs.join(", ")
                }
            ));
            if entity.kind == EntityKind::Component {
                input.components.insert(
                    entity.name.to_lowercase(),
                    (entity.entity_id.clone(), entity.name.clone()),
                );
            }
        }
        // Documentation: title, sections and opening paragraph; ADRs and
        // specs first, then READMEs and guides, within a budget.
        let mut documents = self.store.project_documents(project_id).map_err(|error| {
            OverviewError::Storage(match error {
                DocumentError::Storage(detail) => detail,
                other => other.to_string(),
            })
        })?;
        documents.sort_by(|left, right| (left.kind, &left.path).cmp(&(right.kind, &right.path)));
        let mut budget = MAX_DOCUMENT_CHARS;
        let mut included = 0;
        if !documents.is_empty() {
            text.push_str("\n## Project documentation\n");
        }
        for document in documents.iter().take(MAX_DOCUMENTS) {
            let label = format!("F:{}", document_ref(&document.path));
            let mut entry = format!(
                "- {label} ({}) {} — {}",
                document.kind.as_str(),
                crate::external::protected_text(&document.path),
                crate::external::protected_text(&document.title)
            );
            if !document.headings.is_empty() {
                entry.push_str(&format!(
                    " | sections: {}",
                    document
                        .headings
                        .iter()
                        .map(|heading| crate::external::protected_text(heading))
                        .collect::<Vec<_>>()
                        .join("; ")
                ));
            }
            if !document.excerpt.is_empty() {
                entry.push_str(&format!(
                    " | {}",
                    crate::external::protected_text(&document.excerpt)
                ));
            }
            entry.push('\n');
            let entry = crate::external::protected_text(&entry);
            if entry.chars().count() > budget {
                break;
            }
            budget -= entry.chars().count();
            text.push_str(&entry);
            input.references.insert(
                label.to_lowercase(),
                Citation {
                    kind: "document".into(),
                    id: document.path.clone(),
                    label,
                    title: document.title.clone(),
                },
            );
            included += 1;
        }
        input.documents = included;
        input.text = crate::external::protected_text(&text);
        Ok(input)
    }
}

impl<S, P, K, F> ProjectOverviews<S, P, K, F>
where
    S: GraphStore
        + RelationStore
        + ClaimStore
        + ProjectRepository
        + DecisionStore
        + OverviewStore
        + DocumentStore
        + CaptureRepository
        + InboxStore
        + Clone,
    P: ProfileStore,
    K: SecretStore,
    F: ExtractorFactory,
    F::Extractor: StructuredModel,
{
    /// The architecture and flows page (`crate::page`) of the stored
    /// overview, with the decisions in force and the rules valid now that
    /// explain it. Reads only; never calls the provider.
    ///
    /// # Errors
    ///
    /// `nothing_recorded` when no overview is stored yet, `storage` on failure.
    pub fn page(&self, project_id: &str, project_name: &str) -> Result<String, OverviewError> {
        let view = self
            .current(project_id)?
            .ok_or(OverviewError::NothingRecorded)?;
        let knowledge = self.knowledge(project_id, &view.overview.architecture)?;
        Ok(crate::page::render(
            project_name,
            &view.overview,
            &knowledge,
        ))
    }

    /// The decisions and rules of the page, tied to the containers of
    /// `architecture`.
    ///
    /// # Errors
    ///
    /// `storage` on failure.
    pub fn knowledge(
        &self,
        project_id: &str,
        architecture: &Architecture,
    ) -> Result<PageKnowledge, OverviewError> {
        // Superseded decisions come along only to name the other end of a
        // relation; the page shows those in force.
        let decisions = DecisionStore::list(
            &self.store,
            &DecisionQuery {
                project_id: Some(project_id.to_owned()),
                statuses: vec![DecisionStatus::Accepted, DecisionStatus::Superseded],
                limit: MAX_PAGE_DECISIONS * 4,
                before: None,
            },
        )
        .map_err(storage)?;
        let relations = self.store.project_relations(project_id).map_err(storage)?;
        let graph = KnowledgeGraph::new(self.store.clone());
        let map = graph.project_map(project_id, None).map_err(storage)?;
        let full = graph.project_graph(project_id, None).map_err(storage)?;
        let now = domain::time::Timestamp::parse(&now_rfc3339());
        let claims: Vec<_> = self
            .store
            .project_claims(project_id)
            .map_err(storage)?
            .into_iter()
            .filter(|claim| now.as_ref().is_some_and(|at| claim.is_valid_at(at)))
            .collect();
        // The significance criteria live on the candidate each decision came
        // from; an unreadable candidate only leaves the decision without them.
        let mut criteria: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for decision in decisions
            .iter()
            .filter(|decision| decision.status == DecisionStatus::Accepted)
            .take(MAX_PAGE_DECISIONS * 2)
        {
            if let Ok(Some(candidate)) = InboxStore::get(&self.store, &decision.candidate_id) {
                let ticked: Vec<String> =
                    serde_json::from_str(&candidate.criteria).unwrap_or_default();
                if !ticked.is_empty() {
                    criteria.insert(decision.decision_id.clone(), ticked);
                }
            }
        }
        Ok(crate::page::assemble(
            &decisions,
            &criteria,
            &claims,
            &map,
            &full,
            &relations,
            architecture,
        ))
    }
}

/// Product copy for a provider failure.
fn provider_message(error: &ExtractError) -> String {
    match error {
        ExtractError::Extractor(message) => {
            let mut text = message.clone();
            if let Some(first) = text.get_mut(0..1) {
                first.make_ascii_uppercase();
            }
            format!("{text}.")
        }
        _ => "O provedor de IA não respondeu. Tente de novo.".to_string(),
    }
}

impl<S, P, K, F> OverviewApi for ProjectOverviews<S, P, K, F>
where
    S: GraphStore
        + RelationStore
        + ClaimStore
        + ProjectRepository
        + DecisionStore
        + OverviewStore
        + DocumentStore
        + CaptureRepository
        + InboxStore
        + Clone
        + Send
        + Sync,
    P: ProfileStore + Send + Sync,
    K: SecretStore + Send + Sync,
    F: ExtractorFactory + Send + Sync,
    F::Extractor: StructuredModel,
{
    fn current(&self, project_id: &str) -> Result<Option<OverviewView>, OverviewError> {
        ProjectOverviews::current(self, project_id)
    }

    fn generate(&self, project_id: &str) -> Result<OverviewView, OverviewError> {
        ProjectOverviews::generate(self, project_id)
    }

    fn page(&self, project_id: &str, project_name: &str) -> Result<String, OverviewError> {
        ProjectOverviews::page(self, project_id, project_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> OverviewInput {
        let mut input = OverviewInput::default();
        input.references.insert(
            "d:aaaa1111".into(),
            Citation {
                kind: "decision".into(),
                id: "dec-1".into(),
                label: "D:aaaa1111".into(),
                title: "Qual banco usar?".into(),
            },
        );
        input.references.insert(
            "r:bbbb2222".into(),
            Citation {
                kind: "claim".into(),
                id: "claim-1".into(),
                label: "R:bbbb2222".into(),
                title: "Erros em português".into(),
            },
        );
        input
            .components
            .insert("storage".into(), ("ent-1".into(), "storage".into()));
        input
    }

    #[test]
    fn keeps_only_what_cites_known_records() {
        let answer = r#"```json
        {"summary":[{"text":"App local.","refs":["D:aaaa1111"]},{"text":"Inventado.","refs":["D:zzzz9999"]}],
         "flows":[
           {"title":"Gravar","description":"x","steps":[
              {"title":"Recebe","text":"a","component":"Storage","refs":["d:aaaa1111"]},
              {"title":"Valida","text":"b","component":null,"via":"  HTTP/JSON ","refs":["R:bbbb2222"]},
              {"title":"Sem fonte","text":"c","component":null,"refs":[]}]},
           {"title":"Vago","description":"y","steps":[
              {"title":"Um","text":"a","component":null,"refs":["D:aaaa1111"]}]}]}
        ```"#;
        let (summary, flows) = parse_overview(answer, &input()).expect("parse");
        assert_eq!(summary.len(), 1, "the uncited paragraph is dropped");
        assert_eq!(flows.len(), 1, "a flow needs two cited steps");
        assert_eq!(flows[0].steps.len(), 2);
        assert_eq!(flows[0].steps[0].entity_id.as_deref(), Some("ent-1"));
        assert_eq!(flows[0].steps[1].citations[0].kind, "claim");
        assert_eq!(flows[0].steps[0].via, None, "a step may name no medium");
        assert_eq!(flows[0].steps[1].via.as_deref(), Some("HTTP/JSON"));
    }

    #[test]
    fn nothing_citable_is_an_error() {
        let answer = r#"{"summary":[{"text":"x","refs":[]}],"flows":[]}"#;
        assert_eq!(
            parse_overview(answer, &input()).map(|_| ()),
            Err(OverviewError::EmptyAnswer)
        );
    }
}
