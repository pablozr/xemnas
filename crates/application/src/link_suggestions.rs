//! Links from an adopted decision to the map's components, proposed by the AI.
//!
//! A decision taken from an ADR, a spec or a conversation touches no file, so
//! nothing ties it to the component it governs. When the decision gets no
//! link from files or dependencies, a job asks the model which components of
//! the map it applies to. The model must copy, verbatim, the words of the
//! decision that tie it to each component; the app checks the ids and the
//! quote before storing a pending `affects` suggestion, which the review
//! (manual or automatic) confirms or discards. Similarity of vocabulary never
//! creates a link, and a rejected or existing edge is never suggested again.

use std::collections::BTreeSet;

use domain::entities::{EdgeKind, EdgeOrigin, EntityKind, NodeKind};
use serde::Deserialize;
use serde_json::json;

use crate::analysis::ExtractorFactory;
use crate::clock::now_rfc3339;
use crate::decisions::{DecisionStatus, DecisionStore, StoredDecision};
use crate::graph::{
    ai_link_quote, ai_link_reason, mention_quote, EdgeRecord, EntityRecord, GraphStore,
};
use crate::jobs::{JobRecord, JobRepository, JobState};
use crate::overview::StructuredModel;
use crate::profile::{choose_extractor, AiSettings, ExtractorChoice, ProfileStore, SecretStore};

/// Job kind that proposes the components an adopted decision applies to.
pub const LINK_JOB_KIND: &str = crate::jobs::JobKind::SuggestLinks.as_str();
/// Components sent to the model, at most.
pub const MAX_COMPONENTS: usize = 60;
/// Links kept per decision, at most.
pub const MAX_LINKS: usize = 3;
/// Shortest quote accepted as evidence, in characters.
const MIN_QUOTE_CHARS: usize = 8;
/// Characters of a rationale or a scope item sent.
const MAX_TEXT_CHARS: usize = 600;
/// Characters of a component description sent.
const MAX_DESCRIPTION_CHARS: usize = 200;
/// Patterns and aliases listed per component.
const MAX_LISTED: usize = 6;

/// System prompt of the link proposer.
pub const LINK_PROMPT: &str = concat!(
    "You connect one engineering decision of a software project to the parts of the project \
map it governs. The map lists components; each has an id, a name, aliases, path patterns and \
a description. Link the decision to a component only when the decision governs or constrains \
that component's behavior or code: a rule, contract, policy, format or limit that the \
component must follow or implements. Never link by shared vocabulary alone: a decision that \
only mentions a word, a tool or a topic that a component also uses is not about that \
component. Decisions about the project rather than the code are not links even when they \
name a component: who owns or approves changes, how things are named, published, packaged, \
hosted or announced, and which words to use. Ask: would the component's code or behavior be \
different if this decision were different? If not, do not link. Prefer no link over a weak \
one; an empty list is a good answer when you are not sure. Give at most 3 links.\n\
For each link give component_id exactly as listed (c1, c2, ...); quote: the words of the \
decision, copied verbatim from its question, choice, why or scope, that tie it to the \
component; and reason: one short sentence, in the language of the decision.\n\
Reply with one JSON object only, matching exactly: {\"links\":[{\"component_id\":string,\
\"quote\":string,\"reason\":string}]}.",
    crate::plain_rules!()
);

/// Strict schema of the answer.
pub fn link_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["links"],
        "properties": {
            "links": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["component_id", "quote", "reason"],
                    "properties": {
                        "component_id": { "type": "string" },
                        "quote": { "type": "string" },
                        "reason": { "type": "string" }
                    }
                }
            }
        }
    })
}

/// The decision as the model sees it: secrets redacted, long texts cut. The
/// quote is checked against exactly this text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkSubject {
    /// The question.
    pub question: String,
    /// The choice.
    pub choice: String,
    /// The rationale.
    pub rationale: String,
    /// Scope items.
    pub scope: Vec<String>,
}

impl LinkSubject {
    /// The text of a stored decision as it will be sent.
    pub fn of(decision: &StoredDecision) -> Self {
        let scope: Vec<String> = serde_json::from_str(&decision.scope).unwrap_or_default();
        Self {
            question: crate::external::protected_text(&decision.question),
            choice: crate::external::protected_text(&decision.choice),
            rationale: crate::external::limited_text(&decision.rationale, MAX_TEXT_CHARS),
            scope: scope
                .iter()
                .map(|item| crate::external::limited_text(item, MAX_TEXT_CHARS))
                .collect(),
        }
    }

    fn texts(&self) -> impl Iterator<Item = &str> {
        [&self.question, &self.choice, &self.rationale]
            .into_iter()
            .chain(&self.scope)
            .map(String::as_str)
    }
}

/// A link that survived the checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiLink {
    /// The component.
    pub entity_id: String,
    /// Words of the decision that tie it to the component.
    pub quote: String,
    /// One sentence from the model.
    pub reason: String,
}

/// Live components sent to the model, by name, at most [`MAX_COMPONENTS`].
pub fn candidate_components(entities: &[EntityRecord]) -> Vec<&EntityRecord> {
    let mut components: Vec<&EntityRecord> = entities
        .iter()
        .filter(|entity| entity.kind == EntityKind::Component && entity.retired_at.is_none())
        .collect();
    components.sort_by(|left, right| left.key.cmp(&right.key));
    components.truncate(MAX_COMPONENTS);
    components
}

/// The user message: the decision, then the components numbered `c1`, `c2`...
pub fn link_request(subject: &LinkSubject, components: &[&EntityRecord]) -> String {
    let mut user = format!(
        "## Decision\nQuestion: {}\nChoice: {}\nWhy: {}\n",
        subject.question, subject.choice, subject.rationale
    );
    for item in &subject.scope {
        user.push_str(&format!("Scope: {item}\n"));
    }
    user.push_str("\n## Components\n");
    for (position, component) in components.iter().enumerate() {
        user.push_str(&format!("- c{} | {}", position + 1, component.name));
        let aliases: Vec<&str> = component
            .aliases
            .iter()
            .take(MAX_LISTED)
            .map(String::as_str)
            .collect();
        if !aliases.is_empty() {
            user.push_str(&format!(" | aliases: {}", aliases.join(", ")));
        }
        let patterns: Vec<&str> = component
            .patterns
            .iter()
            .take(MAX_LISTED)
            .map(String::as_str)
            .collect();
        if !patterns.is_empty() {
            user.push_str(&format!(" | paths: {}", patterns.join(", ")));
        }
        if !component.description.trim().is_empty() {
            let description =
                crate::external::limited_text(&component.description, MAX_DESCRIPTION_CHARS);
            user.push_str(&format!(
                " | {}",
                description.split_whitespace().collect::<Vec<_>>().join(" ")
            ));
        }
        user.push('\n');
    }
    user
}

#[derive(Deserialize)]
struct Answer {
    #[serde(default)]
    links: Vec<RawLink>,
}

#[derive(Deserialize)]
struct RawLink {
    #[serde(default)]
    component_id: String,
    #[serde(default)]
    quote: String,
    #[serde(default)]
    reason: String,
}

/// Words only, lowercase and without accents, for comparing a quote with the
/// text.
fn normalized(text: &str) -> String {
    text.split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(crate::terms::fold)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Links from the answer that name a sent component and quote the decision
/// verbatim (case and accents aside), once per component, at most
/// [`MAX_LINKS`]. Anything else is dropped; an unreadable answer gives none.
pub fn parse_links(
    answer: &str,
    subject: &LinkSubject,
    components: &[&EntityRecord],
) -> Vec<AiLink> {
    let Ok(answer) = serde_json::from_str::<Answer>(answer.trim()) else {
        return Vec::new();
    };
    let texts: Vec<String> = subject.texts().map(normalized).collect();
    let mut seen = BTreeSet::new();
    let mut links = Vec::new();
    for raw in answer.links {
        if links.len() >= MAX_LINKS {
            break;
        }
        let id = raw.component_id.trim().to_lowercase();
        let Some(component) = id
            .strip_prefix('c')
            .and_then(|position| position.parse::<usize>().ok())
            .and_then(|position| position.checked_sub(1))
            .and_then(|index| components.get(index))
        else {
            continue;
        };
        let quote = raw.quote.trim();
        let needle = normalized(quote);
        if needle.chars().count() < MIN_QUOTE_CHARS
            || !texts.iter().any(|text| text.contains(&needle))
        {
            continue;
        }
        if seen.insert(component.entity_id.clone()) {
            links.push(AiLink {
                entity_id: component.entity_id.clone(),
                quote: quote.to_string(),
                reason: raw.reason.trim().to_string(),
            });
        }
    }
    links
}

/// Whether the decision still needs the AI's help to reach the map: it has
/// no live tie from a file, a dependency or a person, and the AI was not
/// asked before. Mention and AI suggestions are weak and do not count as
/// ties.
pub fn needs_links(edges: &[EdgeRecord], decision_id: &str) -> bool {
    !edges
        .iter()
        .filter(|edge| edge.source_kind == NodeKind::Decision && edge.source_id == decision_id)
        .any(|edge| {
            let asked = ai_link_quote(&edge.reason).is_some();
            let tie = edge.is_live() && mention_quote(&edge.reason).is_none();
            asked || tie
        })
}

/// Decisions queued per map refresh, at most.
pub const MAX_QUEUED_PER_REFRESH: usize = 50;

fn insert_link_job<S: JobRepository>(store: &S, decision_id: &str) {
    let now = now_rfc3339();
    // A suggestion; nothing waits on it, so a failed insert is not an error.
    let _ = store.insert(&JobRecord {
        id: uuid::Uuid::now_v7().to_string(),
        kind: LINK_JOB_KIND.to_string(),
        payload: decision_id.to_string(),
        state: JobState::Queued,
        idempotent: true,
        attempts: 0,
        last_error: None,
        created_at: now.clone(),
        updated_at: now,
    });
}

/// Decisions that already have a link job in any state: asked, or waiting.
fn already_asked<S: JobRepository>(store: &S) -> BTreeSet<String> {
    store
        .list()
        .unwrap_or_default()
        .into_iter()
        .filter(|job| job.kind == LINK_JOB_KIND)
        .map(|job| job.payload)
        .collect()
}

/// Queues the link job of one decision unless it already has one.
pub fn queue_link_job<S: JobRepository>(store: &S, decision_id: &str) {
    if !already_asked(store).contains(decision_id) {
        insert_link_job(store, decision_id);
    }
}

/// Queues the link job of every decision in force that no file, dependency
/// or person tied to the map and that was never asked about, at most
/// [`MAX_QUEUED_PER_REFRESH`] per call (the rest on later refreshes). It does
/// nothing while the map has no live component. Called by the map refresh,
/// so decisions adopted before this job existed catch up. Returns how many
/// were queued.
pub fn queue_untied<S: JobRepository>(
    store: &S,
    decision_ids: &[&str],
    edges: &[EdgeRecord],
    components: usize,
) -> usize {
    if components == 0 {
        return 0;
    }
    let untied: Vec<&str> = decision_ids
        .iter()
        .copied()
        .filter(|id| needs_links(edges, id))
        .collect();
    if untied.is_empty() {
        return 0;
    }
    let asked = already_asked(store);
    let mut queued = 0;
    for id in untied
        .into_iter()
        .filter(|id| !asked.contains(*id))
        .take(MAX_QUEUED_PER_REFRESH)
    {
        insert_link_job(store, id);
        queued += 1;
    }
    queued
}

/// Failures of a link search, for the job log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkFindError {
    /// Storage failed.
    Storage(String),
    /// The provider failed or answered out of contract.
    Provider(String),
    /// The provider asked for a pause or timed out; the job is requeued.
    Deferred(Option<std::time::Duration>),
}

impl std::fmt::Display for LinkFindError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Storage(detail) => write!(formatter, "storage: {detail}"),
            Self::Provider(detail) => write!(formatter, "provider: {detail}"),
            Self::Deferred(_) => formatter.write_str("provider: deferred"),
        }
    }
}

impl std::error::Error for LinkFindError {}

fn storage(error: impl std::fmt::Display) -> LinkFindError {
    LinkFindError::Storage(error.to_string())
}

/// The job: the components one adopted decision applies to, proposed by the
/// model and stored as pending suggestions.
#[derive(Debug, Clone)]
pub struct LinkFinder<S, P, K, F> {
    store: S,
    settings: AiSettings<P, K>,
    factory: F,
}

impl<S, P, K, F> LinkFinder<S, P, K, F>
where
    S: DecisionStore + GraphStore,
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
        }
    }

    /// Asks the model about the decision and stores what survives the checks.
    /// Returns how many suggestions were stored; 0 without an enabled
    /// provider, without live components or when the decision already has
    /// ties.
    ///
    /// # Errors
    ///
    /// `storage`, or `provider` when the call fails.
    pub fn run(&self, decision_id: &str) -> Result<usize, LinkFindError> {
        let Some(decision) = DecisionStore::get(&self.store, decision_id)
            .map_err(storage)?
            .filter(|decision| decision.status == DecisionStatus::Accepted)
        else {
            return Ok(0);
        };
        let mut edges = self
            .store
            .project_edges(&decision.project_id)
            .map_err(storage)?;
        if !needs_links(&edges, decision_id) {
            return Ok(0);
        }
        let entities = self
            .store
            .project_entities(&decision.project_id)
            .map_err(storage)?;
        let components = candidate_components(&entities);
        if components.is_empty() {
            return Ok(0);
        }
        let profile = self.settings.load_or_seed().map_err(storage)?;
        if choose_extractor(Some(&profile)) != ExtractorChoice::ExternalEnabled {
            return Ok(0);
        }
        let secret = match self.settings.secret(&profile.credential_account()) {
            Ok(Some(secret)) => secret,
            Ok(None) if !profile.credential_required() => String::new(),
            _ => return Ok(0),
        };
        let Ok(model) = self.factory.authorized(&profile, secret, &self.settings) else {
            return Ok(0);
        };
        let subject = LinkSubject::of(&decision);
        let answer = model
            .complete(
                LINK_PROMPT,
                &link_request(&subject, &components),
                "decision_links",
                &link_schema(),
            )
            .map_err(|error| match error.job_failure() {
                crate::jobs::JobFailure::Deferred { retry_after } => {
                    LinkFindError::Deferred(retry_after)
                }
                crate::jobs::JobFailure::Failed => LinkFindError::Provider(error.to_string()),
            })?;
        let now = now_rfc3339();
        let mut stored = 0;
        for link in parse_links(&answer, &subject, &components) {
            // Any row for the same edge blocks it: pending, confirmed or
            // invalidated by a person.
            if edges.iter().any(|edge| {
                edge.kind == EdgeKind::Affects
                    && edge.source_kind == NodeKind::Decision
                    && edge.source_id == decision.decision_id
                    && edge.entity_id == link.entity_id
            }) {
                continue;
            }
            let record = EdgeRecord {
                edge_id: uuid::Uuid::now_v7().to_string(),
                project_id: decision.project_id.clone(),
                kind: EdgeKind::Affects,
                source_kind: NodeKind::Decision,
                source_id: decision.decision_id.clone(),
                entity_id: link.entity_id,
                origin: EdgeOrigin::Derived,
                reason: ai_link_reason(&link.quote, &link.reason),
                created_at: now.clone(),
                confirmed_at: None,
                invalidated_at: None,
            };
            self.store.insert_edge(&record).map_err(storage)?;
            edges.push(record);
            stored += 1;
        }
        Ok(stored)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn component(id: &str, name: &str) -> EntityRecord {
        EntityRecord {
            entity_id: id.into(),
            project_id: "p".into(),
            kind: EntityKind::Component,
            name: name.into(),
            key: domain::entities::entity_key(name),
            description: String::new(),
            patterns: vec![format!("packages/{name}/**")],
            aliases: Vec::new(),
            created_at: "2026-01-01T00:00:00Z".into(),
            retired_at: None,
        }
    }

    fn subject() -> LinkSubject {
        LinkSubject {
            question: "Como o veredito do turno é tratado?".into(),
            choice: "O núcleo devolve sempre um desfecho explícito.".into(),
            rationale: "Evita que o plugin interprete silêncio como aprovação.".into(),
            scope: vec!["Política de desfecho V0.1".into()],
        }
    }

    #[test]
    fn only_known_components_with_a_verbatim_quote_survive() {
        let (core, plugin) = (component("e-core", "core"), component("e-plugin", "plugin"));
        let components = [&core, &plugin];
        let answer = r#"{"links":[
            {"component_id":"c1","quote":"o NÚCLEO devolve sempre um desfecho explícito",
             "reason":"Rege o comportamento do núcleo."},
            {"component_id":"c2","quote":"nenhum texto da decisão diz isto","reason":"inventada"},
            {"component_id":"c9","quote":"Política de desfecho V0.1","reason":"id desconhecido"},
            {"component_id":"c1","quote":"Política de desfecho V0.1","reason":"repetido"},
            {"component_id":"c2","quote":"Evita que o plugin interprete silencio",
             "reason":"Acentos e caixa não importam."},
            {"component_id":"c2","quote":"sim","reason":"curta demais"}
        ]}"#;
        let links = parse_links(answer, &subject(), &components);
        let ids: Vec<&str> = links.iter().map(|link| link.entity_id.as_str()).collect();
        assert_eq!(ids, ["e-core", "e-plugin"]);
        assert!(parse_links("not json", &subject(), &components).is_empty());
        assert!(parse_links(r#"{"links":[]}"#, &subject(), &components).is_empty());
    }

    #[test]
    fn at_most_three_links_are_kept() {
        let all: Vec<EntityRecord> = (0..5)
            .map(|n| component(&format!("e{n}"), &format!("part{n}")))
            .collect();
        let components: Vec<&EntityRecord> = all.iter().collect();
        let links: Vec<_> = (1..=5)
            .map(|n| {
                json!({"component_id": format!("c{n}"), "quote": "Política de desfecho",
                       "reason": "r"})
            })
            .collect();
        let answer = json!({ "links": links }).to_string();
        let kept = parse_links(&answer, &subject(), &components);
        assert_eq!(kept.len(), MAX_LINKS);
    }

    #[test]
    fn the_request_lists_components_by_position_and_never_their_ids() {
        let mut core = component("e-core-secret-id", "core");
        core.aliases = vec!["núcleo".into()];
        core.description = "Regras\ndo núcleo".into();
        let request = link_request(&subject(), &[&core]);
        assert!(request.contains("Question: Como o veredito"));
        assert!(request.contains("Scope: Política de desfecho V0.1"));
        assert!(request.contains(
            "- c1 | core | aliases: núcleo | paths: packages/core/** | Regras do núcleo"
        ));
        assert!(!request.contains("e-core-secret-id"));
    }

    #[test]
    fn the_component_list_is_capped_live_and_ordered() {
        let mut all: Vec<EntityRecord> = (0..MAX_COMPONENTS + 10)
            .map(|n| component(&format!("e{n}"), &format!("part{n:03}")))
            .collect();
        all[0].retired_at = Some("2026-02-01T00:00:00Z".into());
        all[1].kind = EntityKind::Technology;
        let chosen = candidate_components(&all);
        assert_eq!(chosen.len(), MAX_COMPONENTS);
        assert_eq!(chosen[0].name, "part002");
    }

    #[test]
    fn building_the_request_and_validating_are_cheap() {
        let all: Vec<EntityRecord> = (0..MAX_COMPONENTS)
            .map(|n| component(&format!("e{n}"), &format!("part{n:03}")))
            .collect();
        let components: Vec<&EntityRecord> = all.iter().collect();
        let answer = json!({"links": [
            {"component_id": "c3", "quote": "Política de desfecho V0.1", "reason": "r"}
        ]})
        .to_string();
        let started = std::time::Instant::now();
        for _ in 0..200 {
            let _ = link_request(&subject(), &components);
            assert_eq!(parse_links(&answer, &subject(), &components).len(), 1);
        }
        let per_run = started.elapsed() / 200;
        println!("latency link_request+parse_links {per_run:?}");
        assert!(per_run < std::time::Duration::from_millis(5), "{per_run:?}");
    }

    fn edge(reason: &str, live: bool) -> EdgeRecord {
        EdgeRecord {
            edge_id: "x".into(),
            project_id: "p".into(),
            kind: EdgeKind::Affects,
            source_kind: NodeKind::Decision,
            source_id: "d".into(),
            entity_id: "e".into(),
            origin: EdgeOrigin::Derived,
            reason: reason.into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            confirmed_at: None,
            invalidated_at: (!live).then(|| "2026-01-02T00:00:00Z".into()),
        }
    }

    #[test]
    fn a_decision_needs_links_unless_tied_by_a_file_or_a_person_or_already_asked() {
        assert!(needs_links(&[], "d"));
        assert!(!needs_links(&[edge("crates/core/src/lib.rs", true)], "d"));
        assert!(!needs_links(&[edge("", true)], "d"), "a human tie");
        assert!(needs_links(&[edge("crates/core/src/lib.rs", false)], "d"));
        let mention = crate::graph::mention_reason("o core");
        assert!(needs_links(&[edge(&mention, true)], "d"));
        let asked = ai_link_reason("o core grava", "r");
        assert!(!needs_links(&[edge(&asked, false)], "d"));
        assert!(needs_links(
            &[edge("crates/core/src/lib.rs", true)],
            "other"
        ));
    }
}
