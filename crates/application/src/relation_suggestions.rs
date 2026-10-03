//! Suggested relations between decisions (depends on, conflicts with,
//! supersedes), proposed after an adoption and confirmed by a person.
//!
//! Similarity is only how candidates are found: decisions that share a
//! component or technology on the map, or words. A model then judges each
//! pair and must quote, verbatim, the sentence that shows the relation; the
//! app checks the quote against the decisions' own text, the ids, cycles and
//! state before anything is stored. A suggestion becomes a relation only when
//! someone confirms it (ADR-0005, revision of 2026-10-01).

use std::collections::{BTreeMap, BTreeSet};

use domain::relations::{DecisionRelation, RelationKind};
use serde::Deserialize;
use serde_json::json;

use crate::analysis::ExtractorFactory;
use crate::clock::now_rfc3339;
use crate::context::match_any_query;
use crate::decisions::{DecisionStatus, DecisionStore, DecisionsError, StoredDecision};
use crate::graph::GraphStore;
use crate::injection::short_ref;
use crate::overview::StructuredModel;
use crate::profile::{choose_extractor, AiSettings, ExtractorChoice, ProfileStore, SecretStore};
use crate::relations::{DecisionRelations, RelationStore};

/// Job kind that looks for relations of one adopted decision.
pub const RELATION_JOB_KIND: &str = crate::jobs::JobKind::SuggestRelations.as_str();
/// Earlier decisions compared with a new one, at most.
pub const MAX_CANDIDATES: usize = 6;
/// Shortest quote accepted as evidence.
const MIN_QUOTE_CHARS: usize = 12;

/// A stored suggestion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationSuggestionRecord {
    /// Identifier.
    pub suggestion_id: String,
    /// Project of both decisions.
    pub project_id: String,
    /// Source decision.
    pub from_id: String,
    /// Target decision.
    pub to_id: String,
    /// Relation kind.
    pub kind: RelationKind,
    /// Verbatim sentence that shows the relation.
    pub quote: String,
    /// One-sentence explanation.
    pub reason: String,
    /// RFC 3339 creation time.
    pub created_at: String,
}

/// A pending suggestion with both questions, for the Mapa.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationSuggestionView {
    /// The stored suggestion.
    pub record: RelationSuggestionRecord,
    /// Question of the source decision.
    pub from_question: String,
    /// Question of the target decision.
    pub to_question: String,
}

/// Persistence of suggestions.
pub trait RelationSuggestionStore {
    /// Stores a suggestion; `false` when the same `(from, to, kind)` was ever
    /// suggested (pending, confirmed or rejected).
    fn insert_relation_suggestion(
        &self,
        record: &RelationSuggestionRecord,
    ) -> Result<bool, DecisionsError>;

    /// Pending suggestions of a project, oldest first.
    fn pending_relation_suggestions(
        &self,
        project_id: &str,
    ) -> Result<Vec<RelationSuggestionRecord>, DecisionsError>;

    /// One suggestion while it is pending.
    fn pending_relation_suggestion(
        &self,
        suggestion_id: &str,
    ) -> Result<Option<RelationSuggestionRecord>, DecisionsError>;

    /// Marks a pending suggestion `confirmed` or `rejected`; the record when
    /// it was still pending.
    fn resolve_relation_suggestion(
        &self,
        suggestion_id: &str,
        outcome: &str,
        at: &str,
    ) -> Result<Option<RelationSuggestionRecord>, DecisionsError>;
}

/// Reading, confirming and rejecting suggestions.
#[derive(Debug, Clone)]
pub struct RelationSuggestions<S> {
    store: S,
}

impl<S> RelationSuggestions<S>
where
    S: RelationSuggestionStore + DecisionStore + RelationStore + Clone,
{
    /// Wraps the store.
    pub fn new(store: S) -> Self {
        Self { store }
    }

    /// Pending suggestions whose decisions are both still accepted.
    ///
    /// # Errors
    ///
    /// `storage`.
    pub fn pending(&self, project_id: &str) -> Result<Vec<RelationSuggestionView>, DecisionsError> {
        let mut views = Vec::new();
        for record in self.store.pending_relation_suggestions(project_id)? {
            let (Some(from), Some(to)) = (
                accepted(&self.store, &record.from_id)?,
                accepted(&self.store, &record.to_id)?,
            ) else {
                continue;
            };
            views.push(RelationSuggestionView {
                from_question: from.question,
                to_question: to.question,
                record,
            });
        }
        Ok(views)
    }

    /// Records the relation, then marks the suggestion confirmed.
    ///
    /// # Errors
    ///
    /// `not_found` when it is no longer pending; the relation's own errors
    /// (`invalid_relation`, `conflict`) otherwise, leaving it pending.
    pub fn confirm(&self, suggestion_id: &str) -> Result<(), DecisionsError> {
        let record = self
            .store
            .pending_relation_suggestion(suggestion_id)?
            .ok_or(DecisionsError::NotFound)?;
        DecisionRelations::new(self.store.clone()).relate(
            &record.from_id,
            &record.to_id,
            record.kind,
        )?;
        self.store
            .resolve_relation_suggestion(suggestion_id, "confirmed", &now_rfc3339())?;
        Ok(())
    }

    /// Marks the suggestion rejected; it is never suggested again.
    ///
    /// # Errors
    ///
    /// `not_found` when it is no longer pending, `storage`.
    pub fn reject(&self, suggestion_id: &str) -> Result<(), DecisionsError> {
        self.store
            .resolve_relation_suggestion(suggestion_id, "rejected", &now_rfc3339())?
            .ok_or(DecisionsError::NotFound)
            .map(|_| ())
    }
}

fn accepted<S: DecisionStore>(
    store: &S,
    decision_id: &str,
) -> Result<Option<StoredDecision>, DecisionsError> {
    Ok(store
        .get(decision_id)?
        .filter(|decision| decision.status == DecisionStatus::Accepted))
}

/// Failures of a relation search, for the job log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelationFindError {
    /// Storage failed.
    Storage(String),
    /// The provider failed or answered out of contract.
    Provider(String),
}

impl std::fmt::Display for RelationFindError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Storage(detail) => write!(formatter, "storage: {detail}"),
            Self::Provider(detail) => write!(formatter, "provider: {detail}"),
        }
    }
}

impl std::error::Error for RelationFindError {}

/// System prompt of the relation judge.
pub const RELATION_PROMPT: &str = concat!(
    "You compare one new engineering decision of a software \
project with a few earlier decisions of the same project, and say for each whether a real \
relation exists. depends_on: one decision only makes sense because of the other (it builds \
on it, assumes it). conflicts_with: both cannot hold at the same time. supersedes: the new \
decision answers the same question as the earlier one with a different choice, replacing \
it. none: anything else. Most pairs are none; sharing a topic, a component or words is not a \
relation.\n\
For every relation other than none, copy in quote one sentence, verbatim, from the texts \
given that shows it, and explain it in one sentence in reason, in the language of the \
decisions. direction is new_to_earlier when the new decision is the source (the new one \
depends on, conflicts with or supersedes the earlier one) and earlier_to_new otherwise; \
supersedes is always new_to_earlier.\n\
Reply with one JSON object only, matching exactly: {\"relations\":[{\"earlier\":string,\
\"relation\":\"depends_on|conflicts_with|supersedes|none\",\"direction\":\
\"new_to_earlier|earlier_to_new\",\"quote\":string,\"reason\":string}]}, one entry per \
earlier decision, using its id exactly as given (D:xxxxxxxx).",
    crate::plain_rules!()
);

/// Strict schema of the judge's answer.
pub fn relation_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["relations"],
        "properties": {
            "relations": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["earlier", "relation", "direction", "quote", "reason"],
                    "properties": {
                        "earlier": { "type": "string" },
                        "relation": {
                            "type": "string",
                            "enum": ["depends_on", "conflicts_with", "supersedes", "none"]
                        },
                        "direction": {
                            "type": "string",
                            "enum": ["new_to_earlier", "earlier_to_new"]
                        },
                        "quote": { "type": "string" },
                        "reason": { "type": "string" }
                    }
                }
            }
        }
    })
}

#[derive(Deserialize)]
struct RawAnswer {
    #[serde(default)]
    relations: Vec<RawRelation>,
}

#[derive(Deserialize)]
struct RawRelation {
    #[serde(default)]
    earlier: String,
    #[serde(default)]
    relation: String,
    #[serde(default)]
    direction: String,
    #[serde(default)]
    quote: String,
    #[serde(default)]
    reason: String,
}

/// Lowercase words only, for comparing a quote with the source text.
fn normalized(text: &str) -> String {
    text.split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect::<Vec<_>>()
        .join(" ")
}

fn decision_text(decision: &StoredDecision) -> String {
    format!(
        "{}\n{}\n{}",
        decision.question, decision.choice, decision.rationale
    )
}

/// Validated relations from the judge's answer: known ids, a real kind, a
/// quote found in the texts of the pair, a direction the kind allows.
pub fn parse_relations(
    answer: &str,
    new: &StoredDecision,
    earlier: &[StoredDecision],
) -> Vec<(String, String, RelationKind, String, String)> {
    let Ok(raw) = serde_json::from_str::<RawAnswer>(answer.trim()) else {
        return Vec::new();
    };
    let by_ref: BTreeMap<String, &StoredDecision> = earlier
        .iter()
        .map(|decision| (format!("d:{}", short_ref(&decision.decision_id)), decision))
        .collect();
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for relation in raw.relations {
        let kind = match relation.relation.as_str() {
            "depends_on" => RelationKind::DependsOn,
            "conflicts_with" => RelationKind::ConflictsWith,
            "supersedes" => RelationKind::Supersedes,
            _ => continue,
        };
        let Some(other) = by_ref.get(&relation.earlier.trim().to_lowercase()) else {
            continue;
        };
        let quote = relation.quote.trim();
        let needle = normalized(quote);
        if needle.chars().count() < MIN_QUOTE_CHARS {
            continue;
        }
        let found = normalized(&decision_text(new)).contains(&needle)
            || normalized(&decision_text(other)).contains(&needle);
        if !found {
            continue;
        }
        let new_first = relation.direction != "earlier_to_new";
        if kind == RelationKind::Supersedes && !new_first {
            continue;
        }
        let (from, to) = if new_first {
            (new.decision_id.clone(), other.decision_id.clone())
        } else {
            (other.decision_id.clone(), new.decision_id.clone())
        };
        if seen.insert((from.clone(), to.clone(), kind)) {
            out.push((
                from,
                to,
                kind,
                quote.to_string(),
                relation.reason.trim().to_string(),
            ));
        }
    }
    out
}

/// The job: earlier decisions related to a new one, judged by the model.
#[derive(Debug, Clone)]
pub struct RelationFinder<S, P, K, F> {
    store: S,
    settings: AiSettings<P, K>,
    factory: F,
}

impl<S, P, K, F> RelationFinder<S, P, K, F>
where
    S: DecisionStore + RelationStore + RelationSuggestionStore + GraphStore + Clone,
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

    /// Earlier accepted decisions worth comparing with `new`: those tied to
    /// the same entities on the map first, then by shared words, without
    /// the ones already related.
    ///
    /// # Errors
    ///
    /// `storage`.
    pub fn candidates(
        &self,
        new: &StoredDecision,
    ) -> Result<Vec<StoredDecision>, RelationFindError> {
        let storage = |error: DecisionsError| RelationFindError::Storage(error.to_string());
        let related: BTreeSet<String> = self
            .store
            .decision_relations(&new.decision_id)
            .map_err(storage)?
            .into_iter()
            .flat_map(|row| [row.from, row.to])
            .collect();
        let mut ids: Vec<String> = Vec::new();
        let edges = self
            .store
            .project_edges(&new.project_id)
            .map_err(|error| RelationFindError::Storage(error.to_string()))?;
        let mine: BTreeSet<&str> = edges
            .iter()
            .filter(|edge| edge.source_id == new.decision_id && edge.invalidated_at.is_none())
            .map(|edge| edge.entity_id.as_str())
            .collect();
        for edge in &edges {
            if edge.invalidated_at.is_none()
                && mine.contains(edge.entity_id.as_str())
                && edge.source_id != new.decision_id
                && !ids.contains(&edge.source_id)
            {
                ids.push(edge.source_id.clone());
            }
        }
        if let Some(query) = match_any_query(&format!("{} {}", new.question, new.choice)) {
            for row in self
                .store
                .search(&query, Some(&new.project_id), MAX_CANDIDATES * 2)
                .map_err(storage)?
            {
                if !ids.contains(&row.decision_id) {
                    ids.push(row.decision_id);
                }
            }
        }
        let mut out = Vec::new();
        for id in ids {
            if out.len() >= MAX_CANDIDATES {
                break;
            }
            if id == new.decision_id || related.contains(&id) {
                continue;
            }
            if let Some(decision) = accepted(&self.store, &id).map_err(storage)? {
                if decision.project_id == new.project_id {
                    out.push(decision);
                }
            }
        }
        Ok(out)
    }

    /// Asks the model about the new decision and stores what survives the
    /// checks. Returns how many suggestions were stored; 0 without an
    /// enabled provider or candidates.
    ///
    /// # Errors
    ///
    /// `storage`, or `provider` when the call fails.
    pub fn run(&self, decision_id: &str) -> Result<usize, RelationFindError> {
        let storage = |error: DecisionsError| RelationFindError::Storage(error.to_string());
        let Some(new) = accepted(&self.store, decision_id).map_err(storage)? else {
            return Ok(0);
        };
        let earlier = self.candidates(&new)?;
        if earlier.is_empty() {
            return Ok(0);
        }
        let profile = self
            .settings
            .load_or_seed()
            .map_err(|error| RelationFindError::Storage(error.to_string()))?;
        if choose_extractor(Some(&profile)) != ExtractorChoice::ExternalEnabled {
            return Ok(0);
        }
        let secret = match self.settings.secret(&profile.credential_account()) {
            Ok(Some(secret)) => secret,
            Ok(None) if !profile.credential_required() => String::new(),
            _ => return Ok(0),
        };
        let Ok(model) = self.factory.external(&profile, secret) else {
            return Ok(0);
        };
        let mut user = format!(
            "## New decision D:{}\nQuestion: {}\nChoice: {}\nWhy: {}\n\n## Earlier decisions\n",
            short_ref(&new.decision_id),
            new.question,
            new.choice,
            new.rationale.chars().take(600).collect::<String>()
        );
        for decision in &earlier {
            user.push_str(&format!(
                "- D:{} Question: {} Choice: {} Why: {}\n",
                short_ref(&decision.decision_id),
                decision.question,
                decision.choice,
                decision.rationale.chars().take(400).collect::<String>()
            ));
        }
        let answer = model
            .complete(
                RELATION_PROMPT,
                &user,
                "decision_relations",
                &relation_schema(),
            )
            .map_err(|error| RelationFindError::Provider(error.to_string()))?;
        let existing: Vec<DecisionRelation> = self
            .store
            .project_relations(&new.project_id)
            .map_err(storage)?
            .iter()
            .filter_map(|row| {
                DecisionRelation::new(
                    row.from.clone(),
                    row.to.clone(),
                    RelationKind::parse(&row.kind)?,
                )
                .ok()
            })
            .collect();
        let now = now_rfc3339();
        let mut stored = 0;
        for (from, to, kind, quote, reason) in parse_relations(&answer, &new, &earlier) {
            let Ok(relation) = DecisionRelation::new(from.clone(), to.clone(), kind) else {
                continue;
            };
            if relation.check_against(&existing).is_err() {
                continue;
            }
            let inserted = self
                .store
                .insert_relation_suggestion(&RelationSuggestionRecord {
                    suggestion_id: uuid::Uuid::now_v7().to_string(),
                    project_id: new.project_id.clone(),
                    from_id: relation.from().to_string(),
                    to_id: relation.to().to_string(),
                    kind,
                    quote,
                    reason,
                    created_at: now.clone(),
                })
                .map_err(storage)?;
            if inserted {
                stored += 1;
            }
        }
        Ok(stored)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decisions::DecisionStatus;

    fn decision(id: &str, question: &str, choice: &str, rationale: &str) -> StoredDecision {
        StoredDecision {
            decision_id: id.into(),
            candidate_id: format!("c-{id}"),
            project_id: "p".into(),
            project_location: "C:/p".into(),
            capture_id: None,
            status: DecisionStatus::Accepted,
            question: question.into(),
            choice: choice.into(),
            rationale: rationale.into(),
            assumptions: "[]".into(),
            reconsider_when: "[]".into(),
            scope: "[]".into(),
            consequences: "[]".into(),
            version: 1,
            created_at: "2026-01-01T00:00:00Z".into(),
            confirmed_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        }
    }

    #[test]
    fn only_quoted_typed_relations_survive() {
        let new = decision(
            "new-0000aaaa",
            "Onde guardar a fila?",
            "Na mesma base SQLite das decisões.",
            "Reaproveita a transação que já grava as decisões em SQLite.",
        );
        let old = decision(
            "old-0000bbbb",
            "Qual banco usar?",
            "SQLite embutido.",
            "Um arquivo local, sem servidor.",
        );
        let other = decision("oth-0000cccc", "Qual tema?", "Escuro.", "Conforto.");
        let earlier = vec![old.clone(), other.clone()];
        let answer = r#"{"relations":[
            {"earlier":"D:0000bbbb","relation":"depends_on","direction":"new_to_earlier",
             "quote":"Reaproveita a transação que já grava as decisões em SQLite",
             "reason":"A fila assume o banco escolhido."},
            {"earlier":"D:0000cccc","relation":"conflicts_with","direction":"new_to_earlier",
             "quote":"isto não está em nenhum texto das decisões",
             "reason":"inventado"},
            {"earlier":"D:0000cccc","relation":"none","direction":"new_to_earlier",
             "quote":"","reason":""},
            {"earlier":"D:0000bbbb","relation":"supersedes","direction":"earlier_to_new",
             "quote":"Um arquivo local, sem servidor","reason":"direção errada"},
            {"earlier":"D:99999999","relation":"depends_on","direction":"new_to_earlier",
             "quote":"Um arquivo local, sem servidor","reason":"id desconhecido"}
        ]}"#;
        let relations = parse_relations(answer, &new, &earlier);
        assert_eq!(relations.len(), 1, "{relations:?}");
        let (from, to, kind, _, reason) = &relations[0];
        assert_eq!(
            (from.as_str(), to.as_str(), *kind),
            ("new-0000aaaa", "old-0000bbbb", RelationKind::DependsOn)
        );
        assert_eq!(reason, "A fila assume o banco escolhido.");
        assert!(parse_relations("not json", &new, &earlier).is_empty());
    }

    #[test]
    fn a_quote_matches_regardless_of_case_and_punctuation() {
        let new = decision("n-0000aaaa", "Q?", "Usar Redis para cache.", "");
        let old = decision("o-0000bbbb", "Cache?", "Em memória.", "");
        let answer = r#"{"relations":[{"earlier":"D:0000bbbb","relation":"supersedes",
            "direction":"new_to_earlier","quote":"usar redis, para CACHE","reason":"r"}]}"#;
        assert_eq!(parse_relations(answer, &new, &[old]).len(), 1);
    }
}
