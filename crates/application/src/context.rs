//! Context Packs: a small, cited selection of what holds for a task at a date.

use std::collections::{BTreeMap, BTreeSet};

use domain::claims::ClaimKind;
use domain::relations::RelationKind;
use domain::time::Timestamp;
use serde::{Deserialize, Serialize};

use crate::claims::{ClaimRecord, ClaimStore, ClaimsError};
use crate::clock::now_rfc3339;
use crate::decisions::{DecisionStore, DecisionsError, StoredDecision};
use crate::projects::ProjectRepository;
use crate::relations::{RelationRow, RelationStore};

/// Default size budget, in characters of selected text.
pub const DEFAULT_BUDGET_CHARS: usize = 8_000;

/// Smallest accepted budget.
pub const MIN_BUDGET_CHARS: usize = 500;

/// Largest accepted budget.
pub const MAX_BUDGET_CHARS: usize = 50_000;

/// Longest accepted task description.
pub const MAX_TASK_CHARS: usize = 2_000;

/// Ranked candidates fetched per source before selection.
pub const MAX_RANKED: usize = 50;

/// Words ignored when matching a task (Portuguese and English).
const STOPWORDS: &[&str] = &[
    "a", "o", "as", "os", "de", "da", "do", "das", "dos", "e", "em", "no", "na", "nos", "nas",
    "um", "uma", "para", "por", "com", "sem", "que", "se", "ao", "the", "an", "and", "or", "of",
    "to", "in", "on", "for", "with", "is", "it", "not", "near",
];

/// Failure modes of pack building.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContextError {
    /// The storage backend failed; the message is diagnostic only.
    Storage(String),
    /// The project does not exist.
    ProjectNotFound,
    /// The request is malformed.
    InvalidRequest(String),
}

impl ContextError {
    /// Short, stable code.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Storage(_) => "storage",
            Self::ProjectNotFound => "project_not_found",
            Self::InvalidRequest(_) => "invalid_request",
        }
    }
}

impl std::fmt::Display for ContextError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Storage(message) => write!(formatter, "falha de armazenamento: {message}"),
            Self::ProjectNotFound => formatter.write_str("projeto não encontrado"),
            Self::InvalidRequest(message) => write!(formatter, "pedido inválido: {message}"),
        }
    }
}

impl std::error::Error for ContextError {}

impl From<DecisionsError> for ContextError {
    fn from(error: DecisionsError) -> Self {
        Self::Storage(error.to_string())
    }
}

impl From<ClaimsError> for ContextError {
    fn from(error: ClaimsError) -> Self {
        Self::Storage(error.to_string())
    }
}

/// What the caller wants context for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextRequest {
    /// Project to read from.
    pub project_id: String,
    /// Task or question, in free text.
    pub task: String,
    /// Reference date; now when `None`.
    pub as_of: Option<String>,
    /// Size budget in characters; [`DEFAULT_BUDGET_CHARS`] when `None`.
    pub budget_chars: Option<usize>,
}

/// One decision in a pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackDecision {
    /// Decision identifier.
    pub decision_id: String,
    /// Version cited.
    pub version: i64,
    /// Question answered.
    pub question: String,
    /// Choice made.
    pub choice: String,
    /// Why.
    pub rationale: String,
    /// RFC 3339 confirmation time.
    pub confirmed_at: String,
    /// Evidence artifact ids, in stored order.
    pub evidence: Vec<String>,
    /// Decisions this one depends on.
    pub depends_on: Vec<String>,
    /// Decisions this one conflicts with.
    pub conflicts_with: Vec<String>,
}

/// One claim in a pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackClaim {
    /// Claim identifier.
    pub claim_id: String,
    /// Claim kind literal.
    pub kind: String,
    /// Statement.
    pub statement: String,
    /// Start of validity.
    pub valid_from: String,
    /// End of validity, when set.
    pub valid_until: Option<String>,
    /// Decision the claim came from, when any.
    pub source_decision_id: Option<String>,
    /// Whether the claim matched the task (otherwise it is a standing rule).
    pub matched: bool,
}

/// A temporary, cited selection for one task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextPack {
    /// Project read.
    pub project_id: String,
    /// Task as asked.
    pub task: String,
    /// Reference date used.
    pub as_of: String,
    /// Budget requested.
    pub budget_chars: usize,
    /// Characters of selected text.
    pub used_chars: usize,
    /// Relevant decisions in force at `as_of`, most relevant first.
    pub decisions: Vec<PackDecision>,
    /// Claims valid at `as_of`: matched first, then standing constraints and conventions.
    pub claims: Vec<PackClaim>,
    /// Relevant items left out by the budget.
    pub omitted: usize,
}

/// FTS ranking the pack needs.
pub trait ContextStore {
    /// Decision ids matching `match_query` in a project, best first.
    fn rank_decisions(
        &self,
        project_id: &str,
        match_query: &str,
        limit: usize,
    ) -> Result<Vec<String>, ContextError>;

    /// Claim ids matching `match_query` in a project, best first.
    fn rank_claims(
        &self,
        project_id: &str,
        match_query: &str,
        limit: usize,
    ) -> Result<Vec<String>, ContextError>;
}

/// Builds Context Packs.
pub trait ContextProvider {
    /// Selects what holds for the request, within its budget.
    fn build_pack(&self, request: ContextRequest) -> Result<ContextPack, ContextError>;
}

/// [`ContextProvider`] over the local store.
#[derive(Debug, Clone)]
pub struct ContextPacks<S> {
    store: S,
}

impl<S> ContextPacks<S> {
    /// Wraps the store.
    pub fn new(store: S) -> Self {
        Self { store }
    }
}

impl<S> ContextProvider for ContextPacks<S>
where
    S: ContextStore + DecisionStore + RelationStore + ClaimStore + ProjectRepository,
{
    fn build_pack(&self, request: ContextRequest) -> Result<ContextPack, ContextError> {
        let task = request.task.trim().to_string();
        if task.is_empty() || task.chars().count() > MAX_TASK_CHARS {
            return Err(ContextError::InvalidRequest(
                "descreva a tarefa em até 2.000 caracteres".into(),
            ));
        }
        let budget = request.budget_chars.unwrap_or(DEFAULT_BUDGET_CHARS);
        if !(MIN_BUDGET_CHARS..=MAX_BUDGET_CHARS).contains(&budget) {
            return Err(ContextError::InvalidRequest(
                "o orçamento deve ficar entre 500 e 50.000 caracteres".into(),
            ));
        }
        let now = now_rfc3339();
        let as_of =
            Timestamp::parse(request.as_of.as_deref().unwrap_or(&now)).ok_or_else(|| {
                ContextError::InvalidRequest("use a data no formato AAAA-MM-DD".into())
            })?;
        ProjectRepository::get(&self.store, &request.project_id)
            .map_err(|error| ContextError::Storage(error.to_string()))?
            .ok_or(ContextError::ProjectNotFound)?;

        let relations = self.store.project_relations(&request.project_id)?;
        let query = match_any_query(&task);
        let ranked_decisions = match &query {
            Some(query) => self
                .store
                .rank_decisions(&request.project_id, query, MAX_RANKED)?,
            None => Vec::new(),
        };
        let ranked_claims = match &query {
            Some(query) => self
                .store
                .rank_claims(&request.project_id, query, MAX_RANKED)?,
            None => Vec::new(),
        };

        let mut selection = Selection::new(budget);
        let claims = self.store.project_claims(&request.project_id)?;
        let valid: BTreeMap<&str, &ClaimRecord> = claims
            .iter()
            .filter(|claim| claim.is_valid_at(&as_of))
            .map(|claim| (claim.claim_id.as_str(), claim))
            .collect();
        let matched_claims: BTreeSet<&str> = ranked_claims
            .iter()
            .map(String::as_str)
            .filter(|id| valid.contains_key(id))
            .collect();

        let mut decisions = Vec::new();
        for id in &ranked_decisions {
            let Some(decision) = DecisionStore::get(&self.store, id)? else {
                continue;
            };
            if !in_force(&decision, &relations, &as_of) {
                continue;
            }
            let item = self.pack_decision(decision, &relations, &as_of)?;
            if selection.take(decision_cost(&item)) {
                decisions.push(item);
            }
        }

        let mut pack_claims = Vec::new();
        let standing = valid.values().filter(|claim| {
            !matched_claims.contains(claim.claim_id.as_str())
                && matches!(claim.kind, ClaimKind::Constraint | ClaimKind::Convention)
        });
        let ordered = ranked_claims
            .iter()
            .filter_map(|id| valid.get(id.as_str()).copied())
            .map(|claim| (claim, true))
            .chain(standing.map(|claim| (*claim, false)));
        for (claim, matched) in ordered {
            let item = pack_claim(claim, matched);
            if selection.take(item.statement.chars().count()) {
                pack_claims.push(item);
            }
        }

        Ok(ContextPack {
            project_id: request.project_id,
            task,
            as_of: as_of.to_string(),
            budget_chars: budget,
            used_chars: selection.used,
            decisions,
            claims: pack_claims,
            omitted: selection.omitted,
        })
    }
}

impl<S> ContextPacks<S>
where
    S: DecisionStore,
{
    fn pack_decision(
        &self,
        decision: StoredDecision,
        relations: &[RelationRow],
        as_of: &Timestamp,
    ) -> Result<PackDecision, ContextError> {
        let evidence = self
            .store
            .evidence(&decision.decision_id)?
            .into_iter()
            .map(|link| link.artifact_id)
            .collect();
        let related = |kind: RelationKind| -> Vec<String> {
            relations
                .iter()
                .filter(|row| row.kind == kind.as_str() && created_by(row, as_of))
                .filter_map(|row| {
                    if row.from == decision.decision_id {
                        Some(row.to.clone())
                    } else if kind.is_symmetric() && row.to == decision.decision_id {
                        Some(row.from.clone())
                    } else {
                        None
                    }
                })
                .collect()
        };
        Ok(PackDecision {
            depends_on: related(RelationKind::DependsOn),
            conflicts_with: related(RelationKind::ConflictsWith),
            decision_id: decision.decision_id,
            version: decision.version,
            question: decision.question,
            choice: decision.choice,
            rationale: decision.rationale,
            confirmed_at: decision.confirmed_at,
            evidence,
        })
    }
}

/// Tracks the character budget.
struct Selection {
    budget: usize,
    used: usize,
    omitted: usize,
}

impl Selection {
    fn new(budget: usize) -> Self {
        Self {
            budget,
            used: 0,
            omitted: 0,
        }
    }

    fn take(&mut self, cost: usize) -> bool {
        if self.used + cost > self.budget {
            self.omitted += 1;
            return false;
        }
        self.used += cost;
        true
    }
}

/// Whether the decision was confirmed by `as_of` and not superseded by then.
fn in_force(decision: &StoredDecision, relations: &[RelationRow], as_of: &Timestamp) -> bool {
    let confirmed = Timestamp::parse(&decision.confirmed_at).is_some_and(|at| at <= *as_of);
    let superseded = relations.iter().any(|row| {
        row.kind == RelationKind::Supersedes.as_str()
            && row.to == decision.decision_id
            && created_by(row, as_of)
    });
    confirmed && !superseded
}

fn created_by(row: &RelationRow, as_of: &Timestamp) -> bool {
    Timestamp::parse(&row.created_at).is_some_and(|at| at <= *as_of)
}

fn decision_cost(decision: &PackDecision) -> usize {
    [&decision.question, &decision.choice, &decision.rationale]
        .iter()
        .map(|text| text.chars().count())
        .sum()
}

fn pack_claim(claim: &ClaimRecord, matched: bool) -> PackClaim {
    PackClaim {
        claim_id: claim.claim_id.clone(),
        kind: claim.kind.as_str().to_string(),
        statement: claim.statement.clone(),
        valid_from: claim.valid_from.clone(),
        valid_until: claim.valid_until.clone(),
        source_decision_id: claim.source_decision_id.clone(),
        matched,
    }
}

/// FTS5 `MATCH` for any meaningful word of `task`, or `None` when none remain.
pub fn match_any_query(task: &str) -> Option<String> {
    let mut seen = BTreeSet::new();
    let terms: Vec<String> = task
        .split(|character: char| !(character.is_alphanumeric() || character == '_'))
        .map(str::to_lowercase)
        .filter(|term| term.chars().count() >= 2 && !STOPWORDS.contains(&term.as_str()))
        .filter(|term| seen.insert(term.clone()))
        .map(|term| format!("\"{term}\""))
        .collect();
    (!terms.is_empty()).then(|| terms.join(" OR "))
}

#[cfg(test)]
mod tests {
    use super::match_any_query;

    #[test]
    fn match_query_keeps_meaningful_words_once() {
        assert_eq!(
            match_any_query("Adicionar cache na API; cache de respostas!").as_deref(),
            Some("\"adicionar\" OR \"cache\" OR \"api\" OR \"respostas\"")
        );
        assert_eq!(match_any_query("de a o"), None);
        assert_eq!(
            match_any_query("OR NEAR \"x\" config").as_deref(),
            Some("\"config\"")
        );
    }
}
