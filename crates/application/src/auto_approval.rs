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
//! * a rule or context suggestion that repeats a rule in force, or another one of the
//!   batch, is discarded (the judge sees the rules it only resembles);
//! * a decision with high confidence, a source, few files and nothing like it
//!   recorded is accepted.
//!
//! Only what these rules cannot settle (rules, middling confidence, context
//! derived from a decision, relations that conflict with or replace another
//! decision, ties found by mention) is asked, and asked in **one batched call** per pass, with a
//! compact text per item and up to [`BATCH`] items. Calls are not spaced or
//! capped, so a backlog drains pass after pass; only a failed call pauses the
//! next for [`FAILURE_PAUSE_MINUTES`]. An item the AI judged is never asked
//! about again.
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
use crate::conflicts::{ConflictView, Conflicts, Resolution};
use crate::decisions::{DecisionQuery, DecisionStatus, DecisionStore, StoredDecision};
use crate::extract::{CandidateKind, MIN_SIGNIFICANCE};
use crate::graph::{
    ai_link_quote, ai_link_why, mention_quote, GraphStore, KnowledgeGraph, Suggestion,
};
use crate::inbox::{
    CandidateStatus, Cursor, Inbox, InboxQuery, InboxStore, StoredCandidate, MAX_PAGE_LIMIT,
};
use crate::overview::StructuredModel;
use crate::profile::{choose_extractor, AiSettings, ExtractorChoice, ProfileStore, SecretStore};
use crate::projects::ProjectRepository;
use crate::relation_suggestions::{RelationSuggestionStore, RelationSuggestions};
use crate::relations::RelationStore;

/// Items sent to the AI in one call: each is a short text, so a call carries
/// a backlog's worth without growing much.
pub const BATCH: usize = 30;
/// Minutes kept after a failed call, so a provider that is down is not
/// asked again on every pass. Calls that work are not spaced or capped: the
/// person chose to let the AI do the review (05/10/2026).
pub const FAILURE_PAUSE_MINUTES: i64 = 20;
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
/// Decisions in force named beside a candidate decision for the judge.
const NEIGHBORS: usize = 3;
/// Word overlap from which a decision in force is worth naming beside a
/// candidate.
const NEIGHBOR_AT: f64 = 0.2;
/// Rules in force named beside a rule item that only resembles them.
const RULE_NEIGHBORS: usize = 3;
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

/// What the other side of a conflict is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictKind {
    /// Another candidate waiting for review.
    Candidate,
    /// A decision in force.
    Decision,
}

impl ConflictKind {
    /// The literal persisted in the ledger.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Candidate => "candidate",
            Self::Decision => "decision",
        }
    }

    /// Parses a persisted literal.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "candidate" => Some(Self::Candidate),
            "decision" => Some(Self::Decision),
            _ => None,
        }
    }
}

/// The other side of a conflict the judge named.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Conflict {
    /// What kind of item it is.
    pub kind: ConflictKind,
    /// Its id (candidate id or decision id).
    pub id: String,
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
    /// The item this one contradicts, when the judge named it.
    pub conflicts_with: Option<Conflict>,
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
    /// The conflict no longer stands: a side was resolved or dropped.
    Stale,
}

impl std::fmt::Display for ReviewError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Storage(message) => write!(formatter, "armazenamento: {message}"),
            Self::Provider(message) => write!(formatter, "provedor: {message}"),
            Self::Apply(message) => write!(formatter, "não foi possível aplicar: {message}"),
            Self::NotUndoable => write!(formatter, "este item não pode ser desfeito aqui"),
            Self::Stale => write!(formatter, "o conflito mudou; atualize a fila"),
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
    /// The two sides of the conflict a ledger entry names, when it still
    /// stands.
    fn conflict(&self, entry: &Entry) -> Result<Option<ConflictView>, ReviewError>;
    /// Applies the person's pick for a candidate against what it contradicts.
    fn resolve(
        &self,
        candidate_id: &str,
        conflict: &Conflict,
        resolution: &Resolution,
    ) -> Result<(), ReviewError>;
}

/// What the local rules say about an item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Triage {
    /// Accept, for this reason.
    Accept(&'static str),
    /// Discard, for this reason.
    Discard(&'static str),
    /// Discard because it repeats a rule; the reason names it.
    Repeat(String),
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

/// The concepts of a rule statement: each word folded to one key shared by
/// its inflections and its PT/EN glossary synonyms (`application::terms`), so a
/// paraphrase in the other language or with synonyms meets the original.
fn concepts(text: &str) -> BTreeSet<String> {
    words(text)
        .iter()
        .filter_map(|word| crate::terms::variants(word).into_iter().next())
        .collect()
}

/// Overlap of two concept sets, 0 to 1.
fn concept_overlap(a: &BTreeSet<String>, b: &BTreeSet<String>) -> f64 {
    let union = a.union(b).count();
    if union == 0 {
        return 0.0;
    }
    a.intersection(b).count() as f64 / union as f64
}

/// How much two rule statements say the same thing, 0 to 1, by the concepts
/// they share; compared with [`DUPLICATE_AT`] and [`SIMILAR_AT`].
pub fn rule_similarity(a: &str, b: &str) -> f64 {
    concept_overlap(&concepts(a), &concepts(b))
}

/// Sources an evidence list holds.
fn sources(candidate: &StoredCandidate) -> usize {
    serde_json::from_str::<Vec<serde_json::Value>>(&candidate.evidence_refs)
        .map(|refs| refs.len())
        .unwrap_or(0)
}

/// What the rules say about a pending candidate, given the questions already
/// recorded in its project and whether the extractor's confidence has been
/// shown to predict what this person keeps.
///
/// Without that proof (`confidence_trusted` false: fewer than 30 decisions,
/// or a confidence that does not separate kept from dismissed) a high
/// confidence is only the model's opinion, so the free rules never create a
/// norm on it: they discard repeats and send the rest to the batched judge.
pub fn triage_candidate(
    candidate: &StoredCandidate,
    recorded: &[String],
    confidence_trusted: bool,
) -> Triage {
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
    if confidence_trusted
        && candidate.confidence >= SURE_CONFIDENCE
        && candidate.significance >= MIN_SIGNIFICANCE
        && sources(candidate) >= 1
        && files <= MAX_FILES
        && likeness < SIMILAR_AT
    {
        Triage::Accept("alta confiança calibrada, com fonte e sem parecido registrado")
    } else {
        Triage::Ask
    }
}

/// Whether the extractor's confidence predicts what this person keeps, from
/// the candidates already decided (`application::calibration`).
fn confidence_predicts(decided: &[StoredCandidate]) -> bool {
    use crate::calibration::{calibrate, Outcome, Sample};
    let samples: Vec<Sample> = decided
        .iter()
        .filter_map(|row| {
            let outcome = match row.status {
                CandidateStatus::Accepted => Outcome::Accepted,
                CandidateStatus::EditedAndAccepted => Outcome::Edited,
                CandidateStatus::Dismissed => Outcome::Dismissed,
                _ => return None,
            };
            Some(Sample {
                confidence: row.confidence,
                significance: row.significance,
                outcome,
            })
        })
        .collect();
    calibrate(&samples).verdict.allows_automation()
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
/// the decision's text may be a passing remark, and one proposed by the AI
/// from that text is its own guess: the AI judges the quote of both.
pub fn triage_link(reason: &str) -> Triage {
    if mention_quote(reason).is_some() || ai_link_quote(reason).is_some() {
        Triage::Ask
    } else if reason.starts_with(crate::graph::DEPENDENCY_REASON) {
        // The text cites a dependency; one component declaring it is the place.
        if reason.contains(crate::graph::DEPENDENCY_SHARED_MARK) {
            Triage::Ask
        } else {
            Triage::Accept("dependência citada que só este componente declara")
        }
    } else if reason.starts_with(crate::graph::SYMBOL_REASON) {
        // The text cites a name or a file the code of one component holds.
        Triage::Accept("símbolo ou arquivo citado que só este componente define")
    } else {
        Triage::Accept("derivado de arquivo ou dependência que a decisão tocou")
    }
}

/// What the AI is told about a suggested tie.
fn link_text(suggestion: &Suggestion) -> String {
    let evidence = if let Some(quote) = mention_quote(&suggestion.reason) {
        format!("Citação: {}", clip(quote, SENT_CHARS))
    } else if let Some(quote) = ai_link_quote(&suggestion.reason) {
        let why = ai_link_why(&suggestion.reason).unwrap_or_default();
        format!(
            "Citação proposta pela IA: {} | Motivo da IA: {}",
            clip(quote, SENT_CHARS),
            clip(why, 160)
        )
    } else {
        format!("Arquivo ou dependência: {}", clip(&suggestion.reason, 160))
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
    /// A tie the AI itself proposed from the decision's text.
    ai_link: bool,
    /// The question of a candidate decision, to look for decisions in force
    /// that resemble it.
    near: Option<String>,
    /// The concepts of a rule or a context suggestion, to compare with the
    /// rules in force.
    rule: Option<BTreeSet<String>>,
    /// Rules in force that only resemble it, for the judge.
    similar_rules: Vec<String>,
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
An item may list rules in force that resemble it after the items: discard it when it \
only restates one of them, and keep it (accept or human) when it adds a real constraint, \
a different condition or another scope.\n\
Give in reason one short sentence in the language of the item. The ids (I1, D2, R3) are only for \
the conflicts_with field: never write an id in the reason, name the other item by its title \
instead, because the reader never sees the ids. When an item contradicts another item of the \
list or one of the decisions in force listed after the items, put that id in conflicts_with \
(only one, the most direct); otherwise leave it null. Reply with one JSON object \
only, matching exactly: {\"verdicts\":[{\"id\":string,\"verdict\":\"accept|discard|human\",\
\"reason\":string,\"conflicts_with\":string|null}]}, one entry per item, using its id exactly \
as given.",
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
                    "required": ["id", "verdict", "reason", "conflicts_with"],
                    "properties": {
                        "id": { "type": "string" },
                        "verdict": { "type": "string", "enum": ["accept", "discard", "human"] },
                        "reason": { "type": "string" },
                        "conflicts_with": { "type": ["string", "null"] }
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
    #[serde(default)]
    conflicts_with: Option<String>,
}

/// What the judge said about one item of the batch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Judged {
    /// The item's id in the prompt (`I1`).
    pub id: String,
    /// The verdict.
    pub verdict: Verdict,
    /// The reason, as written (it may still cite ids).
    pub reason: String,
    /// The id of the item or decision in force it contradicts, as written.
    pub conflicts_with: Option<String>,
    /// Whether the answer mentioned the item at all.
    pub answered: bool,
}

/// The verdicts of an answer, by the ids given in the prompt; anything the
/// answer left out or garbled is for the person.
pub fn parse_verdicts(text: &str, ids: &[String]) -> Vec<Judged> {
    let start = text.find('{').unwrap_or(0);
    let end = text.rfind('}').map_or(text.len(), |end| end + 1);
    let raw: RawVerdicts = serde_json::from_str(text.get(start..end).unwrap_or(text))
        .unwrap_or(RawVerdicts { verdicts: vec![] });
    ids.iter()
        .map(
            |id| match raw.verdicts.iter().find(|verdict| verdict.id == *id) {
                Some(verdict) => Judged {
                    id: id.clone(),
                    verdict: match verdict.verdict.as_str() {
                        "accept" => Verdict::Accepted,
                        "discard" => Verdict::Discarded,
                        _ => Verdict::NeedsHuman,
                    },
                    reason: verdict.reason.trim().chars().take(240).collect(),
                    conflicts_with: verdict
                        .conflicts_with
                        .as_deref()
                        .map(str::trim)
                        .filter(|other| !other.is_empty())
                        .map(str::to_owned),
                    answered: true,
                },
                None => Judged {
                    id: id.clone(),
                    verdict: Verdict::NeedsHuman,
                    reason: "a IA não respondeu sobre este item".to_owned(),
                    conflicts_with: None,
                    answered: false,
                },
            },
        )
        .collect()
}

/// Replaces the ids of the prompt left in a reason (`I5`, `D2`) by the title
/// of the item they stand for: the reader never saw the ids. An id that
/// stands for nothing becomes "outro item".
pub fn name_items(reason: &str, names: &[(String, String)]) -> String {
    let chars: Vec<char> = reason.chars().collect();
    let mut out = String::new();
    let mut at = 0;
    while at < chars.len() {
        let c = chars[at];
        if matches!(c, 'I' | 'D' | 'R') && (at == 0 || !chars[at - 1].is_alphanumeric()) {
            let digits = chars[at + 1..]
                .iter()
                .take_while(|digit| digit.is_ascii_digit())
                .count();
            let end = at + 1 + digits;
            if digits > 0 && chars.get(end).is_none_or(|next| !next.is_alphanumeric()) {
                let token: String = chars[at..end].iter().collect();
                match names.iter().find(|(id, _)| *id == token) {
                    Some((_, title)) => {
                        out.push('"');
                        out.push_str(&clip(title, 80));
                        out.push('"');
                    }
                    None => out.push_str("outro item"),
                }
                at = end;
                continue;
            }
        }
        out.push(c);
        at += 1;
    }
    out
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
        let decided = self.candidates(
            project_id,
            vec![
                CandidateStatus::Accepted,
                CandidateStatus::EditedAndAccepted,
                CandidateStatus::Dismissed,
            ],
            READ_LIMIT,
        )?;
        let confidence_trusted = confidence_predicts(&decided);
        let recorded: Vec<String> = decided.into_iter().map(|row| row.question).collect();
        for candidate in pending {
            if candidate.significance < MIN_SIGNIFICANCE
                || done.contains(&(ItemKind::Candidate, candidate.id.clone()))
            {
                continue;
            }
            let triage = triage_candidate(&candidate, &recorded, confidence_trusted);
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
                ai_link: false,
                near: (candidate.kind == CandidateKind::Decision.as_str())
                    .then(|| candidate.question.clone()),
                rule: (candidate.kind == CandidateKind::Rule.as_str()).then(|| {
                    // Accepting a rule stores its choice as the statement.
                    concepts(if candidate.choice.trim().is_empty() {
                        &candidate.question
                    } else {
                        &candidate.choice
                    })
                }),
                similar_rules: Vec::new(),
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
                ai_link: false,
                near: None,
                rule: None,
                similar_rules: Vec::new(),
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
                ai_link: false,
                near: None,
                rule: Some(concepts(&record.statement)),
                similar_rules: Vec::new(),
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
                ai_link: ai_link_quote(&suggestion.reason).is_some(),
                near: None,
                rule: None,
                similar_rules: Vec::new(),
            });
        }
        self.compare_rules(project_id, &mut items);
        Ok(items)
    }

    /// Compares the rule items with the rules in force (one read, ranked in
    /// memory) and with each other: a near-duplicate is settled by the rules,
    /// one that is only similar carries its closest neighbors to the judge.
    fn compare_rules(&self, project_id: &str, items: &mut [Item]) {
        if items.iter().all(|item| item.rule.is_none()) {
            return;
        }
        let now = now_rfc3339();
        let live: Vec<(String, BTreeSet<String>)> =
            ClaimStore::project_claims(&self.store, project_id)
                .unwrap_or_default()
                .into_iter()
                .filter(|claim| {
                    claim
                        .valid_until
                        .as_deref()
                        .is_none_or(|end| end > now.as_str())
                })
                .map(|claim| {
                    let concepts = concepts(&claim.statement);
                    (claim.statement, concepts)
                })
                .collect();
        let mut kept: Vec<(String, BTreeSet<String>)> = Vec::new();
        for item in items.iter_mut() {
            let Some(own) = item.rule.take() else {
                continue;
            };
            let mut scored: Vec<(f64, &str)> = live
                .iter()
                .map(|(statement, other)| (concept_overlap(&own, other), statement.as_str()))
                .collect();
            scored.sort_by(|a, b| b.0.total_cmp(&a.0));
            let best = scored.first().copied().unwrap_or((0.0, ""));
            if best.0 >= DUPLICATE_AT {
                item.triage =
                    Triage::Repeat(format!("repete a regra em vigor: {}", clip(best.1, 120)));
                continue;
            }
            if let Some((_, earlier)) = kept
                .iter()
                .map(|(title, other)| (concept_overlap(&own, other), title))
                .find(|(likeness, _)| *likeness >= DUPLICATE_AT)
            {
                item.triage = Triage::Repeat(format!(
                    "repete outra regra pendente: {}",
                    clip(earlier, 120)
                ));
                continue;
            }
            if best.0 >= SIMILAR_AT {
                item.similar_rules = scored
                    .iter()
                    .filter(|(likeness, _)| *likeness >= NEIGHBOR_AT)
                    .take(RULE_NEIGHBORS)
                    .map(|(_, statement)| (*statement).to_owned())
                    .collect();
            }
            kept.push((item.title.clone(), own));
        }
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

    /// Whether the item is still waiting (gather lists only unrecorded,
    /// pending items); a failed read counts as resolved, so nothing is recorded.
    fn is_pending(&self, project_id: &str, item: &Item) -> bool {
        self.gather(project_id).is_ok_and(|items| {
            items
                .iter()
                .any(|other| other.kind == item.kind && other.id == item.id)
        })
    }

    /// Applies a verdict and writes it to the ledger. An item a person
    /// resolved meanwhile is skipped without a record; one that is still
    /// pending but cannot be applied is recorded as left for the person.
    #[allow(clippy::too_many_arguments)]
    fn settle(
        &self,
        project_id: &str,
        item: &Item,
        verdict: Verdict,
        by: By,
        reason: &str,
        conflicts_with: Option<Conflict>,
        now: &str,
        report: &mut RunReport,
    ) -> Result<(), ReviewError> {
        let result_id = match verdict {
            Verdict::NeedsHuman => None,
            Verdict::Accepted | Verdict::Discarded => {
                match self.apply(item, verdict == Verdict::Accepted) {
                    Ok(result) => result,
                    Err(_) => {
                        // A person got there first: nothing to record.
                        if !self.is_pending(project_id, item) {
                            return Ok(());
                        }
                        // Still pending but not applicable: record it for the
                        // person, or every pass would ask the AI again.
                        let reason = if verdict == Verdict::Accepted {
                            "a IA aceitou, mas não foi possível aplicar; revise"
                        } else {
                            "a IA descartou, mas não foi possível aplicar; revise"
                        };
                        return self.settle(
                            project_id,
                            item,
                            Verdict::NeedsHuman,
                            by,
                            reason,
                            conflicts_with,
                            now,
                            report,
                        );
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
            conflicts_with: conflicts_with.filter(|_| verdict == Verdict::NeedsHuman),
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

    /// Whether a call may go now: always, unless the last one failed less
    /// than [`FAILURE_PAUSE_MINUTES`] ago.
    fn may_ask(&self, now: &str) -> Result<bool, ReviewError> {
        let day_ago = add_hours(now, -24).unwrap_or_default();
        let calls = self.store.review_calls(&day_ago)?;
        let Some((last, false)) = calls.last() else {
            return Ok(true);
        };
        let ready_at = add_minutes(last, FAILURE_PAUSE_MINUTES).unwrap_or_default();
        Ok(now >= ready_at.as_str())
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
                Triage::Repeat(reason) => (Verdict::Discarded, reason.as_str()),
                Triage::Ask => continue,
            };
            self.settle(
                project_id,
                item,
                verdict,
                By::Rules,
                reason,
                None,
                now,
                &mut report,
            )?;
        }

        // The rest, in one batched call.
        let ambiguous: Vec<&Item> = items
            .iter()
            .filter(|item| item.triage == Triage::Ask)
            .collect();
        if ambiguous.is_empty() || !self.may_ask(now)? {
            return Ok(report);
        }
        let Some(model) = self.judge() else {
            return Ok(report);
        };
        let mut oldest_first = ambiguous;
        oldest_first.sort_by(|a, b| a.waiting_since.cmp(&b.waiting_since));
        let batch: Vec<&Item> = oldest_first.into_iter().take(BATCH).collect();
        let ids: Vec<String> = (1..=batch.len()).map(|at| format!("I{at}")).collect();

        // The decisions in force that resemble a candidate decision are named
        // beside it, so the judge can tell a contradiction with what already
        // stands. Ranked in memory over one read: no query per item.
        let in_force = if batch.iter().any(|item| item.near.is_some()) {
            self.in_force(project_id)
        } else {
            Vec::new()
        };
        let mut shown: Vec<&StoredDecision> = Vec::new();
        let mut shown_rules: Vec<&str> = Vec::new();
        let mut user = String::from("## Items\n");
        for (id, item) in ids.iter().zip(&batch) {
            user.push_str(&format!("- {id} {}", item.text));
            let near = item
                .near
                .as_deref()
                .map(|question| nearest(question, &in_force))
                .unwrap_or_default();
            if !near.is_empty() {
                let named: Vec<String> = near
                    .into_iter()
                    .map(|decision| {
                        let at = shown
                            .iter()
                            .position(|seen| seen.decision_id == decision.decision_id)
                            .unwrap_or_else(|| {
                                shown.push(decision);
                                shown.len() - 1
                            });
                        format!("D{}", at + 1)
                    })
                    .collect();
                user.push_str(&format!(" | Parecidas em vigor: {}", named.join(", ")));
            }
            if !item.similar_rules.is_empty() {
                let named: Vec<String> = item
                    .similar_rules
                    .iter()
                    .map(|rule| {
                        let at = shown_rules
                            .iter()
                            .position(|seen| *seen == rule.as_str())
                            .unwrap_or_else(|| {
                                shown_rules.push(rule);
                                shown_rules.len() - 1
                            });
                        format!("R{}", at + 1)
                    })
                    .collect();
                user.push_str(&format!(
                    " | Regras parecidas em vigor: {}",
                    named.join(", ")
                ));
            }
            user.push('\n');
        }
        if !shown_rules.is_empty() {
            user.push_str("## Regras em vigor\n");
            for (at, rule) in shown_rules.iter().enumerate() {
                user.push_str(&format!("- R{} {}\n", at + 1, clip(rule, 200)));
            }
        }
        if !shown.is_empty() {
            user.push_str("## Decisões em vigor\n");
            for (at, decision) in shown.iter().enumerate() {
                user.push_str(&format!(
                    "- D{} \"{}\" -> {}\n",
                    at + 1,
                    clip(&decision.question, 160),
                    clip(&decision.choice, 160)
                ));
            }
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

        let mut names: Vec<(String, String)> = ids
            .iter()
            .cloned()
            .zip(batch.iter().map(|item| item.title.clone()))
            .collect();
        names.extend(
            shown
                .iter()
                .enumerate()
                .map(|(at, decision)| (format!("D{}", at + 1), decision.question.clone())),
        );
        names.extend(
            shown_rules
                .iter()
                .enumerate()
                .map(|(at, rule)| (format!("R{}", at + 1), (*rule).to_owned())),
        );
        let mut judged = parse_verdicts(&answer, &ids);
        let mut conflicts: Vec<Option<Conflict>> = judged
            .iter()
            .enumerate()
            .map(|(at, verdict)| {
                let other = verdict.conflicts_with.as_deref()?;
                if batch[at].kind != ItemKind::Candidate {
                    return None;
                }
                conflict_named(other, at, &batch, &shown)
            })
            .collect();
        // A contradiction runs both ways: the other candidate, when it is
        // left for the person too, shows the same pair. One that was
        // discarded no longer contradicts anything.
        for at in 0..conflicts.len() {
            let Some(Conflict {
                kind: ConflictKind::Candidate,
                id,
            }) = conflicts[at].clone()
            else {
                continue;
            };
            let Some(other) = batch.iter().position(|item| item.id == id) else {
                continue;
            };
            match judged[other].verdict {
                Verdict::Discarded => conflicts[at] = None,
                Verdict::NeedsHuman if conflicts[other].is_none() => {
                    conflicts[other] = Some(Conflict {
                        kind: ConflictKind::Candidate,
                        id: batch[at].id.clone(),
                    });
                }
                _ => {}
            }
        }
        for ((verdict, conflict), item) in judged.iter_mut().zip(conflicts).zip(&batch) {
            // Two AIs disagreeing on a weak tie is not worth a person's time:
            // a tie the AI proposed that the judge doubts is dropped, with
            // the judge's reason on the ledger.
            if item.ai_link && verdict.answered && verdict.verdict == Verdict::NeedsHuman {
                verdict.verdict = Verdict::Discarded;
            }
            let reason = name_items(&verdict.reason, &names);
            let reason = if reason.is_empty() {
                "decidido pela IA".to_owned()
            } else {
                reason
            };
            self.settle(
                project_id,
                item,
                verdict.verdict,
                By::Ai,
                &reason,
                conflict,
                now,
                &mut report,
            )?;
        }
        Ok(report)
    }

    /// The decisions in force of a project, newest first, up to the same
    /// bound as the candidates read; a failed read means none.
    fn in_force(&self, project_id: &str) -> Vec<StoredDecision> {
        DecisionStore::list(
            &self.store,
            &DecisionQuery {
                project_id: Some(project_id.to_owned()),
                statuses: vec![DecisionStatus::Accepted],
                limit: READ_LIMIT.min(MAX_PAGE_LIMIT),
                before: None,
            },
        )
        .unwrap_or_default()
    }
}

/// Decisions in force that resemble a question, the closest first, at most
/// [`NEIGHBORS`].
fn nearest<'a>(question: &str, in_force: &'a [StoredDecision]) -> Vec<&'a StoredDecision> {
    let mut scored: Vec<(f64, &StoredDecision)> = in_force
        .iter()
        .map(|decision| (similarity(question, &decision.question), decision))
        .filter(|(likeness, _)| *likeness >= NEIGHBOR_AT)
        .collect();
    scored.sort_by(|a, b| b.0.total_cmp(&a.0));
    scored
        .into_iter()
        .take(NEIGHBORS)
        .map(|(_, decision)| decision)
        .collect()
}

/// The real item an id of the prompt names, if it is a candidate of the
/// batch other than the item itself, or a decision shown in force.
fn conflict_named(
    raw: &str,
    own: usize,
    batch: &[&Item],
    shown: &[&StoredDecision],
) -> Option<Conflict> {
    let (letter, number) = raw.split_at_checked(1)?;
    let at = number.parse::<usize>().ok()?.checked_sub(1)?;
    match letter {
        "I" if at != own => {
            let other = batch.get(at)?;
            (other.kind == ItemKind::Candidate).then(|| Conflict {
                kind: ConflictKind::Candidate,
                id: other.id.clone(),
            })
        }
        "D" => Some(Conflict {
            kind: ConflictKind::Decision,
            id: shown.get(at)?.decision_id.clone(),
        }),
        _ => None,
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

    fn conflict(&self, entry: &Entry) -> Result<Option<ConflictView>, ReviewError> {
        Conflicts::new(self.store.clone(), self.adoption.clone()).view(entry)
    }

    fn resolve(
        &self,
        candidate_id: &str,
        conflict: &Conflict,
        resolution: &Resolution,
    ) -> Result<(), ReviewError> {
        Conflicts::new(self.store.clone(), self.adoption.clone()).resolve(
            candidate_id,
            conflict,
            resolution,
        )
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
                &recorded,
                true,
            ),
            Triage::Accept(_)
        ));
    }

    #[test]
    fn without_calibrated_confidence_the_rules_never_accept_a_norm() {
        let recorded = vec!["Qual fila usar para os jobs?".to_owned()];
        assert_eq!(
            triage_candidate(
                &candidate("Como versionar as decisões revisadas?"),
                &recorded,
                false,
            ),
            Triage::Ask
        );
        let repeat = vec!["Como versionar as decisões revisadas?".to_owned()];
        assert!(matches!(
            triage_candidate(
                &candidate("Como versionar as decisões revisadas?"),
                &repeat,
                false,
            ),
            Triage::Discard(_)
        ));
    }

    #[test]
    fn a_repeated_question_is_discarded_by_the_rules() {
        let recorded = vec!["Como versionar as decisões revisadas?".to_owned()];
        assert!(matches!(
            triage_candidate(
                &candidate("Como versionar as decisões revisadas?"),
                &recorded,
                true,
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
        assert_eq!(triage_candidate(&low, &none, true), Triage::Ask);
        let mut rule = base.clone();
        rule.kind = "rule".into();
        assert_eq!(triage_candidate(&rule, &none, true), Triage::Ask);
        let mut unsourced = base.clone();
        unsourced.evidence_refs = "[]".into();
        assert_eq!(triage_candidate(&unsourced, &none, true), Triage::Ask);
        let mut wide = base.clone();
        wide.diff_summary = "{\"files\":[\"a\",\"b\",\"c\",\"d\"]}".into();
        assert_eq!(triage_candidate(&wide, &none, true), Triage::Ask);
        let alike = vec!["Como versionar as decisões revisadas depois?".to_owned()];
        assert_eq!(
            triage_candidate(&base, &alike, true),
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
        let proposed = crate::graph::ai_link_reason("o core grava pela outbox", "Rege o core.");
        assert_eq!(triage_link(&proposed), Triage::Ask);
    }

    #[test]
    fn a_cited_symbol_is_accepted_by_the_rules() {
        let reason = format!(
            "{}`FLUSH_INTERVAL` (crates/store/src/flush.rs)",
            crate::graph::SYMBOL_REASON
        );
        assert!(matches!(triage_link(&reason), Triage::Accept(_)));
    }

    #[test]
    fn a_cited_dependency_is_accepted_only_when_one_component_declares_it() {
        let single = format!(
            "{}\"iroh\" (crates/net/Cargo.toml)",
            crate::graph::DEPENDENCY_REASON
        );
        assert!(matches!(triage_link(&single), Triage::Accept(_)));
        let shared = format!(
            "{}\"serde\" (crates/net/Cargo.toml{})",
            crate::graph::DEPENDENCY_REASON,
            crate::graph::DEPENDENCY_SHARED_MARK
        );
        assert_eq!(triage_link(&shared), Triage::Ask);
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
        assert_eq!(parsed[0].verdict, Verdict::Accepted);
        assert_eq!(parsed[1].verdict, Verdict::Discarded);
        assert_eq!(parsed[2].verdict, Verdict::NeedsHuman, "unanswered");
        assert!(!parsed[2].answered);
        assert!(parse_verdicts("not json", &ids)
            .iter()
            .all(|judged| judged.verdict == Verdict::NeedsHuman && !judged.answered));
    }

    #[test]
    fn conflicts_with_is_parsed_and_an_empty_one_is_none() {
        let ids = vec!["I1".to_owned(), "I2".to_owned(), "I3".to_owned()];
        let answer = r#"{"verdicts":[
          {"id":"I1","verdict":"human","reason":"contradiz","conflicts_with":" I2 "},
          {"id":"I2","verdict":"human","reason":"contradiz","conflicts_with":null},
          {"id":"I3","verdict":"human","reason":"ok","conflicts_with":""}]}"#;
        let parsed = parse_verdicts(answer, &ids);
        assert_eq!(parsed[0].conflicts_with.as_deref(), Some("I2"));
        assert_eq!(parsed[1].conflicts_with, None);
        assert_eq!(parsed[2].conflicts_with, None);
    }

    #[test]
    fn ids_left_in_a_reason_become_the_title_of_the_item() {
        let names = vec![
            ("I5".to_owned(), "Limiares fixos para o juiz".to_owned()),
            ("D2".to_owned(), "Faixas de probabilidade".to_owned()),
        ];
        assert_eq!(
            name_items("Contradiz I5 sobre limiares e D2.", &names),
            "Contradiz \"Limiares fixos para o juiz\" sobre limiares e \"Faixas de probabilidade\"."
        );
        assert_eq!(
            name_items("Ver I9, não VI5 nem I5x.", &names),
            "Ver outro item, não VI5 nem I5x."
        );
        let long = vec![("I1".to_owned(), "palavra ".repeat(30))];
        assert!(name_items("I1", &long).chars().count() <= 84);
    }

    #[test]
    fn the_conflict_an_id_names_is_a_candidate_of_the_batch_or_a_decision_in_force() {
        let item = |kind, id: &str| Item {
            kind,
            id: id.to_owned(),
            title: id.to_owned(),
            text: String::new(),
            waiting_since: String::new(),
            triage: Triage::Ask,
            ai_link: false,
            near: None,
            rule: None,
            similar_rules: Vec::new(),
        };
        let (a, b, link) = (
            item(ItemKind::Candidate, "ca"),
            item(ItemKind::Candidate, "cb"),
            item(ItemKind::Link, "ek"),
        );
        let batch = vec![&a, &b, &link];
        let named = conflict_named("I2", 0, &batch, &[]).expect("candidate");
        assert_eq!(
            (named.kind, named.id.as_str()),
            (ConflictKind::Candidate, "cb")
        );
        assert_eq!(conflict_named("I1", 0, &batch, &[]), None, "itself");
        assert_eq!(conflict_named("I3", 0, &batch, &[]), None, "a link");
        assert_eq!(conflict_named("I7", 0, &batch, &[]), None, "out of range");
        assert_eq!(conflict_named("D1", 0, &batch, &[]), None, "nothing shown");
        assert_eq!(conflict_named("x", 0, &batch, &[]), None);
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
