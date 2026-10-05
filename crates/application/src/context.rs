//! Context Packs: a small, cited selection of what holds for a task at a date.

use std::collections::{BTreeMap, BTreeSet};

use domain::claims::ClaimKind;
use domain::relations::RelationKind;
use domain::time::Timestamp;
use serde::{Deserialize, Serialize};

use crate::claims::{ClaimRecord, ClaimStore, ClaimsError};
use crate::clock::now_rfc3339;
use crate::decisions::{DecisionStore, DecisionsError, StoredDecision};
use crate::graph::{GraphError, GraphStore, KnowledgeGraph};
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
    /// Files the task touches, relative to the project: what the project map
    /// ties to their components comes first (ADR-0005). With files, the task
    /// text may be empty.
    pub files: Vec<String>,
}

/// Most files one request may name.
pub const MAX_FILES: usize = 20;

/// One decision in a pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackDecision {
    /// Explicit restrictions; absence means not informed.
    #[serde(default)]
    pub qualifiers: Vec<crate::qualifiers::KnowledgeQualifier>,
    /// Declared scope, never implicitly the whole project.
    #[serde(default)]
    pub scope: Vec<String>,
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
    /// Source revision actually used, never inferred for legacy claims.
    #[serde(default)]
    pub source_version: Option<i64>,
    /// Scope inherited independently from qualifiers.
    #[serde(default)]
    pub inherited_scope: Vec<String>,
    /// Explicit inherited restrictions.
    #[serde(default)]
    pub qualifiers: Vec<crate::qualifiers::KnowledgeQualifier>,
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
    /// Descriptive O-citations, independent of normative claims and budgets.
    #[serde(default)]
    pub observations: Vec<crate::observations::PackObservation>,
    /// Explicit bounded source coverage; never implies historical validity.
    #[serde(default)]
    pub observation_coverage: crate::observations::ObservationCoverage,
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
    /// A pending refresh makes previously checked descriptive facts ineligible.
    fn observations_dirty(&self, _project_id: &str) -> Result<bool, ContextError> {
        Ok(false)
    }
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
    routing: Option<std::sync::Arc<crate::context_routing::RoutingLookup>>,
}

impl<S> ContextPacks<S> {
    /// Wraps the store.
    pub fn new(store: S) -> Self {
        Self {
            store,
            routing: None,
        }
    }

    /// Enables optional local cache routing; the default remains deterministic.
    pub fn with_routing(
        mut self,
        routing: Option<std::sync::Arc<crate::context_routing::RoutingLookup>>,
    ) -> Self {
        self.routing = routing;
        self
    }
}

impl<S> ContextProvider for ContextPacks<S>
where
    S: ContextStore
        + DecisionStore
        + RelationStore
        + ClaimStore
        + ProjectRepository
        + GraphStore
        + crate::observations::ObservationStore
        + Clone,
{
    fn build_pack(&self, request: ContextRequest) -> Result<ContextPack, ContextError> {
        let task = request.task.trim().to_string();
        let files: Vec<String> = request.files.iter().take(MAX_FILES).cloned().collect();
        if (task.is_empty() && files.is_empty()) || task.chars().count() > MAX_TASK_CHARS {
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
        let (by_file_decisions, by_file_claims) = KnowledgeGraph::new(self.store.clone())
            .context_for_files(&request.project_id, &files, Some(as_of.as_str()))
            .map_err(graph_error)?;
        let query = match_any_query(&task);
        let lexical_decisions = match &query {
            Some(query) => self
                .store
                .rank_decisions(&request.project_id, query, MAX_RANKED)?,
            None => Vec::new(),
        };
        let lexical_claims = match &query {
            Some(query) => self
                .store
                .rank_claims(&request.project_id, query, MAX_RANKED)?,
            None => Vec::new(),
        };
        let ranked_decisions = first_unique(by_file_decisions, lexical_decisions);
        let ranked_claims = first_unique(by_file_claims, lexical_claims);

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
            let item = pack_claim(claim, matched)?;
            if selection.take(
                serde_json::to_string(&item)
                    .map_err(|e| ContextError::Storage(e.to_string()))?
                    .chars()
                    .count(),
            ) {
                pack_claims.push(item);
            }
        }

        let (observations, observation_coverage) = if request.as_of.is_some() {
            (Vec::new(), Default::default())
        } else {
            let snapshot = self
                .store
                .snapshot(&request.project_id)
                .map_err(|error| ContextError::Storage(error.to_string()))?;
            if snapshot.sources.iter().any(|source| {
                matches!(
                    source.last_check_status,
                    crate::observations::CheckStatus::Unreadable
                        | crate::observations::CheckStatus::Unsupported
                        | crate::observations::CheckStatus::QuotaExceeded
                )
            }) {
                self.store
                    .request_refresh(&crate::observations::RefreshRequest {
                        project_id: request.project_id.clone(),
                        capture_trigger: "context_suspect_source".into(),
                        requested_at: now_rfc3339(),
                    })
                    .map_err(|error| ContextError::Storage(error.to_string()))?;
            }
            let mut observation_files = files.clone();
            observation_files.extend(crate::injection::mentioned_paths(&task, ""));
            let eligible = if self.store.observations_dirty(&request.project_id)? {
                Vec::new()
            } else {
                select_observations(&snapshot, &request.project_id, &task, &observation_files)
            };
            let observations = eligible
                .into_iter()
                .filter(|item| {
                    selection.take(crate::injection::observation_line(item).chars().count())
                })
                .collect();
            (observations, snapshot.coverage)
        };
        let mut pack = ContextPack {
            observations,
            observation_coverage,
            project_id: request.project_id,
            task,
            as_of: as_of.to_string(),
            budget_chars: budget,
            used_chars: selection.used,
            decisions,
            claims: pack_claims,
            omitted: selection.omitted,
        };
        if let Some(routing) = &self.routing {
            routing.apply(&mut pack, &files, request.as_of.is_some());
        }
        Ok(pack)
    }
}

pub(crate) fn routing_tokens(text: &str) -> BTreeSet<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .map(str::to_lowercase)
        .filter(|w| w.chars().count() >= 4 && !STOPWORDS.contains(&w.as_str()))
        .collect()
}

/// Deterministic current-source routing, independent of the normative graph.
pub(crate) fn select_observations(
    snapshot: &crate::observations::ObservationSnapshot,
    project_id: &str,
    task: &str,
    files: &[String],
) -> Vec<crate::observations::PackObservation> {
    use crate::observations::{
        CheckStatus, ObservationAuthority, ObservationSubject, RecordStatus,
    };
    let verified: BTreeMap<_, _> = snapshot
        .sources
        .iter()
        .filter(|source| {
            source.project_id == project_id && source.last_check_status == CheckStatus::Verified
        })
        .map(|source| (source.source_id.as_str(), source))
        .collect();
    let tokens: BTreeSet<_> = task
        .split(|c: char| !(c.is_alphanumeric() || "_-@/".contains(c)))
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect();
    let current: Vec<_> = snapshot
        .observations
        .iter()
        .filter(|record| {
            record.project_id == project_id
                && record.status == RecordStatus::Current
                && verified
                    .get(record.provenance.source_id.as_str())
                    .is_some_and(|source| {
                        source.sha256.as_deref() == Some(record.provenance.source_sha256.as_str())
                            && source.parser_policy_version
                                == record.provenance.parser_policy_version
                    })
        })
        .collect();
    let scope_length = |record: &crate::observations::ObservationRecord, file: &str| {
        let declared = record
            .path_scope
            .iter()
            .filter_map(|scope| {
                let scope = scope.trim_end_matches('/');
                (file == scope || file.starts_with(&format!("{scope}/"))).then_some(scope.len())
            })
            .max();
        let package = verified
            .get(record.provenance.source_id.as_str())
            .and_then(|source| {
                let scope = source
                    .project_relative_path
                    .rsplit_once('/')
                    .map_or("", |(parent, _)| parent);
                (scope.is_empty() || file.starts_with(&format!("{scope}/"))).then_some(scope.len())
            });
        declared.into_iter().chain(package).max()
    };
    let longest: Vec<_> = files
        .iter()
        .map(|file| {
            current
                .iter()
                .filter_map(|record| scope_length(record, file))
                .max()
        })
        .collect();
    let mut selected: Vec<_> = current
        .into_iter()
        .filter(|record| {
            let name = match &record.subject {
                ObservationSubject::Package { name } | ObservationSubject::Dependency { name } => {
                    name
                }
            };
            tokens.contains(&name.to_lowercase())
                || files.iter().zip(&longest).any(|(file, longest)| {
                    scope_length(record, file).is_some_and(|length| Some(length) == *longest)
                        || verified
                            .get(record.provenance.source_id.as_str())
                            .is_some_and(|source| source.project_relative_path == *file)
                })
        })
        .map(|record| crate::observations::PackObservation {
            record: record.clone(),
            authority: ObservationAuthority::Descriptive,
        })
        .collect();
    selected.sort_by(|a, b| a.record.observation_id.cmp(&b.record.observation_id));
    selected.truncate(crate::observations::MAX_OBSERVATIONS);
    selected
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
            qualifiers: crate::qualifiers::decode(&decision.qualifiers)
                .map_err(ContextError::Storage)?,
            scope: serde_json::from_str(&decision.scope)
                .map_err(|e| ContextError::Storage(e.to_string()))?,
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
/// `first` then the items of `then` not already in it.
fn first_unique(mut first: Vec<String>, then: Vec<String>) -> Vec<String> {
    for item in then {
        if !first.contains(&item) {
            first.push(item);
        }
    }
    first
}

fn graph_error(error: GraphError) -> ContextError {
    match error {
        GraphError::ProjectNotFound => ContextError::ProjectNotFound,
        GraphError::InvalidRequest(message) => ContextError::InvalidRequest(message),
        other => ContextError::Storage(other.to_string()),
    }
}

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
    serde_json::to_string(decision)
        .expect("pack serialization is infallible")
        .chars()
        .count()
}

fn pack_claim(claim: &ClaimRecord, matched: bool) -> Result<PackClaim, ContextError> {
    Ok(PackClaim {
        source_version: claim.source_version,
        inherited_scope: serde_json::from_str(&claim.inherited_scope)
            .map_err(|e| ContextError::Storage(e.to_string()))?,
        qualifiers: crate::qualifiers::decode(&claim.qualifiers).map_err(ContextError::Storage)?,
        claim_id: claim.claim_id.clone(),
        kind: claim.kind.as_str().to_string(),
        statement: claim.statement.clone(),
        valid_from: claim.valid_from.clone(),
        valid_until: claim.valid_until.clone(),
        source_decision_id: claim.source_decision_id.clone(),
        matched,
    })
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
    fn observations_require_verified_matching_provenance_and_explicit_subject() {
        use crate::observations::*;
        let source = ObservationSource {
            source_id: "source".into(),
            project_id: "p".into(),
            project_relative_path: "pkg/Cargo.toml".into(),
            manifest_kind: ManifestKind::Cargo,
            sha256: Some("hash".into()),
            parser_policy_version: "1".into(),
            last_checked_at: None,
            last_check_status: CheckStatus::Verified,
            semantic_cache: None,
        };
        let record = ObservationRecord {
            observation_id: "o".into(),
            project_id: "p".into(),
            version: 1,
            subject: ObservationSubject::Dependency {
                name: "serde".into(),
            },
            value: ObservationValue {
                declared_version: None,
                dependency_category: None,
                version_requirement: Some("1".into()),
                target: None,
            },
            path_scope: vec!["pkg".into()],
            provenance: ObservationProvenance {
                source_id: "source".into(),
                source_sha256: "hash".into(),
                supporting_sources: vec![],
                parser_policy_version: "1".into(),
                field_pointer: "dependencies.serde".into(),
                capture_trigger: "test".into(),
                commit: None,
            },
            observed_at: "2026-10-04T00:00:00Z".into(),
            status: RecordStatus::Current,
            invalidated_at: None,
            invalidation_reason: None,
        };
        let mut snapshot = ObservationSnapshot {
            observations: vec![record],
            sources: vec![source],
            coverage: Default::default(),
        };
        assert!(super::select_observations(&snapshot, "p", "alterar projeto", &[]).is_empty());
        assert_eq!(
            super::select_observations(&snapshot, "p", "usar serde", &[]).len(),
            1
        );
        assert_eq!(
            super::select_observations(&snapshot, "p", "", &["pkg/src/lib.rs".into()]).len(),
            1
        );
        assert!(super::select_observations(&snapshot, "other", "serde", &[]).is_empty());
        snapshot.sources[0].last_check_status = CheckStatus::Unreadable;
        assert!(super::select_observations(&snapshot, "p", "serde", &[]).is_empty());
        snapshot.sources[0].last_check_status = CheckStatus::Verified;
        snapshot.sources[0].sha256 = Some("changed".into());
        assert!(super::select_observations(&snapshot, "p", "serde", &[]).is_empty());
    }

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
