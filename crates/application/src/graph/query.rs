//! Read models of the graph (ADR-0005): project map, entity detail,
//! neighborhood, file lens, impact, timeline and pending suggestions. Every
//! query takes an `as_of` date and never shows the whole graph at once.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use domain::entities::{normalize_path, pattern_matches, EdgeKind, EntityKind, NodeKind};
use domain::relations::RelationKind;
use domain::time::Timestamp;

use super::{
    before_or_at, resolve_as_of, DecisionNode, EdgeRecord, EntityRecord, GraphError, GraphStore,
    KnowledgeGraph,
};
use crate::claims::{ClaimRecord, ClaimStore};
use crate::projects::ProjectRepository;
use crate::relations::{RelationRow, RelationStore};

/// Deepest neighborhood the queries expand.
pub const MAX_NEIGHBORHOOD_DEPTH: usize = 2;

/// Nodes a neighborhood returns before it is truncated.
pub const DEFAULT_NEIGHBORHOOD_LIMIT: usize = 40;

/// A node of the graph, by kind and id.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeRef {
    /// Node kind.
    pub kind: NodeKind,
    /// Node id.
    pub id: String,
}

impl NodeRef {
    /// A decision node.
    pub fn decision(id: impl Into<String>) -> Self {
        Self {
            kind: NodeKind::Decision,
            id: id.into(),
        }
    }

    /// A claim node.
    pub fn claim(id: impl Into<String>) -> Self {
        Self {
            kind: NodeKind::Claim,
            id: id.into(),
        }
    }

    /// An entity node.
    pub fn entity(id: impl Into<String>) -> Self {
        Self {
            kind: NodeKind::Entity,
            id: id.into(),
        }
    }
}

/// A decision, claim or entity as a list row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeSummary {
    /// The node.
    pub node: NodeRef,
    /// Main text: question, statement or entity name.
    pub label: String,
    /// Secondary text: choice, claim kind or entity kind.
    pub detail: String,
    /// Relevant date: confirmation, start of validity or creation.
    pub at: String,
}

/// A node in a neighborhood.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphNode {
    /// Row data.
    pub summary: NodeSummary,
    /// Hops from the center (0 for the center).
    pub depth: usize,
    /// Whether it holds at the date asked (decision in force, claim valid,
    /// entity alive).
    pub active: bool,
}

/// An edge in a neighborhood.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphEdge {
    /// Source node.
    pub from: NodeRef,
    /// Target node.
    pub to: NodeRef,
    /// Edge kind literal: an [`EdgeKind`] or a [`RelationKind`].
    pub kind: String,
}

/// A node and what is around it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Neighborhood {
    /// The center first, then by depth.
    pub nodes: Vec<GraphNode>,
    /// Edges between returned nodes.
    pub edges: Vec<GraphEdge>,
    /// Whether nodes were left out by the limit.
    pub truncated: bool,
}

/// One entity on the project map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapEntity {
    /// The entity.
    pub entity: EntityRecord,
    /// Decisions in force tied to it.
    pub decisions: usize,
    /// Claims valid that apply to it.
    pub claims: usize,
    /// Most recent confirmation among its decisions and edges.
    pub last_activity: Option<String>,
    /// Conflicting pairs of decisions in force on it.
    pub conflicts: usize,
}

/// Everything on the project map at a date, for the graph view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectGraph {
    /// Entities alive, decisions in force and claims valid (depth 0).
    pub nodes: Vec<GraphNode>,
    /// Confirmed edges and decision relations between returned nodes.
    pub edges: Vec<GraphEdge>,
    /// Suggestions waiting for confirmation, by edge id.
    pub suggested: Vec<(String, GraphEdge)>,
}

/// Components and technologies of a project at a date.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectMap {
    /// Date used.
    pub as_of: String,
    /// Entities alive at `as_of`, components first, then by name.
    pub entities: Vec<MapEntity>,
    /// `(child, parent)` component pairs.
    pub part_of: Vec<(String, String)>,
    /// Suggestions waiting for confirmation.
    pub pending: usize,
}

/// Everything around one entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityDetail {
    /// The entity.
    pub entity: EntityRecord,
    /// Decisions in force tied to it, newest first.
    pub decisions: Vec<NodeSummary>,
    /// Claims valid that apply to it.
    pub claims: Vec<NodeSummary>,
    /// Parent component, when any.
    pub parent: Option<NodeSummary>,
    /// Components that are part of it.
    pub parts: Vec<NodeSummary>,
    /// Conflicting pairs of decisions in force on it.
    pub conflicts: Vec<(NodeSummary, NodeSummary)>,
    /// Decisions that depend, transitively, on its decisions.
    pub impact: Vec<NodeSummary>,
}

/// What holds for one file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileLens {
    /// Normalized path asked.
    pub path: String,
    /// Components whose patterns match, with their parents.
    pub components: Vec<NodeSummary>,
    /// Decisions in force tied to those components, newest first.
    pub decisions: Vec<NodeSummary>,
    /// Claims valid that apply to those components.
    pub claims: Vec<NodeSummary>,
}

/// A suggestion waiting for confirmation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    /// Edge id to confirm or reject.
    pub edge_id: String,
    /// Edge kind.
    pub kind: EdgeKind,
    /// The decision it starts from.
    pub source: NodeSummary,
    /// The entity it points to.
    pub entity: NodeSummary,
    /// Why it was derived: a file, a dependency or a mention
    /// ([`super::mention_quote`] reads its quote).
    pub reason: String,
    /// RFC 3339 time it was derived.
    pub created_at: String,
}

/// What happened in a timeline event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TimelineKind {
    /// A decision was confirmed.
    DecisionConfirmed,
    /// A decision was replaced.
    DecisionSuperseded,
    /// A claim started to hold.
    ClaimStarted,
    /// A claim stopped holding.
    ClaimEnded,
    /// An entity was created.
    EntityCreated,
    /// An entity was retired.
    EntityRetired,
    /// An edge was confirmed.
    EdgeConfirmed,
    /// An edge was removed.
    EdgeInvalidated,
}

impl TimelineKind {
    /// Stable literal.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::DecisionConfirmed => "decision_confirmed",
            Self::DecisionSuperseded => "decision_superseded",
            Self::ClaimStarted => "claim_started",
            Self::ClaimEnded => "claim_ended",
            Self::EntityCreated => "entity_created",
            Self::EntityRetired => "entity_retired",
            Self::EdgeConfirmed => "edge_confirmed",
            Self::EdgeInvalidated => "edge_invalidated",
        }
    }
}

/// One dated event of the project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineEvent {
    /// RFC 3339 time.
    pub at: String,
    /// What happened.
    pub kind: TimelineKind,
    /// The node it happened to.
    pub node: NodeSummary,
    /// The other end, for edges and supersessions.
    pub other: Option<NodeSummary>,
}

/// A part of the project: a top-level component of the map (the same
/// "container" as in [`crate::architecture`]) and how much was decided on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartCount {
    /// The component's entity id.
    pub entity_id: String,
    /// Its name.
    pub name: String,
    /// Decisions in force tied to it or to a component inside it.
    pub decisions: usize,
}

/// Decisions in force grouped by part of the project, to narrow a list of
/// decisions to "everything decided about the storage".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DecisionParts {
    /// Every live top-level component, by name (parts without decisions too).
    pub parts: Vec<PartCount>,
    /// Decisions in force of each part (by entity id), newest first.
    pub decisions: BTreeMap<String, Vec<String>>,
}

impl DecisionParts {
    /// The part with this entity id.
    pub fn part(&self, entity_id: &str) -> Option<&PartCount> {
        self.parts.iter().find(|part| part.entity_id == entity_id)
    }

    /// Decisions in force of a part, newest first (empty for an unknown part).
    pub fn decisions_of(&self, entity_id: &str) -> &[String] {
        self.decisions
            .get(entity_id)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    /// Parts a decision affects, directly or through a component inside them,
    /// in the order of [`DecisionParts::parts`].
    pub fn parts_of(&self, decision_id: &str) -> Vec<&PartCount> {
        self.parts
            .iter()
            .filter(|part| {
                self.decisions_of(&part.entity_id)
                    .iter()
                    .any(|id| id == decision_id)
            })
            .collect()
    }
}

/// Climbs of `part_of` before giving up (a cycle in the data).
const MAX_PART_DEPTH: usize = 16;

/// Whether a decision holds at `at`: confirmed by then and not superseded by then.
pub(crate) fn decision_in_force(
    decision: &DecisionNode,
    relations: &[RelationRow],
    at: &Timestamp,
) -> bool {
    before_or_at(&decision.confirmed_at, at)
        && !relations.iter().any(|row| {
            row.kind == RelationKind::Supersedes.as_str()
                && row.to == decision.decision_id
                && before_or_at(&row.created_at, at)
        })
}

/// The graph's share of one Context Pack.
pub(crate) struct PackGraph {
    pub decisions: Vec<String>,
    pub scopes: super::scope::ClaimScopes,
    pub all_claims: Vec<ClaimRecord>,
}

fn files_context(snapshot: &Snapshot, files: &[String]) -> (Vec<String>, Vec<String>) {
    let components = components_for(snapshot, files);
    let (decisions, claims) = tied_to_all(snapshot, &components);
    let decisions = snapshot
        .summaries(&decisions, NodeKind::Decision)
        .into_iter()
        .map(|summary| summary.node.id)
        .collect();
    (decisions, claims)
}

/// Everything the queries read, loaded once per call.
struct Snapshot {
    at: Timestamp,
    entities: BTreeMap<String, EntityRecord>,
    edges: Vec<EdgeRecord>,
    decisions: BTreeMap<String, DecisionNode>,
    claims: BTreeMap<String, ClaimRecord>,
    relations: Vec<RelationRow>,
}

impl Snapshot {
    fn decision_active(&self, id: &str) -> bool {
        self.decisions
            .get(id)
            .is_some_and(|decision| decision_in_force(decision, &self.relations, &self.at))
    }

    fn claim_active(&self, id: &str) -> bool {
        self.claims
            .get(id)
            .is_some_and(|claim| claim.is_valid_at(&self.at))
    }

    fn entity_active(&self, id: &str) -> bool {
        self.entities
            .get(id)
            .is_some_and(|entity| entity.alive_at(&self.at))
    }

    fn active(&self, node: &NodeRef) -> bool {
        match node.kind {
            NodeKind::Decision => self.decision_active(&node.id),
            NodeKind::Claim => self.claim_active(&node.id),
            NodeKind::Entity => self.entity_active(&node.id),
        }
    }

    fn summary(&self, node: &NodeRef) -> Option<NodeSummary> {
        let (label, detail, at) = match node.kind {
            NodeKind::Decision => {
                let decision = self.decisions.get(&node.id)?;
                (
                    decision.question.clone(),
                    decision.choice.clone(),
                    decision.confirmed_at.clone(),
                )
            }
            NodeKind::Claim => {
                let claim = self.claims.get(&node.id)?;
                (
                    claim.statement.clone(),
                    claim.kind.as_str().to_string(),
                    claim.valid_from.clone(),
                )
            }
            NodeKind::Entity => {
                let entity = self.entities.get(&node.id)?;
                (
                    entity.name.clone(),
                    entity.kind.as_str().to_string(),
                    entity.created_at.clone(),
                )
            }
        };
        Some(NodeSummary {
            node: node.clone(),
            label,
            detail,
            at,
        })
    }

    /// Edges holding at `at`.
    fn holding(&self) -> impl Iterator<Item = &EdgeRecord> {
        self.edges.iter().filter(|edge| edge.holds_at(&self.at))
    }

    /// Decisions (in force) and claims (valid) tied to `entity_id` by holding edges.
    fn tied(&self, entity_id: &str) -> (Vec<String>, Vec<String>) {
        let mut decisions = BTreeSet::new();
        let mut claims = BTreeSet::new();
        for edge in self.holding().filter(|edge| edge.entity_id == entity_id) {
            match edge.source_kind {
                NodeKind::Decision if self.decision_active(&edge.source_id) => {
                    decisions.insert(edge.source_id.clone());
                }
                NodeKind::Claim if self.claim_active(&edge.source_id) => {
                    claims.insert(edge.source_id.clone());
                }
                _ => {}
            }
        }
        (
            decisions.into_iter().collect(),
            claims.into_iter().collect(),
        )
    }

    fn parent(&self, entity_id: &str) -> Option<String> {
        self.holding()
            .find(|edge| {
                edge.kind == EdgeKind::PartOf
                    && edge.source_id == entity_id
                    && self.entity_active(&edge.entity_id)
            })
            .map(|edge| edge.entity_id.clone())
    }

    fn parts(&self, entity_id: &str) -> Vec<String> {
        self.holding()
            .filter(|edge| {
                edge.kind == EdgeKind::PartOf
                    && edge.entity_id == entity_id
                    && self.entity_active(&edge.source_id)
            })
            .map(|edge| edge.source_id.clone())
            .collect()
    }

    fn relation_holds(&self, row: &RelationRow) -> bool {
        before_or_at(&row.created_at, &self.at)
    }

    /// Conflicting pairs among `decisions`.
    fn conflicts(&self, decisions: &[String]) -> Vec<(String, String)> {
        let set: BTreeSet<&String> = decisions.iter().collect();
        self.relations
            .iter()
            .filter(|row| {
                row.kind == RelationKind::ConflictsWith.as_str() && self.relation_holds(row)
            })
            .filter(|row| set.contains(&row.from) && set.contains(&row.to))
            .map(|row| (row.from.clone(), row.to.clone()))
            .collect()
    }

    /// Decisions in force that depend, transitively, on `seeds`.
    fn dependents(&self, seeds: &[String]) -> Vec<String> {
        let mut reached: BTreeSet<String> = BTreeSet::new();
        let mut queue: VecDeque<String> = seeds.iter().cloned().collect();
        while let Some(current) = queue.pop_front() {
            for row in self.relations.iter().filter(|row| {
                row.kind == RelationKind::DependsOn.as_str()
                    && row.to == current
                    && self.relation_holds(row)
            }) {
                if !seeds.contains(&row.from) && reached.insert(row.from.clone()) {
                    queue.push_back(row.from.clone());
                }
            }
        }
        reached
            .into_iter()
            .filter(|id| self.decision_active(id))
            .collect()
    }

    fn summaries(&self, ids: &[String], kind: NodeKind) -> Vec<NodeSummary> {
        let mut rows: Vec<NodeSummary> = ids
            .iter()
            .filter_map(|id| {
                self.summary(&NodeRef {
                    kind,
                    id: id.clone(),
                })
            })
            .collect();
        rows.sort_by(|left, right| right.at.cmp(&left.at).then(left.label.cmp(&right.label)));
        rows
    }

    /// Decisions in force by top-level component; one pass over the edges.
    fn decision_parts(&self) -> DecisionParts {
        let components: BTreeSet<&str> = self
            .entities
            .values()
            .filter(|entity| entity.kind == EntityKind::Component && entity.alive_at(&self.at))
            .map(|entity| entity.entity_id.as_str())
            .collect();
        let parent: BTreeMap<&str, &str> = self
            .holding()
            .filter(|edge| edge.kind == EdgeKind::PartOf)
            .filter(|edge| components.contains(edge.entity_id.as_str()))
            .map(|edge| (edge.source_id.as_str(), edge.entity_id.as_str()))
            .collect();
        let top = |id: &str| -> Option<&str> {
            let mut at = *components.get(id)?;
            for _ in 0..MAX_PART_DEPTH {
                match parent.get(at) {
                    Some(next) => at = next,
                    None => break,
                }
            }
            Some(at)
        };
        let mut grouped: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
        for edge in self.holding() {
            if edge.source_kind != NodeKind::Decision || !self.decision_active(&edge.source_id) {
                continue;
            }
            if let Some(part) = top(&edge.entity_id) {
                grouped
                    .entry(part)
                    .or_default()
                    .insert(edge.source_id.clone());
            }
        }
        let mut parts: Vec<PartCount> = components
            .iter()
            .filter(|id| top(id) == Some(**id))
            .filter_map(|id| self.entities.get(*id))
            .map(|entity| PartCount {
                entity_id: entity.entity_id.clone(),
                name: entity.name.clone(),
                decisions: grouped
                    .get(entity.entity_id.as_str())
                    .map_or(0, BTreeSet::len),
            })
            .collect();
        parts.sort_by(|left, right| {
            (left.name.to_lowercase(), &left.entity_id)
                .cmp(&(right.name.to_lowercase(), &right.entity_id))
        });
        let decisions = grouped
            .into_iter()
            .map(|(part, ids)| {
                let ids: Vec<String> = ids.into_iter().collect();
                let newest_first = self
                    .summaries(&ids, NodeKind::Decision)
                    .into_iter()
                    .map(|summary| summary.node.id)
                    .collect();
                (part.to_owned(), newest_first)
            })
            .collect();
        DecisionParts { parts, decisions }
    }

    /// Edges touching `node` at `at`: graph edges and decision relations.
    fn adjacent(&self, node: &NodeRef) -> Vec<GraphEdge> {
        let mut edges: Vec<GraphEdge> = self
            .holding()
            .filter_map(|edge| {
                let source = NodeRef {
                    kind: edge.source_kind,
                    id: edge.source_id.clone(),
                };
                let target = NodeRef::entity(edge.entity_id.clone());
                (source == *node || target == *node).then(|| GraphEdge {
                    from: source,
                    to: target,
                    kind: edge.kind.as_str().to_string(),
                })
            })
            .collect();
        if node.kind == NodeKind::Decision {
            edges.extend(
                self.relations
                    .iter()
                    .filter(|row| {
                        self.relation_holds(row) && (row.from == node.id || row.to == node.id)
                    })
                    .map(|row| GraphEdge {
                        from: NodeRef::decision(row.from.clone()),
                        to: NodeRef::decision(row.to.clone()),
                        kind: row.kind.clone(),
                    }),
            );
        }
        edges
    }
}

impl<S> KnowledgeGraph<S>
where
    S: GraphStore + RelationStore + ClaimStore + ProjectRepository,
{
    fn snapshot(&self, project_id: &str, as_of: Option<&str>) -> Result<Snapshot, GraphError> {
        let at = resolve_as_of(as_of)?;
        ProjectRepository::get(&self.store, project_id)?.ok_or(GraphError::ProjectNotFound)?;
        Ok(Snapshot {
            at,
            entities: self
                .store
                .project_entities(project_id)?
                .into_iter()
                .map(|entity| (entity.entity_id.clone(), entity))
                .collect(),
            edges: self.store.project_edges(project_id)?,
            decisions: self
                .store
                .project_decisions(project_id)?
                .into_iter()
                .map(|decision| (decision.decision_id.clone(), decision))
                .collect(),
            claims: self
                .store
                .project_claims(project_id)?
                .into_iter()
                .map(|claim| (claim.claim_id.clone(), claim))
                .collect(),
            relations: self.store.project_relations(project_id)?,
        })
    }

    /// Every entity of the project, retired ones included, by kind and name.
    ///
    /// # Errors
    ///
    /// `storage` on failure.
    pub fn entities(&self, project_id: &str) -> Result<Vec<EntityRecord>, GraphError> {
        let mut entities = self.store.project_entities(project_id)?;
        entities.sort_by(|left, right| {
            (left.kind, left.name.to_lowercase()).cmp(&(right.kind, right.name.to_lowercase()))
        });
        Ok(entities)
    }

    /// Components and technologies alive at `as_of`, with their weight.
    ///
    /// # Errors
    ///
    /// `project_not_found`, `invalid_request` for a bad date, `storage`.
    pub fn project_map(
        &self,
        project_id: &str,
        as_of: Option<&str>,
    ) -> Result<ProjectMap, GraphError> {
        let snapshot = self.snapshot(project_id, as_of)?;
        let mut entities: Vec<MapEntity> = snapshot
            .entities
            .values()
            .filter(|entity| entity.alive_at(&snapshot.at))
            .map(|entity| {
                let (decisions, claims) = snapshot.tied(&entity.entity_id);
                let last_activity = snapshot
                    .holding()
                    .filter(|edge| edge.entity_id == entity.entity_id)
                    .filter_map(|edge| edge.confirmed_at.clone())
                    .chain(
                        decisions
                            .iter()
                            .filter_map(|id| snapshot.decisions.get(id))
                            .map(|decision| decision.confirmed_at.clone()),
                    )
                    .max();
                MapEntity {
                    conflicts: snapshot.conflicts(&decisions).len(),
                    decisions: decisions.len(),
                    claims: claims.len(),
                    last_activity,
                    entity: entity.clone(),
                }
            })
            .collect();
        entities.sort_by(|left, right| {
            (left.entity.kind, left.entity.name.to_lowercase())
                .cmp(&(right.entity.kind, right.entity.name.to_lowercase()))
        });
        let part_of = snapshot
            .holding()
            .filter(|edge| edge.kind == EdgeKind::PartOf)
            .map(|edge| (edge.source_id.clone(), edge.entity_id.clone()))
            .collect();
        let pending = snapshot
            .edges
            .iter()
            .filter(|edge| edge.is_pending())
            .count();
        Ok(ProjectMap {
            as_of: snapshot.at.to_string(),
            entities,
            part_of,
            pending,
        })
    }

    /// The parts of the project (top-level components) with the decisions in
    /// force on each, directly or through a component inside them: what the
    /// Decisions screen narrows its list by.
    ///
    /// # Errors
    ///
    /// `project_not_found`, `invalid_request` for a bad date, `storage`.
    pub fn decision_parts(
        &self,
        project_id: &str,
        as_of: Option<&str>,
    ) -> Result<DecisionParts, GraphError> {
        Ok(self.snapshot(project_id, as_of)?.decision_parts())
    }

    /// The whole map as nodes and edges: entities alive, decisions in force,
    /// claims valid, the edges that hold between them, decision relations
    /// and pending suggestions. The desktop lays it out; nothing is derived
    /// here beyond what the map already says.
    ///
    /// # Errors
    ///
    /// `project_not_found`, `invalid_request` for a bad date, `storage`.
    pub fn project_graph(
        &self,
        project_id: &str,
        as_of: Option<&str>,
    ) -> Result<ProjectGraph, GraphError> {
        let snapshot = self.snapshot(project_id, as_of)?;
        let mut refs: Vec<NodeRef> = Vec::new();
        refs.extend(
            snapshot
                .entities
                .values()
                .filter(|entity| entity.alive_at(&snapshot.at))
                .map(|entity| NodeRef::entity(entity.entity_id.clone())),
        );
        refs.extend(
            snapshot
                .decisions
                .keys()
                .filter(|id| snapshot.decision_active(id))
                .map(|id| NodeRef::decision(id.clone())),
        );
        refs.extend(
            snapshot
                .claims
                .keys()
                .filter(|id| snapshot.claim_active(id))
                .map(|id| NodeRef::claim(id.clone())),
        );
        let present: BTreeSet<NodeRef> = refs.iter().cloned().collect();
        let nodes = refs
            .iter()
            .filter_map(|node| {
                Some(GraphNode {
                    summary: snapshot.summary(node)?,
                    depth: 0,
                    active: true,
                })
            })
            .collect();
        let edge = |record: &EdgeRecord| GraphEdge {
            from: NodeRef {
                kind: record.source_kind,
                id: record.source_id.clone(),
            },
            to: NodeRef::entity(record.entity_id.clone()),
            kind: record.kind.as_str().to_string(),
        };
        let mut edges: Vec<GraphEdge> = snapshot
            .holding()
            .map(edge)
            .filter(|edge| present.contains(&edge.from) && present.contains(&edge.to))
            .collect();
        edges.extend(
            snapshot
                .relations
                .iter()
                .filter(|row| snapshot.relation_holds(row))
                .map(|row| GraphEdge {
                    from: NodeRef::decision(row.from.clone()),
                    to: NodeRef::decision(row.to.clone()),
                    kind: row.kind.clone(),
                })
                .filter(|edge| present.contains(&edge.from) && present.contains(&edge.to)),
        );
        let suggested = snapshot
            .edges
            .iter()
            .filter(|record| record.is_pending())
            .map(|record| (record.edge_id.clone(), edge(record)))
            .filter(|(_, edge)| present.contains(&edge.from) && present.contains(&edge.to))
            .collect();
        Ok(ProjectGraph {
            nodes,
            edges,
            suggested,
        })
    }

    /// One entity with its decisions, claims, parts, conflicts and impact.
    ///
    /// # Errors
    ///
    /// `not_found`, `invalid_request` for a bad date, `storage`.
    pub fn entity_detail(
        &self,
        entity_id: &str,
        as_of: Option<&str>,
    ) -> Result<EntityDetail, GraphError> {
        let entity = self
            .store
            .get_entity(entity_id)?
            .ok_or(GraphError::NotFound)?;
        let snapshot = self.snapshot(&entity.project_id, as_of)?;
        let (decisions, claims) = snapshot.tied(entity_id);
        let conflicts = snapshot
            .conflicts(&decisions)
            .into_iter()
            .filter_map(|(left, right)| {
                Some((
                    snapshot.summary(&NodeRef::decision(left))?,
                    snapshot.summary(&NodeRef::decision(right))?,
                ))
            })
            .collect();
        let impact = snapshot.dependents(&decisions);
        Ok(EntityDetail {
            decisions: snapshot.summaries(&decisions, NodeKind::Decision),
            claims: snapshot.summaries(&claims, NodeKind::Claim),
            parent: snapshot
                .parent(entity_id)
                .and_then(|parent| snapshot.summary(&NodeRef::entity(parent))),
            parts: snapshot.summaries(&snapshot.parts(entity_id), NodeKind::Entity),
            conflicts,
            impact: snapshot.summaries(&impact, NodeKind::Decision),
            entity,
        })
    }

    /// A node and what is around it, up to `depth` hops (at most 2) and
    /// `limit` nodes.
    ///
    /// # Errors
    ///
    /// `not_found` for an unknown node, `invalid_request`, `storage`.
    pub fn neighborhood(
        &self,
        project_id: &str,
        center: &NodeRef,
        depth: usize,
        limit: usize,
        as_of: Option<&str>,
    ) -> Result<Neighborhood, GraphError> {
        let snapshot = self.snapshot(project_id, as_of)?;
        let center_summary = snapshot.summary(center).ok_or(GraphError::NotFound)?;
        let depth = depth.clamp(1, MAX_NEIGHBORHOOD_DEPTH);
        let limit = limit.max(1);
        let mut nodes = vec![GraphNode {
            active: snapshot.active(center),
            summary: center_summary,
            depth: 0,
        }];
        let mut seen: BTreeSet<NodeRef> = BTreeSet::from([center.clone()]);
        let mut edges: Vec<GraphEdge> = Vec::new();
        let mut frontier = vec![center.clone()];
        let mut truncated = false;
        for hop in 1..=depth {
            let mut next = Vec::new();
            for node in &frontier {
                for edge in snapshot.adjacent(node) {
                    let other = if edge.from == *node {
                        edge.to.clone()
                    } else {
                        edge.from.clone()
                    };
                    if !seen.contains(&other) {
                        if nodes.len() >= limit {
                            truncated = true;
                            continue;
                        }
                        let Some(summary) = snapshot.summary(&other) else {
                            continue;
                        };
                        seen.insert(other.clone());
                        nodes.push(GraphNode {
                            active: snapshot.active(&other),
                            summary,
                            depth: hop,
                        });
                        next.push(other.clone());
                    }
                    if seen.contains(&edge.from)
                        && seen.contains(&edge.to)
                        && !edges.contains(&edge)
                    {
                        edges.push(edge);
                    }
                }
            }
            frontier = next;
        }
        Ok(Neighborhood {
            nodes,
            edges,
            truncated,
        })
    }

    /// What holds for a file: components whose patterns match (and their
    /// parents), the decisions in force and the claims valid on them.
    ///
    /// # Errors
    ///
    /// `invalid_request` for an empty path or bad date, `storage`.
    pub fn file_lens(
        &self,
        project_id: &str,
        path: &str,
        as_of: Option<&str>,
    ) -> Result<FileLens, GraphError> {
        let path = normalize_path(path);
        if path.is_empty() {
            return Err(GraphError::InvalidRequest(
                "informe o caminho de um arquivo".into(),
            ));
        }
        let snapshot = self.snapshot(project_id, as_of)?;
        let components = components_for(&snapshot, std::slice::from_ref(&path));
        let (decisions, claims) = tied_to_all(&snapshot, &components);
        Ok(FileLens {
            components: snapshot.summaries(&components, NodeKind::Entity),
            decisions: snapshot.summaries(&decisions, NodeKind::Decision),
            claims: snapshot.summaries(&claims, NodeKind::Claim),
            path,
        })
    }

    /// Files the latest decisions changed, newest decision first and without
    /// repeats: real paths to offer in the file lens.
    ///
    /// # Errors
    ///
    /// `storage`.
    pub fn recent_files(&self, project_id: &str, limit: usize) -> Result<Vec<String>, GraphError> {
        let mut decisions = self.store.project_decisions(project_id)?;
        decisions.sort_by(|left, right| right.confirmed_at.cmp(&left.confirmed_at));
        let mut files: Vec<String> = Vec::new();
        for file in decisions.iter().flat_map(|decision| &decision.files) {
            if files.len() == limit {
                break;
            }
            let file = normalize_path(file);
            if !file.is_empty() && !files.contains(&file) {
                files.push(file);
            }
        }
        Ok(files)
    }

    /// Decision and claim ids that hold for any of `files`, for the Context
    /// Pack: decisions newest first.
    ///
    /// # Errors
    ///
    /// `invalid_request` for a bad date, `storage`.
    pub fn context_for_files(
        &self,
        project_id: &str,
        files: &[String],
        as_of: Option<&str>,
    ) -> Result<(Vec<String>, Vec<String>), GraphError> {
        let files: Vec<String> = files
            .iter()
            .map(|file| normalize_path(file))
            .filter(|file| !file.is_empty())
            .collect();
        if files.is_empty() {
            return Ok((Vec::new(), Vec::new()));
        }
        let snapshot = self.snapshot(project_id, as_of)?;
        Ok(files_context(&snapshot, &files))
    }

    /// What `build_pack` reads from the graph, from one load of the project:
    /// decisions tied to the files, the claims by scope of the task, and
    /// every claim of the project (so the pack does not load them
    /// again).
    pub(crate) fn pack_graph(
        &self,
        project_id: &str,
        task: &str,
        files: &[String],
        as_of: &str,
    ) -> Result<PackGraph, GraphError> {
        let snapshot = self.snapshot(project_id, Some(as_of))?;
        let project =
            ProjectRepository::get(&self.store, project_id)?.ok_or(GraphError::ProjectNotFound)?;
        let keys = super::discover::project_keys(std::path::Path::new(&project.location));
        let normalized: Vec<String> = files
            .iter()
            .map(|file| normalize_path(file))
            .filter(|file| !file.is_empty())
            .collect();
        // The claims tied to the files come through `scopes`, ranked.
        let decisions = if normalized.is_empty() {
            Vec::new()
        } else {
            files_context(&snapshot, &normalized).0
        };
        let scopes = super::scope::scopes_in(
            &snapshot.at,
            &snapshot.edges,
            snapshot.entities.values(),
            task,
            files,
            &keys,
        );
        Ok(PackGraph {
            decisions,
            scopes,
            all_claims: snapshot.claims.into_values().collect(),
        })
    }

    /// Decisions in force that depend, transitively, on a decision or on the
    /// decisions tied to an entity.
    ///
    /// # Errors
    ///
    /// `not_found`, `invalid_request`, `storage`.
    pub fn impact(
        &self,
        project_id: &str,
        node: &NodeRef,
        as_of: Option<&str>,
    ) -> Result<Vec<NodeSummary>, GraphError> {
        let snapshot = self.snapshot(project_id, as_of)?;
        snapshot.summary(node).ok_or(GraphError::NotFound)?;
        let seeds = match node.kind {
            NodeKind::Decision => vec![node.id.clone()],
            NodeKind::Entity => {
                let mut ids = vec![node.id.clone()];
                let mut index = 0;
                while index < ids.len() {
                    let parts = snapshot.parts(&ids[index]);
                    for part in parts {
                        if !ids.contains(&part) {
                            ids.push(part);
                        }
                    }
                    index += 1;
                }
                tied_to_all(&snapshot, &ids).0
            }
            NodeKind::Claim => Vec::new(),
        };
        let mut reached = snapshot.dependents(&seeds);
        if node.kind == NodeKind::Entity {
            reached.extend(seeds.into_iter().filter(|id| snapshot.decision_active(id)));
            reached.sort();
            reached.dedup();
        }
        Ok(snapshot.summaries(&reached, NodeKind::Decision))
    }

    /// Dated events of the project between `from` and `to` (inclusive),
    /// oldest first; only those touching `entity_id` when given.
    ///
    /// # Errors
    ///
    /// `invalid_request` for a bad date, `storage`.
    pub fn timeline(
        &self,
        project_id: &str,
        entity_id: Option<&str>,
        from: Option<&str>,
        to: Option<&str>,
    ) -> Result<Vec<TimelineEvent>, GraphError> {
        let snapshot = self.snapshot(project_id, to)?;
        let from = match from {
            Some(value) => Some(Timestamp::parse(value).ok_or_else(|| {
                GraphError::InvalidRequest("use a data no formato AAAA-MM-DD".into())
            })?),
            None => None,
        };
        let in_range = |when: &str| {
            Timestamp::parse(when).is_some_and(|when| {
                when <= snapshot.at && from.as_ref().is_none_or(|from| when >= *from)
            })
        };
        // With an entity, only nodes tied to it (at any time) are followed.
        let scope: Option<BTreeSet<NodeRef>> = entity_id.map(|entity| {
            let mut nodes: BTreeSet<NodeRef> = snapshot
                .edges
                .iter()
                .filter(|edge| edge.entity_id == entity && edge.confirmed_at.is_some())
                .map(|edge| NodeRef {
                    kind: edge.source_kind,
                    id: edge.source_id.clone(),
                })
                .collect();
            nodes.insert(NodeRef::entity(entity));
            nodes
        });
        let wanted = |node: &NodeRef| scope.as_ref().is_none_or(|scope| scope.contains(node));

        let mut events = Vec::new();
        let mut push = |at: &str, kind: TimelineKind, node: NodeRef, other: Option<NodeRef>| {
            if !in_range(at) || !(wanted(&node) || other.as_ref().is_some_and(wanted)) {
                return;
            }
            let Some(summary) = snapshot.summary(&node) else {
                return;
            };
            events.push(TimelineEvent {
                at: at.to_string(),
                kind,
                node: summary,
                other: other.and_then(|other| snapshot.summary(&other)),
            });
        };
        for decision in snapshot.decisions.values() {
            push(
                &decision.confirmed_at,
                TimelineKind::DecisionConfirmed,
                NodeRef::decision(decision.decision_id.clone()),
                None,
            );
        }
        for row in snapshot
            .relations
            .iter()
            .filter(|row| row.kind == RelationKind::Supersedes.as_str())
        {
            push(
                &row.created_at,
                TimelineKind::DecisionSuperseded,
                NodeRef::decision(row.to.clone()),
                Some(NodeRef::decision(row.from.clone())),
            );
        }
        for claim in snapshot.claims.values() {
            push(
                &claim.valid_from,
                TimelineKind::ClaimStarted,
                NodeRef::claim(claim.claim_id.clone()),
                None,
            );
            if let Some(until) = &claim.valid_until {
                push(
                    until,
                    TimelineKind::ClaimEnded,
                    NodeRef::claim(claim.claim_id.clone()),
                    None,
                );
            }
        }
        for entity in snapshot.entities.values() {
            push(
                &entity.created_at,
                TimelineKind::EntityCreated,
                NodeRef::entity(entity.entity_id.clone()),
                None,
            );
            if let Some(retired) = &entity.retired_at {
                push(
                    retired,
                    TimelineKind::EntityRetired,
                    NodeRef::entity(entity.entity_id.clone()),
                    None,
                );
            }
        }
        for edge in &snapshot.edges {
            let source = NodeRef {
                kind: edge.source_kind,
                id: edge.source_id.clone(),
            };
            let target = NodeRef::entity(edge.entity_id.clone());
            if let Some(confirmed) = &edge.confirmed_at {
                push(
                    confirmed,
                    TimelineKind::EdgeConfirmed,
                    source.clone(),
                    Some(target.clone()),
                );
                if let Some(invalidated) = &edge.invalidated_at {
                    push(
                        invalidated,
                        TimelineKind::EdgeInvalidated,
                        source,
                        Some(target),
                    );
                }
            }
        }
        events.sort_by(|left, right| (&left.at, left.kind).cmp(&(&right.at, right.kind)));
        Ok(events)
    }

    /// Suggestions waiting for confirmation, newest first.
    ///
    /// # Errors
    ///
    /// `project_not_found`, `storage`.
    pub fn suggestions(&self, project_id: &str) -> Result<Vec<Suggestion>, GraphError> {
        let snapshot = self.snapshot(project_id, None)?;
        let mut rows: Vec<(String, Suggestion)> = snapshot
            .edges
            .iter()
            .filter(|edge| edge.is_pending())
            .filter_map(|edge| {
                let source = snapshot.summary(&NodeRef {
                    kind: edge.source_kind,
                    id: edge.source_id.clone(),
                })?;
                let entity = snapshot.summary(&NodeRef::entity(edge.entity_id.clone()))?;
                Some((
                    edge.created_at.clone(),
                    Suggestion {
                        edge_id: edge.edge_id.clone(),
                        kind: edge.kind,
                        source,
                        entity,
                        reason: edge.reason.clone(),
                        created_at: edge.created_at.clone(),
                    },
                ))
            })
            .collect();
        rows.sort_by(|left, right| {
            right
                .0
                .cmp(&left.0)
                .then(left.1.edge_id.cmp(&right.1.edge_id))
        });
        Ok(rows.into_iter().map(|(_, suggestion)| suggestion).collect())
    }
}

/// Live components whose patterns match any of `files`, plus their parents.
fn components_for(snapshot: &Snapshot, files: &[String]) -> Vec<String> {
    let mut components: Vec<String> = snapshot
        .entities
        .values()
        .filter(|entity| entity.kind == EntityKind::Component && entity.alive_at(&snapshot.at))
        .filter(|entity| {
            files.iter().any(|file| {
                entity
                    .patterns
                    .iter()
                    .any(|pattern| pattern_matches(pattern, file))
            })
        })
        .map(|entity| entity.entity_id.clone())
        .collect();
    let mut index = 0;
    while index < components.len() {
        if let Some(parent) = snapshot.parent(&components[index]) {
            if !components.contains(&parent) {
                components.push(parent);
            }
        }
        index += 1;
    }
    components
}

/// Decisions in force and claims valid tied to any of `entities`.
fn tied_to_all(snapshot: &Snapshot, entities: &[String]) -> (Vec<String>, Vec<String>) {
    let mut decisions = BTreeSet::new();
    let mut claims = BTreeSet::new();
    for entity in entities {
        let (tied_decisions, tied_claims) = snapshot.tied(entity);
        decisions.extend(tied_decisions);
        claims.extend(tied_claims);
    }
    (
        decisions.into_iter().collect(),
        claims.into_iter().collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::entities::EdgeOrigin;

    fn component(id: &str, name: &str) -> EntityRecord {
        EntityRecord {
            entity_id: id.into(),
            project_id: "p".into(),
            kind: EntityKind::Component,
            name: name.into(),
            key: id.into(),
            description: String::new(),
            patterns: vec![],
            aliases: vec![],
            created_at: "2026-01-01T00:00:00Z".into(),
            retired_at: None,
        }
    }

    fn edge(kind: EdgeKind, source: (NodeKind, &str), target: &str) -> EdgeRecord {
        EdgeRecord {
            edge_id: format!("{}-{target}", source.1),
            project_id: "p".into(),
            kind,
            source_kind: source.0,
            source_id: source.1.into(),
            entity_id: target.into(),
            origin: EdgeOrigin::Human,
            reason: String::new(),
            created_at: "2026-01-02T00:00:00Z".into(),
            confirmed_at: Some("2026-01-02T00:00:00Z".into()),
            invalidated_at: None,
        }
    }

    fn decision(id: &str, confirmed_at: &str) -> DecisionNode {
        DecisionNode {
            decision_id: id.into(),
            question: format!("pergunta {id}"),
            choice: String::new(),
            rationale: String::new(),
            context: vec![],
            confirmed_at: confirmed_at.into(),
            files: vec![],
            diffs: vec![],
        }
    }

    /// storage (with its part inbox), app, and an empty docs; d1 and d2 on the
    /// storage (d2 through inbox), d3 on app and inbox, d4 superseded, d5 only
    /// suggested.
    fn snapshot() -> Snapshot {
        let mut technology = component("sqlite", "SQLite");
        technology.kind = EntityKind::Technology;
        let mut retired = component("old", "Antigo");
        retired.retired_at = Some("2026-02-01T00:00:00Z".into());
        let entities = [
            component("storage", "storage-sqlite"),
            component("inbox", "inbox"),
            component("app", "Application"),
            component("docs", "docs"),
            technology,
            retired,
        ];
        let mut pending = edge(EdgeKind::Affects, (NodeKind::Decision, "d5"), "app");
        pending.confirmed_at = None;
        let edges = vec![
            edge(EdgeKind::PartOf, (NodeKind::Entity, "inbox"), "storage"),
            edge(EdgeKind::Affects, (NodeKind::Decision, "d1"), "storage"),
            edge(EdgeKind::Uses, (NodeKind::Decision, "d1"), "sqlite"),
            edge(EdgeKind::Affects, (NodeKind::Decision, "d2"), "inbox"),
            edge(EdgeKind::Affects, (NodeKind::Decision, "d3"), "app"),
            edge(EdgeKind::Affects, (NodeKind::Decision, "d3"), "inbox"),
            edge(EdgeKind::Affects, (NodeKind::Decision, "d4"), "storage"),
            edge(EdgeKind::AppliesTo, (NodeKind::Claim, "c1"), "docs"),
            pending,
        ];
        let decisions = [
            decision("d1", "2026-03-01T00:00:00Z"),
            decision("d2", "2026-03-03T00:00:00Z"),
            decision("d3", "2026-03-02T00:00:00Z"),
            decision("d4", "2026-03-01T00:00:00Z"),
            decision("d5", "2026-03-01T00:00:00Z"),
        ];
        Snapshot {
            at: Timestamp::parse("2026-10-01").expect("date"),
            entities: entities
                .into_iter()
                .map(|entity| (entity.entity_id.clone(), entity))
                .collect(),
            edges,
            decisions: decisions
                .into_iter()
                .map(|decision| (decision.decision_id.clone(), decision))
                .collect(),
            claims: BTreeMap::new(),
            relations: vec![RelationRow {
                from: "d1".into(),
                to: "d4".into(),
                kind: RelationKind::Supersedes.as_str().into(),
                created_at: "2026-03-05T00:00:00Z".into(),
            }],
        }
    }

    #[test]
    fn parts_are_top_level_components_with_decisions_in_force() {
        let parts = snapshot().decision_parts();
        let listed: Vec<(&str, usize)> = parts
            .parts
            .iter()
            .map(|part| (part.entity_id.as_str(), part.decisions))
            .collect();
        // By name; a part (inbox), a technology and a retired component are
        // not parts; a part without decisions still is one.
        assert_eq!(listed, vec![("app", 1), ("docs", 0), ("storage", 3)]);
        // Through a component inside it, newest first; the superseded d4 and
        // the suggested d5 do not count.
        assert_eq!(parts.decisions_of("storage"), ["d2", "d3", "d1"]);
        assert_eq!(parts.decisions_of("app"), ["d3"]);
        assert!(parts.decisions_of("docs").is_empty());
        assert!(parts.decisions_of("nowhere").is_empty());
    }

    #[test]
    fn a_decision_knows_every_part_it_affects() {
        let parts = snapshot().decision_parts();
        let names = |id: &str| -> Vec<String> {
            parts
                .parts_of(id)
                .iter()
                .map(|part| part.name.clone())
                .collect()
        };
        assert_eq!(names("d3"), vec!["Application", "storage-sqlite"]);
        assert_eq!(names("d2"), vec!["storage-sqlite"]);
        assert!(names("d4").is_empty(), "superseded");
        assert_eq!(parts.part("docs").map(|part| part.decisions), Some(0));
    }

    #[test]
    fn a_part_of_cycle_does_not_hang() {
        let mut snapshot = snapshot();
        snapshot.edges.push(edge(
            EdgeKind::PartOf,
            (NodeKind::Entity, "storage"),
            "inbox",
        ));
        // Neither is top-level any more; the count stays finite.
        let parts = snapshot.decision_parts();
        assert!(parts.part("app").is_some());
    }
}
