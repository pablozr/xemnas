//! Adoption: confirming a candidate together with its place on the map.
//!
//! Before, adopting a decision created no ties: the map only proposed them
//! when someone opened the Mapa, and until then context by file missed the
//! decision. Now Revisão shows, next to the candidate, the components and
//! technologies its own evidence points to (files it cites, dependencies its
//! cited hunks add, ADR-0005); the person keeps or drops each, and adoption
//! confirms the candidate, links what was kept and derives the remaining
//! suggestions at once. Nothing is linked that the person did not see.

use std::collections::BTreeSet;

use domain::entities::{pattern_matches, EdgeKind, EntityKind, NodeKind};

use crate::claims::ClaimStore;
use crate::extract::CandidateKind;
use crate::graph::{added_dependencies, GraphError, GraphStore, KnowledgeGraph, LinkRequest};
use crate::inbox::{CandidateEdits, Inbox, InboxError, InboxStore};
use crate::projects::ProjectRepository;
use crate::relations::RelationStore;

/// A tie the candidate's evidence points to, on an entity that exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposedLink {
    /// `affects`, `uses` or `applies_to`.
    pub kind: EdgeKind,
    /// The entity.
    pub entity_id: String,
    /// Its display name.
    pub entity_name: String,
    /// Component or technology.
    pub entity_kind: EntityKind,
    /// The file or dependency that points to it.
    pub reason: String,
}

/// What adopting the candidate would put on the map.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AdoptionPreview {
    /// Ties to existing entities, components first.
    pub links: Vec<ProposedLink>,
    /// Files the evidence cites that no component covers yet.
    pub uncovered: Vec<String>,
}

/// What an adoption did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdoptOutcome {
    /// The decision (or rule) created.
    pub id: String,
    /// Whether it became a rule.
    pub rule: bool,
    /// Ties confirmed with the adoption.
    pub linked: usize,
    /// New suggestions derived right after (other decisions, other files).
    pub suggested: usize,
}

/// Failures of an adoption.
#[derive(Debug, Clone, PartialEq)]
pub enum AdoptionError {
    /// The confirmation itself failed; nothing was adopted.
    Inbox(InboxError),
    /// The candidate was adopted but the map could not be updated.
    Graph(GraphError),
}

impl std::fmt::Display for AdoptionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Inbox(error) => write!(formatter, "{error}"),
            Self::Graph(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for AdoptionError {}

/// Object-safe entry point for the desktop.
pub trait AdoptionApi: Send + Sync {
    /// What adopting `candidate_id` would put on the map.
    fn preview(&self, candidate_id: &str) -> Result<AdoptionPreview, AdoptionError>;

    /// Confirms the candidate (with optional edits), links `kept`, rejects
    /// `declined` and derives the remaining suggestions.
    fn adopt(
        &self,
        candidate_id: &str,
        edits: Option<CandidateEdits>,
        kept: &[ProposedLink],
        declined: &[ProposedLink],
    ) -> Result<AdoptOutcome, AdoptionError>;
}

/// The adoption use case.
#[derive(Debug, Clone)]
pub struct Adoption<S> {
    store: S,
}

impl<S> Adoption<S>
where
    S: InboxStore + GraphStore + RelationStore + ClaimStore + ProjectRepository + Clone,
{
    /// Wraps the store.
    pub fn new(store: S) -> Self {
        Self { store }
    }

    /// Ties the candidate's own evidence points to: components whose
    /// patterns cover the files it cites (`affects`, or `applies_to` for a
    /// rule) and, for a decision, technologies its cited hunks add (`uses`).
    ///
    /// # Errors
    ///
    /// `inbox` when the candidate cannot be read, `graph` for the map.
    pub fn preview(&self, candidate_id: &str) -> Result<AdoptionPreview, AdoptionError> {
        let detail = Inbox::new(self.store.clone())
            .detail(candidate_id)
            .map_err(AdoptionError::Inbox)?;
        let entities = self
            .store
            .project_entities(&detail.summary.project_id)
            .map_err(AdoptionError::Graph)?;
        let live: Vec<_> = entities
            .iter()
            .filter(|entity| entity.retired_at.is_none())
            .collect();
        let rule = detail.summary.kind == CandidateKind::Rule;
        let mut preview = AdoptionPreview::default();
        let mut seen: BTreeSet<(EdgeKind, String)> = BTreeSet::new();
        for file in &detail.diff_summary.files {
            let covering: Vec<_> = live
                .iter()
                .filter(|entity| entity.kind == EntityKind::Component)
                .filter(|entity| {
                    entity
                        .patterns
                        .iter()
                        .any(|pattern| pattern_matches(pattern, file))
                })
                .collect();
            if covering.is_empty() {
                if !preview.uncovered.contains(file) {
                    preview.uncovered.push(file.clone());
                }
                continue;
            }
            let kind = if rule {
                EdgeKind::AppliesTo
            } else {
                EdgeKind::Affects
            };
            for entity in covering {
                if seen.insert((kind, entity.entity_id.clone())) {
                    preview.links.push(ProposedLink {
                        kind,
                        entity_id: entity.entity_id.clone(),
                        entity_name: entity.name.clone(),
                        entity_kind: entity.kind,
                        reason: file.clone(),
                    });
                }
            }
        }
        if !rule {
            for dependency in detail
                .artifacts
                .iter()
                .filter(|artifact| artifact.kind == "diff_hunk")
                .flat_map(|artifact| added_dependencies(&artifact.content))
            {
                let key = domain::entities::entity_key(&dependency);
                let technology = live.iter().find(|entity| {
                    entity.kind == EntityKind::Technology && entity.keys().any(|known| known == key)
                });
                if let Some(technology) = technology {
                    if seen.insert((EdgeKind::Uses, technology.entity_id.clone())) {
                        preview.links.push(ProposedLink {
                            kind: EdgeKind::Uses,
                            entity_id: technology.entity_id.clone(),
                            entity_name: technology.name.clone(),
                            entity_kind: technology.kind,
                            reason: dependency,
                        });
                    }
                }
            }
        }
        Ok(preview)
    }

    /// Confirms the candidate, then links what the person kept, rejects what
    /// they declined (it is never suggested again) and derives the remaining
    /// suggestions of the project.
    ///
    /// # Errors
    ///
    /// `inbox` when the confirmation fails (nothing changed); `graph` when the
    /// candidate was adopted but the map could not be updated.
    pub fn adopt(
        &self,
        candidate_id: &str,
        edits: Option<CandidateEdits>,
        kept: &[ProposedLink],
        declined: &[ProposedLink],
    ) -> Result<AdoptOutcome, AdoptionError> {
        let project = Inbox::new(self.store.clone())
            .detail(candidate_id)
            .map_err(AdoptionError::Inbox)?
            .summary
            .project_id;
        let confirmed = Inbox::new(self.store.clone())
            .confirm(candidate_id, edits)
            .map_err(AdoptionError::Inbox)?;
        let source_kind = if confirmed.rule {
            NodeKind::Claim
        } else {
            NodeKind::Decision
        };
        let graph = KnowledgeGraph::new(self.store.clone());
        let mut linked = 0;
        for link in kept {
            let allowed = match source_kind {
                NodeKind::Claim => link.kind == EdgeKind::AppliesTo,
                _ => matches!(link.kind, EdgeKind::Affects | EdgeKind::Uses),
            };
            if !allowed {
                continue;
            }
            match graph.link(LinkRequest {
                kind: link.kind,
                source_kind,
                source_id: confirmed.decision_id.clone(),
                entity_id: link.entity_id.clone(),
            }) {
                Ok(_) => linked += 1,
                Err(GraphError::DuplicateEdge) => {}
                Err(error) => return Err(AdoptionError::Graph(error)),
            }
        }
        let mut suggested = graph
            .refresh_suggestions(&project)
            .map_err(AdoptionError::Graph)?
            .new_edges;
        // What the person saw and dropped is rejected, so the map does not
        // ask again.
        if !declined.is_empty() {
            let pending: Vec<_> = self
                .store
                .project_edges(&project)
                .map_err(AdoptionError::Graph)?
                .into_iter()
                .filter(|edge| {
                    edge.is_pending()
                        && edge.source_kind == source_kind
                        && edge.source_id == confirmed.decision_id
                })
                .collect();
            for link in declined {
                if let Some(edge) = pending
                    .iter()
                    .find(|edge| edge.kind == link.kind && edge.entity_id == link.entity_id)
                {
                    graph
                        .invalidate(&edge.edge_id)
                        .map_err(AdoptionError::Graph)?;
                    suggested = suggested.saturating_sub(1);
                }
            }
        }
        Ok(AdoptOutcome {
            id: confirmed.decision_id,
            rule: confirmed.rule,
            linked,
            suggested,
        })
    }
}

impl<S> AdoptionApi for Adoption<S>
where
    S: InboxStore
        + GraphStore
        + RelationStore
        + ClaimStore
        + ProjectRepository
        + Clone
        + Send
        + Sync,
{
    fn preview(&self, candidate_id: &str) -> Result<AdoptionPreview, AdoptionError> {
        Adoption::preview(self, candidate_id)
    }

    fn adopt(
        &self,
        candidate_id: &str,
        edits: Option<CandidateEdits>,
        kept: &[ProposedLink],
        declined: &[ProposedLink],
    ) -> Result<AdoptOutcome, AdoptionError> {
        Adoption::adopt(self, candidate_id, edits, kept, declined)
    }
}
