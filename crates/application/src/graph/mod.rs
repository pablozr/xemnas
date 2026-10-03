//! Knowledge graph of a project (ADR-0005): components and technologies,
//! and the typed, temporal, human-confirmed edges that tie decisions and
//! claims to them.
//!
//! Nothing is ever deleted: entities are retired, edges are invalidated.
//! Derived edges start as suggestions and only count once confirmed.

mod derive;
mod discover;
mod query;

pub use derive::{added_dependencies, ComponentProposal, SuggestionReport, TechnologyProposal};
pub use discover::{declared_components, DeclaredComponent, WorkspaceKind};
pub use query::{
    DecisionParts, EntityDetail, FileLens, GraphEdge, GraphNode, MapEntity, Neighborhood, NodeRef,
    NodeSummary, PartCount, ProjectGraph, ProjectMap, Suggestion, TimelineEvent, TimelineKind,
    DEFAULT_NEIGHBORHOOD_LIMIT, MAX_NEIGHBORHOOD_DEPTH,
};

use domain::entities::{
    check_part_of, entity_description, entity_key, entity_name, path_pattern, EdgeKind, EdgeOrigin,
    EntityError, EntityKind, NodeKind,
};
use domain::time::Timestamp;

use crate::claims::{ClaimStore, ClaimsError};
use crate::clock::now_rfc3339;
use crate::decisions::DecisionsError;
use crate::projects::{ProjectError, ProjectRepository};
use crate::relations::RelationStore;

/// Most path patterns or aliases one entity keeps.
pub const MAX_ENTITY_LIST: usize = 20;

/// Failure modes of the graph use cases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphError {
    /// The storage backend failed; the message is diagnostic only.
    Storage(String),
    /// The entity or edge does not exist.
    NotFound,
    /// The project does not exist.
    ProjectNotFound,
    /// The decision or claim does not exist in this project.
    InvalidSource,
    /// A domain rule was broken.
    Invalid(EntityError),
    /// Another entity of the same kind already has this name or alias.
    DuplicateName,
    /// The same live edge already exists.
    DuplicateEdge,
    /// The entity or edge changed (retired, confirmed, invalidated) meanwhile.
    Conflict,
    /// The request is malformed.
    InvalidRequest(String),
}

impl GraphError {
    /// Short, stable code.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Storage(_) => "storage",
            Self::NotFound => "not_found",
            Self::ProjectNotFound => "project_not_found",
            Self::InvalidSource => "invalid_source",
            Self::Invalid(error) => error.code(),
            Self::DuplicateName => "duplicate_name",
            Self::DuplicateEdge => "duplicate_edge",
            Self::Conflict => "conflict",
            Self::InvalidRequest(_) => "invalid_request",
        }
    }
}

impl std::fmt::Display for GraphError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Storage(message) => write!(formatter, "falha de armazenamento: {message}"),
            Self::NotFound => formatter.write_str("item do mapa não encontrado"),
            Self::ProjectNotFound => formatter.write_str("projeto não encontrado"),
            Self::InvalidSource => {
                formatter.write_str("a decisão ou regra não existe neste projeto")
            }
            Self::Invalid(error) => write!(formatter, "{error}"),
            Self::DuplicateName => {
                formatter.write_str("já existe um item desse tipo com esse nome ou apelido")
            }
            Self::DuplicateEdge => formatter.write_str("esse vínculo já existe"),
            Self::Conflict => formatter.write_str("o item mudou; atualize e tente de novo"),
            Self::InvalidRequest(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for GraphError {}

impl From<EntityError> for GraphError {
    fn from(error: EntityError) -> Self {
        Self::Invalid(error)
    }
}

impl From<DecisionsError> for GraphError {
    fn from(error: DecisionsError) -> Self {
        Self::Storage(error.to_string())
    }
}

impl From<ClaimsError> for GraphError {
    fn from(error: ClaimsError) -> Self {
        Self::Storage(error.to_string())
    }
}

impl From<ProjectError> for GraphError {
    fn from(error: ProjectError) -> Self {
        Self::Storage(error.to_string())
    }
}

/// A persisted entity with its patterns and aliases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityRecord {
    /// Identifier (UUID v7).
    pub entity_id: String,
    /// Owning project.
    pub project_id: String,
    /// Component or technology.
    pub kind: EntityKind,
    /// Display name.
    pub name: String,
    /// Normalized name ([`entity_key`]).
    pub key: String,
    /// Optional description.
    pub description: String,
    /// Path patterns (components only).
    pub patterns: Vec<String>,
    /// Alternative names.
    pub aliases: Vec<String>,
    /// RFC 3339 creation time.
    pub created_at: String,
    /// RFC 3339 retirement time, when retired.
    pub retired_at: Option<String>,
}

impl EntityRecord {
    /// Whether the entity exists at `at`.
    pub fn alive_at(&self, at: &Timestamp) -> bool {
        before_or_at(&self.created_at, at)
            && !self
                .retired_at
                .as_deref()
                .is_some_and(|end| before_or_at(end, at))
    }

    /// Every key that resolves to this entity: its name and aliases.
    ///
    /// Computed from the name, so rows stored under an older key rule resolve
    /// the same way as new ones.
    pub fn keys(&self) -> impl Iterator<Item = String> + '_ {
        std::iter::once(entity_key(&self.name))
            .chain(self.aliases.iter().map(|alias| entity_key(alias)))
    }
}

/// A persisted edge from a decision, claim or entity to an entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeRecord {
    /// Identifier (UUID v7).
    pub edge_id: String,
    /// Owning project.
    pub project_id: String,
    /// Edge kind.
    pub kind: EdgeKind,
    /// Kind of the source node.
    pub source_kind: NodeKind,
    /// Source node id.
    pub source_id: String,
    /// Target entity.
    pub entity_id: String,
    /// Who created it.
    pub origin: EdgeOrigin,
    /// Why it was derived (the matching file or dependency); empty for human edges.
    pub reason: String,
    /// RFC 3339 creation time.
    pub created_at: String,
    /// RFC 3339 confirmation time; `None` while it is a suggestion.
    pub confirmed_at: Option<String>,
    /// RFC 3339 invalidation time, when invalidated.
    pub invalidated_at: Option<String>,
}

impl EdgeRecord {
    /// Whether the edge holds at `at`: confirmed by then and not invalidated.
    pub fn holds_at(&self, at: &Timestamp) -> bool {
        self.confirmed_at
            .as_deref()
            .is_some_and(|when| before_or_at(when, at))
            && !self
                .invalidated_at
                .as_deref()
                .is_some_and(|when| before_or_at(when, at))
    }

    /// Whether the edge is a live suggestion.
    pub fn is_pending(&self) -> bool {
        self.confirmed_at.is_none() && self.invalidated_at.is_none()
    }

    /// Whether the edge is not invalidated (confirmed or pending).
    pub fn is_live(&self) -> bool {
        self.invalidated_at.is_none()
    }
}

/// A decision as the graph needs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionNode {
    /// Decision identifier.
    pub decision_id: String,
    /// Question answered.
    pub question: String,
    /// Choice made.
    pub choice: String,
    /// RFC 3339 confirmation time.
    pub confirmed_at: String,
    /// Files the decision's capture changed (`diff_summary.files`).
    pub files: Vec<String>,
    /// Diff hunks of the decision's evidence, for dependency detection.
    pub diffs: Vec<String>,
}

/// A rule adopted from a candidate, with the files its evidence touched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleSource {
    /// The claim.
    pub claim_id: String,
    /// Files of the candidate's evidence (`diff_summary.files`).
    pub files: Vec<String>,
}

/// Persistence port of the graph.
pub trait GraphStore {
    /// Every entity of a project, with patterns and aliases.
    fn project_entities(&self, project_id: &str) -> Result<Vec<EntityRecord>, GraphError>;

    /// One entity.
    fn get_entity(&self, entity_id: &str) -> Result<Option<EntityRecord>, GraphError>;

    /// Inserts an entity with its patterns and aliases.
    fn insert_entity(&self, record: &EntityRecord) -> Result<(), GraphError>;

    /// Replaces name, key, description, patterns and aliases of a live entity;
    /// `false` when it was retired meanwhile.
    fn update_entity(&self, record: &EntityRecord) -> Result<bool, GraphError>;

    /// Retires a live entity; `false` when it was already retired.
    fn retire_entity(&self, entity_id: &str, at: &str) -> Result<bool, GraphError>;

    /// Every edge of a project, invalidated ones included.
    fn project_edges(&self, project_id: &str) -> Result<Vec<EdgeRecord>, GraphError>;

    /// One edge.
    fn get_edge(&self, edge_id: &str) -> Result<Option<EdgeRecord>, GraphError>;

    /// Inserts an edge.
    fn insert_edge(&self, record: &EdgeRecord) -> Result<(), GraphError>;

    /// Confirms a pending edge; `false` when it is no longer pending.
    fn confirm_edge(&self, edge_id: &str, at: &str) -> Result<bool, GraphError>;

    /// Invalidates a live edge; `false` when it was already invalidated.
    fn invalidate_edge(&self, edge_id: &str, at: &str) -> Result<bool, GraphError>;

    /// Every decision of the project that was ever confirmed, with the files
    /// and diff hunks of its capture.
    fn project_decisions(&self, project_id: &str) -> Result<Vec<DecisionNode>, GraphError>;

    /// Rules of the project adopted from a candidate, with its files.
    fn rule_sources(&self, project_id: &str) -> Result<Vec<RuleSource>, GraphError>;
}

/// Input for [`KnowledgeGraph::create_entity`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NewEntity {
    /// Owning project.
    pub project_id: String,
    /// Component or technology.
    pub kind: Option<EntityKind>,
    /// Display name.
    pub name: String,
    /// Optional description.
    pub description: String,
    /// Path patterns (components only).
    pub patterns: Vec<String>,
    /// Alternative names.
    pub aliases: Vec<String>,
}

/// Input for [`KnowledgeGraph::update_entity`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EntityEdit {
    /// New display name.
    pub name: String,
    /// New description.
    pub description: String,
    /// New path patterns.
    pub patterns: Vec<String>,
    /// New aliases.
    pub aliases: Vec<String>,
}

/// Input for [`KnowledgeGraph::link`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkRequest {
    /// Edge kind.
    pub kind: EdgeKind,
    /// Source node kind.
    pub source_kind: NodeKind,
    /// Source node id.
    pub source_id: String,
    /// Target entity.
    pub entity_id: String,
}

/// Graph use cases and queries over the local store.
#[derive(Debug, Clone)]
pub struct KnowledgeGraph<S> {
    store: S,
}

impl<S> KnowledgeGraph<S> {
    /// Wraps the store.
    pub fn new(store: S) -> Self {
        Self { store }
    }
}

impl<S> KnowledgeGraph<S>
where
    S: GraphStore + RelationStore + ClaimStore + ProjectRepository,
{
    /// Creates a component or technology, confirmed on creation.
    ///
    /// # Errors
    ///
    /// Domain rule violations, an unknown project, or a name or alias another
    /// live entity of the same kind already uses.
    pub fn create_entity(&self, input: NewEntity) -> Result<EntityRecord, GraphError> {
        let kind = input
            .kind
            .ok_or_else(|| GraphError::InvalidRequest("escolha componente ou tecnologia".into()))?;
        ProjectRepository::get(&self.store, &input.project_id)?
            .ok_or(GraphError::ProjectNotFound)?;
        let now = now_rfc3339();
        let mut record = EntityRecord {
            entity_id: uuid::Uuid::now_v7().to_string(),
            project_id: input.project_id,
            kind,
            name: String::new(),
            key: String::new(),
            description: String::new(),
            patterns: Vec::new(),
            aliases: Vec::new(),
            created_at: now,
            retired_at: None,
        };
        apply_edit(
            &mut record,
            EntityEdit {
                name: input.name,
                description: input.description,
                patterns: input.patterns,
                aliases: input.aliases,
            },
        )?;
        self.check_unique(&record)?;
        self.store.insert_entity(&record)?;
        Ok(record)
    }

    /// Renames or re-describes a live entity and replaces its patterns and aliases.
    ///
    /// # Errors
    ///
    /// `not_found`, domain rule violations, `duplicate_name`, or `conflict`
    /// when the entity was retired.
    pub fn update_entity(
        &self,
        entity_id: &str,
        edit: EntityEdit,
    ) -> Result<EntityRecord, GraphError> {
        let mut record = self
            .store
            .get_entity(entity_id)?
            .ok_or(GraphError::NotFound)?;
        if record.retired_at.is_some() {
            return Err(GraphError::Conflict);
        }
        apply_edit(&mut record, edit)?;
        self.check_unique(&record)?;
        if !self.store.update_entity(&record)? {
            return Err(GraphError::Conflict);
        }
        Ok(record)
    }

    /// Retires an entity and invalidates its live edges; the history stays.
    ///
    /// # Errors
    ///
    /// `not_found`, or `conflict` when it was already retired.
    pub fn retire_entity(&self, entity_id: &str) -> Result<EntityRecord, GraphError> {
        let record = self
            .store
            .get_entity(entity_id)?
            .ok_or(GraphError::NotFound)?;
        let now = now_rfc3339();
        if !self.store.retire_entity(entity_id, &now)? {
            return Err(GraphError::Conflict);
        }
        for edge in self.store.project_edges(&record.project_id)? {
            let touches = edge.entity_id == entity_id
                || (edge.source_kind == NodeKind::Entity && edge.source_id == entity_id);
            if touches && edge.is_live() {
                self.store.invalidate_edge(&edge.edge_id, &now)?;
            }
        }
        Ok(EntityRecord {
            retired_at: Some(now),
            ..record
        })
    }

    /// Records a human edge, confirmed on creation.
    ///
    /// # Errors
    ///
    /// `invalid_source` for a source outside the project, the domain's edge
    /// rules, `duplicate_edge`, or `not_found` for an unknown entity.
    pub fn link(&self, request: LinkRequest) -> Result<EdgeRecord, GraphError> {
        let entity = self.live_entity(&request.entity_id)?;
        request.kind.check(request.source_kind, entity.kind)?;
        self.check_source(&entity.project_id, request.source_kind, &request.source_id)?;
        let edges = self.store.project_edges(&entity.project_id)?;
        if request.kind == EdgeKind::PartOf {
            let existing: Vec<(String, String)> = edges
                .iter()
                .filter(|edge| edge.kind == EdgeKind::PartOf && edge.is_live())
                .map(|edge| (edge.source_id.clone(), edge.entity_id.clone()))
                .collect();
            check_part_of(&request.source_id, &request.entity_id, &existing)?;
        }
        if let Some(pending) = edges
            .iter()
            .find(|edge| same_edge(edge, &request) && edge.is_live())
        {
            if pending.is_pending() {
                // Linking what was already suggested confirms the suggestion.
                return self.confirm(&pending.edge_id);
            }
            return Err(GraphError::DuplicateEdge);
        }
        let now = now_rfc3339();
        let record = EdgeRecord {
            edge_id: uuid::Uuid::now_v7().to_string(),
            project_id: entity.project_id,
            kind: request.kind,
            source_kind: request.source_kind,
            source_id: request.source_id,
            entity_id: request.entity_id,
            origin: EdgeOrigin::Human,
            reason: String::new(),
            created_at: now.clone(),
            confirmed_at: Some(now),
            invalidated_at: None,
        };
        self.store.insert_edge(&record)?;
        Ok(record)
    }

    /// Confirms a suggested edge.
    ///
    /// # Errors
    ///
    /// `not_found`, or `conflict` when it is no longer a suggestion.
    pub fn confirm(&self, edge_id: &str) -> Result<EdgeRecord, GraphError> {
        let edge = self.edge(edge_id)?;
        let now = now_rfc3339();
        if !self.store.confirm_edge(edge_id, &now)? {
            return Err(GraphError::Conflict);
        }
        Ok(EdgeRecord {
            confirmed_at: Some(now),
            ..edge
        })
    }

    /// Rejects a suggestion or removes a confirmed edge; both keep the row
    /// as invalidated, so a derivation never proposes it again.
    ///
    /// # Errors
    ///
    /// `not_found`, or `conflict` when it was already invalidated.
    pub fn invalidate(&self, edge_id: &str) -> Result<EdgeRecord, GraphError> {
        let edge = self.edge(edge_id)?;
        let now = now_rfc3339();
        if !self.store.invalidate_edge(edge_id, &now)? {
            return Err(GraphError::Conflict);
        }
        Ok(EdgeRecord {
            invalidated_at: Some(now),
            ..edge
        })
    }

    fn edge(&self, edge_id: &str) -> Result<EdgeRecord, GraphError> {
        self.store.get_edge(edge_id)?.ok_or(GraphError::NotFound)
    }

    fn live_entity(&self, entity_id: &str) -> Result<EntityRecord, GraphError> {
        let entity = self
            .store
            .get_entity(entity_id)?
            .ok_or(GraphError::NotFound)?;
        if entity.retired_at.is_some() {
            return Err(GraphError::Conflict);
        }
        Ok(entity)
    }

    fn check_source(&self, project_id: &str, kind: NodeKind, id: &str) -> Result<(), GraphError> {
        let found = match kind {
            NodeKind::Decision => self
                .store
                .project_decisions(project_id)?
                .iter()
                .any(|decision| decision.decision_id == id),
            NodeKind::Claim => self
                .store
                .project_claims(project_id)?
                .iter()
                .any(|claim| claim.claim_id == id),
            NodeKind::Entity => self.store.get_entity(id)?.is_some_and(|entity| {
                entity.project_id == project_id && entity.retired_at.is_none()
            }),
        };
        if found {
            Ok(())
        } else {
            Err(GraphError::InvalidSource)
        }
    }

    fn check_unique(&self, record: &EntityRecord) -> Result<(), GraphError> {
        let keys: Vec<String> = record.keys().collect();
        let taken = self
            .store
            .project_entities(&record.project_id)?
            .iter()
            .filter(|other| {
                other.entity_id != record.entity_id
                    && other.kind == record.kind
                    && other.retired_at.is_none()
            })
            .any(|other| other.keys().any(|key| keys.contains(&key)));
        if taken {
            Err(GraphError::DuplicateName)
        } else {
            Ok(())
        }
    }
}

fn apply_edit(record: &mut EntityRecord, edit: EntityEdit) -> Result<(), GraphError> {
    record.name = entity_name(&edit.name)?;
    record.key = entity_key(&record.name);
    record.description = entity_description(&edit.description)?;
    let patterns = edit
        .patterns
        .iter()
        .filter(|pattern| !pattern.trim().is_empty())
        .map(|pattern| path_pattern(pattern))
        .collect::<Result<Vec<_>, _>>()?;
    if !patterns.is_empty() && record.kind == EntityKind::Technology {
        return Err(EntityError::PatternOnTechnology.into());
    }
    let aliases = edit
        .aliases
        .iter()
        .filter(|alias| !alias.trim().is_empty())
        .map(|alias| entity_name(alias))
        .collect::<Result<Vec<_>, _>>()?;
    if patterns.len() > MAX_ENTITY_LIST || aliases.len() > MAX_ENTITY_LIST {
        return Err(GraphError::InvalidRequest(
            "use até 20 padrões e 20 apelidos por item".into(),
        ));
    }
    record.patterns = dedup(patterns);
    record.aliases = dedup(aliases);
    Ok(())
}

fn dedup(items: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::BTreeSet::new();
    items
        .into_iter()
        .filter(|item| seen.insert(item.to_lowercase()))
        .collect()
}

fn same_edge(edge: &EdgeRecord, request: &LinkRequest) -> bool {
    edge.kind == request.kind
        && edge.source_kind == request.source_kind
        && edge.source_id == request.source_id
        && edge.entity_id == request.entity_id
}

/// `files` of a candidate's `diff_summary` JSON; malformed summaries have none.
pub fn summary_files(summary: &str) -> Vec<String> {
    serde_json::from_str::<serde_json::Value>(summary)
        .ok()
        .and_then(|value| {
            value.get("files")?.as_array().map(|files| {
                files
                    .iter()
                    .filter_map(|file| file.as_str().map(str::to_string))
                    .collect()
            })
        })
        .unwrap_or_default()
}

/// Whether `when` is at or before `at`; unparseable dates never are.
pub(crate) fn before_or_at(when: &str, at: &Timestamp) -> bool {
    Timestamp::parse(when).is_some_and(|when| when <= *at)
}

/// `as_of` parsed, or now.
pub(crate) fn resolve_as_of(as_of: Option<&str>) -> Result<Timestamp, GraphError> {
    let now = now_rfc3339();
    Timestamp::parse(as_of.unwrap_or(&now))
        .ok_or_else(|| GraphError::InvalidRequest("use a data no formato AAAA-MM-DD".into()))
}
