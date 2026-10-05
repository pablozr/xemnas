//! Automatic review: one switch that lets the AI handle the whole cycle of
//! approving what waits for a person, the way the permission modes of an
//! agent let it get on with the work.
//!
//! With the mode on, the candidate decisions and rules, the relations between
//! decisions, the context derived from them and the ties to the map are
//! accepted, discarded or left for the person without anyone pressing
//! Confirmar. What was done goes into a ledger, and what the AI would rather
//! not decide says why.
//!
//! The AI is not asked about everything. A call per item would be a call per
//! thing the extractor ever proposed, so each item first goes through free,
//! local rules that settle the plain cases:
//!
//! * a tie derived from a touched file or dependency is accepted (one found
//!   by a mention in the decision's text is asked, with its quote);
//! * a relation that depends on another decision (its quote was already
//!   checked verbatim) is accepted;
//! * a candidate that repeats a recorded question is discarded;
//! * a decision with high confidence, a source, few files and nothing like it
//!   recorded is accepted.
//!
//! Only what these rules cannot settle (rules, middling confidence, context
//! derived from a decision, relations that conflict with or replace another
//! decision, ties found by mention) is asked, and asked in **one batched call** per pass, with a
//! compact text per item. Passes are spaced out: the call waits until a few
//! items have gathered or the oldest has waited two hours, keeps twenty
//! minutes between calls (and after a failure), and stops at six a day. An
//! item the AI judged is never asked about again.
//!
//! The ledger is written for every verdict, accepted or not. What the AI
//! discarded is a candidate that can be put back; what it accepted is open in
//! the page of the decision or rule it created, where it is edited or ended.

use std::collections::BTreeSet;
use std::sync::Arc;

use domain::relations::RelationKind;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::adoption::AdoptionApi;
use crate::analysis::ExtractorFactory;
use crate::claim_suggestions::{ClaimSuggestionStore, ClaimSuggestions};
use crate::claims::ClaimStore;
use crate::clock::{add_hours, add_seconds, now_rfc3339};
use crate::decisions::DecisionStore;
use crate::extract::{CandidateKind, MIN_SIGNIFICANCE};
use crate::graph::{mention_quote, GraphStore, KnowledgeGraph, Suggestion};
use crate::inbox::{
    CandidateStatus, Cursor, Inbox, InboxQuery, InboxStore, StoredCandidate, MAX_PAGE_LIMIT,
};
use crate::overview::StructuredModel;
use crate::profile::{choose_extractor, AiSettings, ExtractorChoice, ProfileStore, SecretStore};
use crate::projects::ProjectRepository;
use crate::relation_suggestions::{RelationSuggestionStore, RelationSuggestions};
use crate::relations::RelationStore;

/// Items sent to the AI in one call.
pub const BATCH: usize = 12;
/// Ambiguous items that make a call worth it before the oldest has waited.
pub const MIN_TO_ASK: usize = 3;
/// Hours the oldest ambiguous item may wait for company.
pub const MAX_WAIT_HOURS: i64 = 2;
/// Minutes kept between calls, and after a failed one.
pub const CALL_GAP_MINUTES: i64 = 20;
/// Calls per day, at most.
pub const CALLS_PER_DAY: usize = 6;
/// Confidence from which a plain decision is accepted without asking.
pub const SURE_CONFIDENCE: f64 = 0.85;
/// Word overlap from which a candidate repeats a recorded question.
pub const DUPLICATE_AT: f64 = 0.85;
/// Word overlap from which a candidate merely resembles one.
pub const SIMILAR_AT: f64 = 0.6;
/// Most files a decision may touch and still be settled by the rules.
pub const MAX_FILES: usize = 3;
/// Candidates read at most per pass.
const READ_LIMIT: usize = 300;
/// Characters of a rationale or quote sent to the AI.
const SENT_CHARS: usize = 280;

/// How approval works.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// A person decides everything (the default).
    #[default]
    Manual,
    /// The AI handles the cycle.
    Automatic,
}

impl Mode {
    /// The literal persisted in the settings table.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Automatic => "automatic",
        }
    }

    /// Parses a persisted literal.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "manual" => Some(Self::Manual),
            "automatic" => Some(Self::Automatic),
            _ => None,
        }
    }
}

/// What kind of thing was reviewed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    /// A candidate decision or rule.
    Candidate,
    /// A relation between two decisions.
    Relation,
    /// Context derived from a decision.
    Claim,
    /// A tie between a decision and the map.
    Link,
}

impl ItemKind {
    /// The literal persisted in the ledger.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Candidate => "candidate",
            Self::Relation => "relation",
            Self::Claim => "claim",
            Self::Link => "link",
        }
    }

    /// Parses a persisted literal.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "candidate" => Some(Self::Candidate),
            "relation" => Some(Self::Relation),
            "claim" => Some(Self::Claim),
            "link" => Some(Self::Link),
            _ => None,
        }
    }
}

/// What was decided about an item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    /// Confirmed.
    Accepted,
    /// Rejected.
    Discarded,
    /// Left for the person.
    NeedsHuman,
}

impl Verdict {
    /// The literal persisted in the ledger.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Discarded => "discarded",
            Self::NeedsHuman => "needs_human",
        }
    }

    /// Parses a persisted literal.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "accepted" => Some(Self::Accepted),
            "discarded" => Some(Self::Discarded),
            "needs_human" => Some(Self::NeedsHuman),
            _ => None,
        }
    }
}

/// Who decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum By {
    /// The local rules.
    Rules,
    /// The AI.
    Ai,
}

impl By {
    /// The literal persisted in the ledger.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Rules => "rules",
            Self::Ai => "ai",
        }
    }

    /// Parses a persisted literal.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "rules" => Some(Self::Rules),
            "ai" => Some(Self::Ai),
            _ => None,
        }
    }
}

/// One line of the ledger.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// What kind of item.
    pub kind: ItemKind,
    /// Its id (candidate id, suggestion id or edge id).
    pub item_id: String,
    /// Its project.
    pub project_id: String,
    /// What was decided.
    pub verdict: Verdict,
    /// Who decided.
    pub by: By,
    /// Why, in a sentence.
    pub reason: String,
    /// What the item says, for the list.
    pub title: String,
    /// The decision or rule that accepting created.
    pub result_id: Option<String>,
    /// RFC 3339 moment.
    pub created_at: String,
    /// RFC 3339 moment a person put it back, when they did.
    pub undone_at: Option<String>,
}

/// What one pass did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RunReport {
    /// Items accepted.
    pub accepted: usize,
    /// Items discarded.
    pub discarded: usize,
    /// Items left for the person.
    pub left: usize,
    /// Whether the AI was asked.
    pub asked: bool,
}

impl RunReport {
    /// Whether the pass changed what a person sees in the queue.
    pub fn changed(&self) -> bool {
        self.accepted + self.discarded + self.left > 0
    }
}

/// Where the mode stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Status {
    /// The saved mode.
    pub automatic: bool,
    /// Whether an AI provider is enabled to judge what the rules cannot.
    pub judge: bool,
}

/// Failure modes of the review use case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReviewError {
    /// A storage query failed; the message is diagnostic only.
    Storage(String),
    /// The AI failed or answered out of contract.
    Provider(String),
    /// Something could not be applied.
    Apply(String),
    /// The item cannot be put back.
    NotUndoable,
}

impl std::fmt::Display for ReviewError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Storage(message) => write!(formatter, "armazenamento: {message}"),
            Self::Provider(message) => write!(formatter, "provedor: {message}"),
            Self::Apply(message) => write!(formatter, "não foi possível aplicar: {message}"),
            Self::NotUndoable => write!(formatter, "este item não pode ser desfeito aqui"),
        }
    }
}

impl std::error::Error for ReviewError {}

impl From<crate::inbox::InboxError> for ReviewError {
    fn from(error: crate::inbox::InboxError) -> Self {
        Self::Storage(error.to_string())
    }
}

/// Persistence the review needs besides the stores of the items.
pub trait ApprovalStore {
    /// The saved mode (manual when nothing was saved).
    fn approval_mode(&self) -> Result<Mode, ReviewError>;

    /// Saves the mode.
    fn set_approval_mode(&self, mode: Mode, at: &str) -> Result<(), ReviewError>;

    /// Items of a project already reviewed, whatever the verdict.
    fn reviewed(&self, project_id: &str) -> Result<BTreeSet<(ItemKind, String)>, ReviewError>;

    /// Records a verdict.
    fn record_review(&self, entry: &Entry) -> Result<(), ReviewError>;

    /// The ledger of a project, newest first.
    fn review_ledger(&self, project_id: &str, limit: usize) -> Result<Vec<Entry>, ReviewError>;

    /// Marks an entry as put back.
    fn mark_undone(&self, kind: ItemKind, item_id: &str, at: &str) -> Result<(), ReviewError>;

    /// The calls to the AI since `since`, as (moment, succeeded), oldest first.
    fn review_calls(&self, since: &str) -> Result<Vec<(String, bool)>, ReviewError>;

    /// Records a call to the AI.
    fn record_review_call(&self, at: &str, items: usize, ok: bool) -> Result<(), ReviewError>;
}

/// Object-safe entry point for the desktop.
pub trait ApprovalsApi: Send + Sync {
    /// Where the mode stands.
    fn status(&self) -> Result<Status, ReviewError>;
    /// Turns the automatic mode on or off.
    fn set_mode(&self, mode: Mode) -> Result<(), ReviewError>;
    /// One pass for a project.
    fn run(&self, project_id: &str) -> Result<RunReport, ReviewError>;
    /// What the review decided in a project.
    fn ledger(&self, project_id: &str) -> Result<Vec<Entry>, ReviewError>;
    /// Puts a discarded candidate back in the queue.
    fn undo(&self, kind: ItemKind, item_id: &str) -> Result<(), ReviewError>;
}

/// What the local rules say about an item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Triage {
    /// Accept, for this reason.
    Accept(&'static str),
    /// Discard, for this reason.
    Discard(&'static str),
    /// The rules cannot tell: ask the AI.
    Ask,
}

/// The words of a question, lowercase, without short noise words.
fn words(text: &str) -> BTreeSet<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .map(str::to_lowercase)
        .filter(|word| word.chars().count() > 2)
        .collect()
}

/// Overlap of two word sets, 0 to 1.
pub fn similarity(a: &str, b: &str) -> f64 {
    let (a, b) = (words(a), words(b));
    let union = a.union(&b).count();
    if union == 0 {
        return 0.0;
    }
    a.intersection(&b).count() as f64 / union as f64
}

/// Sources an evidence list holds.
fn sources(candidate: &StoredCandidate) -> usize {
    serde_json::from_str::<Vec<serde_json::Value>>(&candidate.evidence_refs)
        .map(|refs| refs.len())
        .unwrap_or(0)
}

/// What the rules say about a pending candidate, given the questions already
/// recorded in its project.
pub fn triage_candidate(candidate: &StoredCandidate, recorded: &[String]) -> Triage {
    let likeness = recorded
        .iter()
        .map(|question| similarity(question, &candidate.question))
        .fold(0.0, f64::max);
    if likeness >= DUPLICATE_AT {
        return Triage::Discard("repete uma decisão já registrada");
    }
    if candidate.kind != CandidateKind::Decision.as_str() {
        return Triage::Ask;
    }
    let files = serde_json::from_str::<serde_json::Value>(&candidate.diff_summary)
        .ok()
        .and_then(|summary| {
            summary
                .get("files")
                .and_then(|files| files.as_array().map(Vec::len))
        })
        .unwrap_or(0);
    if candidate.confidence >= SURE_CONFIDENCE
        && candidate.significance >= MIN_SIGNIFICANCE
        && sources(candidate) >= 1
        && files <= MAX_FILES
        && likeness < SIMILAR_AT
    {
        Triage::Accept("alta confiança, com fonte e sem parecido registrado")
    } else {
        Triage::Ask
    }
}

/// What the rules say about a relation between decisions.
pub fn triage_relation(kind: RelationKind) -> Triage {
    match kind {
        RelationKind::DependsOn => {
            Triage::Accept("depende de outra decisão, com citação conferida")
        }
        RelationKind::Supersedes | RelationKind::ConflictsWith => Triage::Ask,
    }
}

/// What the rules say about a suggested tie to the map: one derived from a
/// touched file or an added dependency is plain; one found by a mention in
/// the decision's text may be a passing remark, so the AI judges its quote.
pub fn triage_link(reason: &str) -> Triage {
    if mention_quote(reason).is_some() {
        Triage::Ask
    } else {
        Triage::Accept("derivado de arquivo ou dependência que a decisão tocou")
    }
}

/// What the AI is told about a suggested tie.
fn link_text(suggestion: &Suggestion) -> String {
    let evidence = match mention_quote(&suggestion.reason) {
        Some(quote) => format!("Citação: {}", clip(quote, SENT_CHARS)),
        None => format!("Arquivo ou dependência: {}", clip(&suggestion.reason, 160)),
    };
    format!(
        "[vínculo {}] \"{}\" -> {} \"{}\" | {evidence}",
        suggestion.kind.as_str(),
        clip(&suggestion.source.label, 160),
        suggestion.entity.detail,
        clip(&suggestion.entity.label, 80),
    )
}

/// An item of a pass, with what the AI would be told about it.
struct Item {
    kind: ItemKind,
    id: String,
    title: String,
    text: String,
    waiting_since: String,
    triage: Triage,
}

/// System prompt of the judge.
pub const REVIEW_PROMPT: &str = concat!(
    "You help a developer keep the memory of the engineering \
decisions of a software project. Items wait for the developer to approve them; you decide \
which can go ahead without them. For each item answer accept (it is clearly right, useful \
and safe for coding agents to rely on), discard (it is wrong, trivial or repeats something) \
or human (it is ambiguous, high impact, contradicts something or you are not sure). When in \
doubt answer human: a wrong accept becomes context given to agents. A rule or relation that \
conflicts with or replaces another decision is accept only when its quote clearly supports it. \
A link from a decision to a part of the project map found by a mention is accept only when \
its quote shows the decision really affects or uses that part, not a passing remark.\n\
Give in reason one short sentence in the language of the item. Reply with one JSON object \
only, matching exactly: {\"verdicts\":[{\"id\":string,\"verdict\":\"accept|discard|human\",\
\"reason\":string}]}, one entry per item, using its id exactly as given.",
    crate::plain_rules!()
);

/// Strict schema of the judge's answer.
pub fn review_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["verdicts"],
        "properties": {
            "verdicts": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["id", "verdict", "reason"],
                    "properties": {
                        "id": { "type": "string" },
                        "verdict": { "type": "string", "enum": ["accept", "discard", "human"] },
                        "reason": { "type": "string" }
                    }
                }
            }
        }
    })
}

#[derive(Deserialize)]
struct RawVerdicts {
    #[serde(default)]
    verdicts: Vec<RawVerdict>,
}

#[derive(Deserialize)]
struct RawVerdict {
    id: String,
    verdict: String,
    #[serde(default)]
    reason: String,
}

/// The verdicts of an answer, by the ids given in the prompt; anything the
/// answer left out or garbled is for the person.
pub fn parse_verdicts(text: &str, ids: &[String]) -> Vec<(String, Verdict, String)> {
    let start = text.find('{').unwrap_or(0);
    let end = text.rfind('}').map_or(text.len(), |end| end + 1);
    let raw: RawVerdicts = serde_json::from_str(text.get(start..end).unwrap_or(text))
        .unwrap_or(RawVerdicts { verdicts: vec![] });
    ids.iter()
        .map(
            |id| match raw.verdicts.iter().find(|verdict| verdict.id == *id) {
                Some(verdict) => {
                    let reason: String = verdict.reason.trim().chars().take(240).collect();
                    match verdict.verdict.as_str() {
                        "accept" => (id.clone(), Verdict::Accepted, reason),
                        "discard" => (id.clone(), Verdict::Discarded, reason),
                        _ => (id.clone(), Verdict::NeedsHuman, reason),
                    }
                }
                None => (
                    id.clone(),
                    Verdict::NeedsHuman,
                    "a IA não respondeu sobre este item".to_owned(),
                ),
            },
        )
        .collect()
}

fn clip(text: &str, max: usize) -> String {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.chars().count() <= max {
        text
    } else {
        let cut: String = text.chars().take(max.saturating_sub(1)).collect();
        format!("{}…", cut.trim_end())
    }
}

/// `timestamp` moved by `minutes`, in the same shape.
fn add_minutes(timestamp: &str, minutes: i64) -> Option<String> {
    add_seconds(timestamp, minutes * 60)
}

/// The review use case.
pub struct Approvals<S, P, K, F> {
    store: S,
    adoption: Arc<dyn AdoptionApi>,
    settings: AiSettings<P, K>,
    factory: F,
}

impl<S, P, K, F> Approvals<S, P, K, F>
where
    S: ApprovalStore
        + InboxStore
        + RelationSuggestionStore
        + ClaimSuggestionStore
        + ClaimStore
        + DecisionStore
        + GraphStore
        + RelationStore
        + ProjectRepository
        + Clone,
    P: ProfileStore,
    K: SecretStore,
    F: ExtractorFactory,
    F::Extractor: StructuredModel,
{
    /// Wraps the store, the adoption path, the AI settings and the provider.
    pub fn new(
        store: S,
        adoption: Arc<dyn AdoptionApi>,
        settings: AiSettings<P, K>,
        factory: F,
    ) -> Self {
        Self {
            store,
            adoption,
            settings,
            factory,
        }
    }

    /// Candidates of a project by status, newest first.
    fn candidates(
        &self,
        project_id: &str,
        statuses: Vec<CandidateStatus>,
        limit: usize,
    ) -> Result<Vec<StoredCandidate>, ReviewError> {
        let mut found = Vec::new();
        let mut query = InboxQuery {
            min_significance: None,
            project_id: Some(project_id.to_owned()),
            statuses,
            limit: MAX_PAGE_LIMIT,
            before: None,
        };
        while found.len() < limit {
            let rows = InboxStore::list(&self.store, &query)?;
            let full = rows.len() == query.limit;
            if let Some(last) = rows.last() {
                query.before = Some(Cursor {
                    created_at: last.created_at.clone(),
                    id: last.id.clone(),
                });
            }
            found.extend(rows);
            if !full {
                break;
            }
        }
        found.truncate(limit);
        Ok(found)
    }

    /// Everything of a project that waits for a person and has not been
    /// reviewed yet, with what the rules say about it.
    fn gather(&self, project_id: &str) -> Result<Vec<Item>, ReviewError> {
        let done = self.store.reviewed(project_id)?;
        let storage = |detail: String| ReviewError::Storage(detail);
        let mut items = Vec::new();

        let pending = self.candidates(project_id, vec![CandidateStatus::Pending], READ_LIMIT)?;
        let recorded: Vec<String> = self
            .candidates(
                project_id,
                vec![
                    CandidateStatus::Accepted,
                    CandidateStatus::EditedAndAccepted,
                    CandidateStatus::Dismissed,
                ],
                READ_LIMIT,
            )?
            .into_iter()
            .map(|row| row.question)
            .collect();
        for candidate in pending {
            if candidate.significance < MIN_SIGNIFICANCE
                || done.contains(&(ItemKind::Candidate, candidate.id.clone()))
            {
                continue;
            }
            let triage = triage_candidate(&candidate, &recorded);
            let kind = if candidate.kind == CandidateKind::Rule.as_str() {
                "regra"
            } else {
                "decisão"
            };
            items.push(Item {
                kind: ItemKind::Candidate,
                id: candidate.id.clone(),
                title: candidate.question.clone(),
                text: format!(
                    "[{kind}] Pergunta: {} | Escolha: {} | Motivo: {} | Confiança do extrator: \
                     {:.0}% | Fontes: {}",
                    clip(&candidate.question, 200),
                    clip(&candidate.choice, 200),
                    clip(&candidate.rationale, SENT_CHARS),
                    candidate.confidence * 100.0,
                    sources(&candidate)
                ),
                waiting_since: candidate.created_at.clone(),
                triage,
            });
        }

        let relations = RelationSuggestions::new(self.store.clone());
        for view in relations
            .pending(project_id)
            .map_err(|error| storage(error.to_string()))?
        {
            let record = &view.record;
            if done.contains(&(ItemKind::Relation, record.suggestion_id.clone())) {
                continue;
            }
            items.push(Item {
                kind: ItemKind::Relation,
                id: record.suggestion_id.clone(),
                title: format!(
                    "{} {} {}",
                    view.from_question,
                    record.kind.as_str(),
                    view.to_question
                ),
                text: format!(
                    "[relação {}] \"{}\" -> \"{}\" | Citação: {} | Motivo: {}",
                    record.kind.as_str(),
                    clip(&view.from_question, 160),
                    clip(&view.to_question, 160),
                    clip(&record.quote, SENT_CHARS),
                    clip(&record.reason, SENT_CHARS)
                ),
                waiting_since: record.created_at.clone(),
                triage: triage_relation(record.kind),
            });
        }

        let claims = ClaimSuggestions::new(self.store.clone());
        for view in claims
            .pending(project_id)
            .map_err(|error| storage(error.to_string()))?
        {
            let record = &view.record;
            if done.contains(&(ItemKind::Claim, record.suggestion_id.clone())) {
                continue;
            }
            items.push(Item {
                kind: ItemKind::Claim,
                id: record.suggestion_id.clone(),
                title: record.statement.clone(),
                text: format!(
                    "[contexto {}] {} | Citação: {} | Da decisão: {}",
                    record.kind.as_str(),
                    clip(&record.statement, 200),
                    clip(&record.quote, SENT_CHARS),
                    clip(&view.decision_question, 160)
                ),
                waiting_since: record.created_at.clone(),
                triage: Triage::Ask,
            });
        }

        let graph = KnowledgeGraph::new(self.store.clone());
        for suggestion in graph
            .suggestions(project_id)
            .map_err(|error| storage(error.to_string()))?
        {
            if done.contains(&(ItemKind::Link, suggestion.edge_id.clone())) {
                continue;
            }
            items.push(Item {
                kind: ItemKind::Link,
                id: suggestion.edge_id.clone(),
                title: format!("{} → {}", suggestion.source.label, suggestion.entity.label),
                text: link_text(&suggestion),
                waiting_since: suggestion.created_at.clone(),
                triage: triage_link(&suggestion.reason),
            });
        }
        Ok(items)
    }

    /// Accepts or discards one item through the use case a person's press
    /// would run; the id of what accepting created, when it created one.
    fn apply(&self, item: &Item, accept: bool) -> Result<Option<String>, ReviewError> {
        let failed = |detail: String| ReviewError::Apply(detail);
        match (item.kind, accept) {
            (ItemKind::Candidate, true) => {
                let links = self
                    .adoption
                    .preview(&item.id)
                    .map(|preview| preview.links)
                    .unwrap_or_default();
                let outcome = self
                    .adoption
                    .adopt(&item.id, None, &links, &[])
                    .map_err(|error| failed(error.to_string()))?;
                Ok(Some(outcome.id))
            }
            (ItemKind::Candidate, false) => Inbox::new(self.store.clone())
                .reject(&item.id)
                .map(|_| None)
                .map_err(|error| failed(error.to_string())),
            (ItemKind::Relation, true) => RelationSuggestions::new(self.store.clone())
                .confirm(&item.id)
                .map(|_| None)
                .map_err(|error| failed(error.to_string())),
            (ItemKind::Relation, false) => RelationSuggestions::new(self.store.clone())
                .reject(&item.id)
                .map(|_| None)
                .map_err(|error| failed(error.to_string())),
            (ItemKind::Claim, true) => ClaimSuggestions::new(self.store.clone())
                .confirm(&item.id)
                .map(Some)
                .map_err(|error| failed(error.to_string())),
            (ItemKind::Claim, false) => ClaimSuggestions::new(self.store.clone())
                .reject(&item.id)
                .map(|_| None)
                .map_err(|error| failed(error.to_string())),
            (ItemKind::Link, true) => KnowledgeGraph::new(self.store.clone())
                .confirm(&item.id)
                .map(|_| None)
                .map_err(|error| failed(error.to_string())),
            (ItemKind::Link, false) => KnowledgeGraph::new(self.store.clone())
                .invalidate(&item.id)
                .map(|_| None)
                .map_err(|error| failed(error.to_string())),
        }
    }

    /// Applies a verdict and writes it to the ledger. An item that cannot be
    /// applied (a person got there first) is skipped without a record.
    #[allow(clippy::too_many_arguments)]
    fn settle(
        &self,
        project_id: &str,
        item: &Item,
        verdict: Verdict,
        by: By,
        reason: &str,
        now: &str,
        report: &mut RunReport,
    ) -> Result<(), ReviewError> {
        let result_id = match verdict {
            Verdict::NeedsHuman => None,
            Verdict::Accepted | Verdict::Discarded => {
                match self.apply(item, verdict == Verdict::Accepted) {
                    Ok(result) => result,
                    // A person got there first, or the item changed: nothing to record.
                    Err(_) => {
                        return Ok(());
                    }
                }
            }
        };
        self.store.record_review(&Entry {
            kind: item.kind,
            item_id: item.id.clone(),
            project_id: project_id.to_owned(),
            verdict,
            by,
            reason: reason.to_owned(),
            title: clip(&item.title, 200),
            result_id,
            created_at: now.to_owned(),
            undone_at: None,
        })?;
        match verdict {
            Verdict::Accepted => report.accepted += 1,
            Verdict::Discarded => report.discarded += 1,
            Verdict::NeedsHuman => report.left += 1,
        }
        Ok(())
    }

    /// The AI, when a provider is enabled and consented.
    fn judge(&self) -> Option<F::Extractor> {
        let profile = self.settings.load_or_seed().ok()?;
        if choose_extractor(Some(&profile)) != ExtractorChoice::ExternalEnabled {
            return None;
        }
        let secret = match self.settings.secret(&profile.credential_account()) {
            Ok(Some(secret)) => secret,
            Ok(None) if !profile.credential_required() => String::new(),
            _ => return None,
        };
        self.factory.external(&profile, secret).ok()
    }

    /// Whether a call is allowed and worth making now.
    fn may_ask(&self, ambiguous: &[&Item], now: &str) -> Result<bool, ReviewError> {
        let day_ago = add_hours(now, -24).unwrap_or_default();
        let calls = self.store.review_calls(&day_ago)?;
        if calls.len() >= CALLS_PER_DAY {
            return Ok(false);
        }
        if let Some((last, _)) = calls.last() {
            let ready_at = add_minutes(last, CALL_GAP_MINUTES).unwrap_or_default();
            if now < ready_at.as_str() {
                return Ok(false);
            }
        }
        let oldest = ambiguous
            .iter()
            .map(|item| item.waiting_since.as_str())
            .filter(|moment| !moment.is_empty())
            .min()
            .unwrap_or(now);
        let patient = add_hours(oldest, MAX_WAIT_HOURS).unwrap_or_default();
        Ok(ambiguous.len() >= MIN_TO_ASK || now >= patient.as_str())
    }

    /// One pass at the given moment (RFC 3339), so tests do not wait.
    pub fn run_at(&self, project_id: &str, now: &str) -> Result<RunReport, ReviewError> {
        let mut report = RunReport::default();
        if self.store.approval_mode()? != Mode::Automatic {
            return Ok(report);
        }
        let items = self.gather(project_id)?;

        // The plain cases, settled by the rules for free.
        for item in &items {
            let (verdict, reason) = match &item.triage {
                Triage::Accept(reason) => (Verdict::Accepted, *reason),
                Triage::Discard(reason) => (Verdict::Discarded, *reason),
                Triage::Ask => continue,
            };
            self.settle(
                project_id,
                item,
                verdict,
                By::Rules,
                reason,
                now,
                &mut report,
            )?;
        }

        // The rest, in one batched call when it is worth one.
        let ambiguous: Vec<&Item> = items
            .iter()
            .filter(|item| item.triage == Triage::Ask)
            .collect();
        if ambiguous.is_empty() || !self.may_ask(&ambiguous, now)? {
            return Ok(report);
        }
        let Some(model) = self.judge() else {
            return Ok(report);
        };
        let mut oldest_first = ambiguous;
        oldest_first.sort_by(|a, b| a.waiting_since.cmp(&b.waiting_since));
        let batch: Vec<&Item> = oldest_first.into_iter().take(BATCH).collect();
        let ids: Vec<String> = (1..=batch.len()).map(|at| format!("I{at}")).collect();
        let mut user = String::from("## Items\n");
        for (id, item) in ids.iter().zip(&batch) {
            user.push_str(&format!("- {id} {}\n", item.text));
        }
        report.asked = true;
        let answer =
            match model.complete(REVIEW_PROMPT, &user, "approval_verdicts", &review_schema()) {
                Ok(answer) => {
                    self.store.record_review_call(now, batch.len(), true)?;
                    answer
                }
                Err(error) => {
                    self.store.record_review_call(now, batch.len(), false)?;
                    return Err(ReviewError::Provider(error.to_string()));
                }
            };
        for ((_, verdict, reason), item) in parse_verdicts(&answer, &ids).into_iter().zip(&batch) {
            let reason = if reason.is_empty() {
                "decidido pela IA".to_owned()
            } else {
                reason
            };
            self.settle(project_id, item, verdict, By::Ai, &reason, now, &mut report)?;
        }
        Ok(report)
    }
}

impl<S, P, K, F> ApprovalsApi for Approvals<S, P, K, F>
where
    S: ApprovalStore
        + InboxStore
        + RelationSuggestionStore
        + ClaimSuggestionStore
        + ClaimStore
        + DecisionStore
        + GraphStore
        + RelationStore
        + ProjectRepository
        + Clone
        + Send
        + Sync,
    P: ProfileStore + Send + Sync,
    K: SecretStore + Send + Sync,
    F: ExtractorFactory + Send + Sync,
    F::Extractor: StructuredModel,
{
    fn status(&self) -> Result<Status, ReviewError> {
        Ok(Status {
            automatic: self.store.approval_mode()? == Mode::Automatic,
            judge: self.judge().is_some(),
        })
    }

    fn set_mode(&self, mode: Mode) -> Result<(), ReviewError> {
        self.store.set_approval_mode(mode, &now_rfc3339())
    }

    fn run(&self, project_id: &str) -> Result<RunReport, ReviewError> {
        self.run_at(project_id, &now_rfc3339())
    }

    fn ledger(&self, project_id: &str) -> Result<Vec<Entry>, ReviewError> {
        self.store.review_ledger(project_id, 60)
    }

    fn undo(&self, kind: ItemKind, item_id: &str) -> Result<(), ReviewError> {
        if kind != ItemKind::Candidate {
            return Err(ReviewError::NotUndoable);
        }
        Inbox::new(self.store.clone())
            .reopen(item_id)
            .map_err(|_| ReviewError::NotUndoable)?;
        self.store.mark_undone(kind, item_id, &now_rfc3339())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(question: &str) -> StoredCandidate {
        StoredCandidate {
            kind: "decision".into(),
            significance: 0.9,
            criteria: "[]".into(),
            qualifiers: "[]".into(),
            id: "c1".into(),
            project_id: "p".into(),
            project_location: "C:/p".into(),
            capture_id: "cap".into(),
            status: CandidateStatus::Pending,
            question: question.into(),
            choice: "x".into(),
            rationale: "y".into(),
            signals: "[]".into(),
            confidence: 0.9,
            confidence_reason: "z".into(),
            evidence_refs: "[\"art-1\"]".into(),
            diff_summary: "{\"files\":[\"a.rs\"],\"artifacts\":1}".into(),
            adapter: None,
            session_id: None,
            observed_at: None,
            received_at: "2026-01-01T00:00:00Z".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        }
    }

    #[test]
    fn a_plain_confident_sourced_novel_decision_is_accepted_by_the_rules() {
        let recorded = vec!["Qual fila usar para os jobs?".to_owned()];
        assert!(matches!(
            triage_candidate(
                &candidate("Como versionar as decisões revisadas?"),
                &recorded
            ),
            Triage::Accept(_)
        ));
    }

    #[test]
    fn a_repeated_question_is_discarded_by_the_rules() {
        let recorded = vec!["Como versionar as decisões revisadas?".to_owned()];
        assert!(matches!(
            triage_candidate(
                &candidate("Como versionar as decisões revisadas?"),
                &recorded
            ),
            Triage::Discard(_)
        ));
    }

    #[test]
    fn what_the_rules_cannot_settle_is_asked() {
        let none: Vec<String> = Vec::new();
        let base = candidate("Como versionar as decisões revisadas?");
        let mut low = base.clone();
        low.confidence = 0.7;
        assert_eq!(triage_candidate(&low, &none), Triage::Ask);
        let mut rule = base.clone();
        rule.kind = "rule".into();
        assert_eq!(triage_candidate(&rule, &none), Triage::Ask);
        let mut unsourced = base.clone();
        unsourced.evidence_refs = "[]".into();
        assert_eq!(triage_candidate(&unsourced, &none), Triage::Ask);
        let mut wide = base.clone();
        wide.diff_summary = "{\"files\":[\"a\",\"b\",\"c\",\"d\"]}".into();
        assert_eq!(triage_candidate(&wide, &none), Triage::Ask);
        let alike = vec!["Como versionar as decisões revisadas depois?".to_owned()];
        assert_eq!(
            triage_candidate(&base, &alike),
            Triage::Ask,
            "resembles one without repeating it"
        );
    }

    #[test]
    fn only_a_relation_that_depends_on_another_decision_is_plain() {
        assert!(matches!(
            triage_relation(RelationKind::DependsOn),
            Triage::Accept(_)
        ));
        assert_eq!(triage_relation(RelationKind::Supersedes), Triage::Ask);
        assert_eq!(triage_relation(RelationKind::ConflictsWith), Triage::Ask);
    }

    #[test]
    fn a_link_by_mention_is_asked_and_one_by_file_or_dependency_is_plain() {
        assert!(matches!(
            triage_link("crates/core/src/lib.rs"),
            Triage::Accept(_)
        ));
        assert!(matches!(triage_link("rusqlite"), Triage::Accept(_)));
        let mention = crate::graph::mention_reason("o core grava pela outbox");
        assert_eq!(triage_link(&mention), Triage::Ask);
    }

    #[test]
    fn verdicts_follow_the_ids_and_anything_missing_is_for_the_person() {
        let ids = vec!["I1".to_owned(), "I2".to_owned(), "I3".to_owned()];
        let answer = r#"```json
        {"verdicts":[
          {"id":"I1","verdict":"accept","reason":"correta e útil"},
          {"id":"I2","verdict":"discard","reason":"repete outra"},
          {"id":"I9","verdict":"accept","reason":"não existe"}]}
        ```"#;
        let parsed = parse_verdicts(answer, &ids);
        assert_eq!(parsed[0].1, Verdict::Accepted);
        assert_eq!(parsed[1].1, Verdict::Discarded);
        assert_eq!(parsed[2].1, Verdict::NeedsHuman, "unanswered");
        assert!(parse_verdicts("not json", &ids)
            .iter()
            .all(|(_, verdict, _)| *verdict == Verdict::NeedsHuman));
    }

    #[test]
    fn similarity_is_word_overlap_and_ignores_case_and_short_words() {
        assert_eq!(
            similarity("Onde guardar a chave?", "onde GUARDAR a chave"),
            1.0
        );
        assert!(similarity("Qual banco usar?", "Como versionar decisões?") < 0.1);
        assert_eq!(similarity("", ""), 0.0);
    }

    #[test]
    fn minutes_are_added_across_the_hour() {
        assert_eq!(
            add_minutes("2026-03-01T10:50:00Z", 20).as_deref(),
            Some("2026-03-01T11:10:00Z")
        );
        assert_eq!(
            add_minutes("2026-03-01T23:50:30Z", 20).as_deref(),
            Some("2026-03-02T00:10:30Z")
        );
    }
}
