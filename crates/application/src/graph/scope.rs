//! Scope of standing rules: a claim tied to components applies only when the
//! task touches one of them; a claim with no component tie stays global.
//! Derived claims follow the component ties of their source decision.

use std::collections::{BTreeMap, BTreeSet};

use domain::entities::{
    normalize_path, pattern_matches, EdgeKind, EdgeOrigin, EntityKind, NodeKind,
};
use domain::time::Timestamp;

use super::mention::{entity_terms, Folded};
use super::{EdgeRecord, GraphError, GraphStore, KnowledgeGraph};
use crate::claims::{ClaimRecord, ClaimStore};
use crate::clock::now_rfc3339;
use crate::projects::ProjectRepository;
use crate::relations::RelationStore;

/// Reason of an edge a claim inherited from its source decision.
const INHERITED_REASON: &str = "herdado da decisão de origem";

impl<S> KnowledgeGraph<S>
where
    S: GraphStore + RelationStore + ClaimStore + ProjectRepository,
{
    /// Claims tied to components (live `applies_to` edges) none of which the
    /// task touches. The task touches a component when a file matches its
    /// patterns, the text mentions it, or a decision of the pack is tied to
    /// it; a component also counts as touched through its sub-components.
    /// Claims with no component tie are never listed: they stay global.
    ///
    /// One pass over the project's entities and edges, whatever the claims.
    ///
    /// # Errors
    ///
    /// `invalid_request` for a bad date, `storage`.
    pub fn claims_out_of_scope(
        &self,
        project_id: &str,
        task: &str,
        files: &[String],
        decision_ids: &[String],
        as_of: &str,
    ) -> Result<BTreeSet<String>, GraphError> {
        let at = super::resolve_as_of(Some(as_of))?;
        let edges = self.store.project_edges(project_id)?;
        let mut targets: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        for edge in edges.iter().filter(|edge| edge.holds_at(&at)) {
            if edge.source_kind == NodeKind::Claim && edge.kind == EdgeKind::AppliesTo {
                targets
                    .entry(edge.source_id.as_str())
                    .or_default()
                    .insert(edge.entity_id.as_str());
            }
        }
        if targets.is_empty() {
            return Ok(BTreeSet::new());
        }

        let entities = self.store.project_entities(project_id)?;
        let components: BTreeMap<&str, &super::EntityRecord> = entities
            .iter()
            .filter(|entity| entity.kind == EntityKind::Component && entity.alive_at(&at))
            .map(|entity| (entity.entity_id.as_str(), entity))
            .collect();
        // Only components some claim is tied to can decide anything.
        let tied: BTreeSet<&str> = targets
            .values()
            .flatten()
            .copied()
            .filter(|id| components.contains_key(id))
            .collect();
        let files: Vec<String> = files
            .iter()
            .map(|file| normalize_path(file))
            .filter(|file| !file.is_empty())
            .collect();
        let text = Folded::new(task);
        let mut touched: BTreeSet<&str> = BTreeSet::new();
        for id in tied {
            let entity = components[id];
            let by_file = files.iter().any(|file| {
                entity
                    .patterns
                    .iter()
                    .any(|pattern| pattern_matches(pattern, file))
            });
            if by_file
                || entity_terms(entity)
                    .iter()
                    .any(|term| text.mention(term).is_some())
            {
                touched.insert(id);
            }
        }
        for edge in edges.iter().filter(|edge| edge.holds_at(&at)) {
            if edge.source_kind == NodeKind::Decision
                && decision_ids.contains(&edge.source_id)
                && components.contains_key(edge.entity_id.as_str())
            {
                touched.insert(edge.entity_id.as_str());
            }
        }
        // A touched sub-component touches the components that contain it.
        let parent: BTreeMap<&str, &str> = edges
            .iter()
            .filter(|edge| edge.holds_at(&at) && edge.kind == EdgeKind::PartOf)
            .filter(|edge| components.contains_key(edge.entity_id.as_str()))
            .map(|edge| (edge.source_id.as_str(), edge.entity_id.as_str()))
            .collect();
        let mut climbing: Vec<&str> = touched.iter().copied().collect();
        while let Some(current) = climbing.pop() {
            if let Some(next) = parent.get(current) {
                if touched.insert(next) {
                    climbing.push(next);
                }
            }
        }

        Ok(targets
            .into_iter()
            .filter(|(_, ties)| {
                let live: Vec<&&str> = ties
                    .iter()
                    .filter(|id| components.contains_key(**id))
                    .collect();
                !live.is_empty() && !live.iter().any(|id| touched.contains(**id))
            })
            .map(|(claim, _)| claim.to_string())
            .collect())
    }

    /// Confirmed ties of decisions to components, as `(decision, component)`.
    pub(super) fn decision_ties(edges: &[EdgeRecord]) -> Vec<(String, String)> {
        edges
            .iter()
            .filter(|edge| {
                edge.source_kind == NodeKind::Decision
                    && edge.kind == EdgeKind::Affects
                    && edge.confirmed_at.is_some()
                    && edge.invalidated_at.is_none()
            })
            .map(|edge| (edge.source_id.clone(), edge.entity_id.clone()))
            .collect()
    }

    /// Gives each valid claim derived from a decision the `applies_to` edge
    /// to every component the decision is tied to. A row that exists in any
    /// state (pending, confirmed, invalidated by a person) blocks it, so a
    /// removed tie is never recreated. Returns the edges written.
    pub(super) fn inherit_claim_scope(
        &self,
        edges: &mut Vec<EdgeRecord>,
        claims: &[ClaimRecord],
        ties: &[(String, String)],
        now: &str,
    ) -> Result<usize, GraphError> {
        let Some(at) = Timestamp::parse(now) else {
            return Ok(0);
        };
        let mut written = 0;
        for claim in claims.iter().filter(|claim| claim.is_valid_at(&at)) {
            let Some(decision) = claim.source_decision_id.as_deref() else {
                continue;
            };
            for (_, entity_id) in ties.iter().filter(|(source, _)| source == decision) {
                let exists = edges.iter().any(|edge| {
                    edge.kind == EdgeKind::AppliesTo
                        && edge.source_kind == NodeKind::Claim
                        && edge.source_id == claim.claim_id
                        && edge.entity_id == *entity_id
                });
                if exists {
                    continue;
                }
                let record = EdgeRecord {
                    edge_id: uuid::Uuid::now_v7().to_string(),
                    project_id: claim.project_id.clone(),
                    kind: EdgeKind::AppliesTo,
                    source_kind: NodeKind::Claim,
                    source_id: claim.claim_id.clone(),
                    entity_id: entity_id.clone(),
                    origin: EdgeOrigin::Derived,
                    reason: INHERITED_REASON.into(),
                    created_at: now.to_string(),
                    confirmed_at: Some(now.to_string()),
                    invalidated_at: None,
                };
                self.store.insert_edge(&record)?;
                edges.push(record);
                written += 1;
            }
        }
        Ok(written)
    }

    /// Hook for an edge that was just confirmed or created confirmed: when it
    /// ties a decision to a component, the decision's claims follow.
    pub(super) fn propagate_to_claims(&self, edge: &EdgeRecord) -> Result<(), GraphError> {
        if edge.source_kind != NodeKind::Decision || edge.kind != EdgeKind::Affects {
            return Ok(());
        }
        let claims: Vec<ClaimRecord> = self
            .store
            .project_claims(&edge.project_id)?
            .into_iter()
            .filter(|claim| claim.source_decision_id.as_deref() == Some(edge.source_id.as_str()))
            .collect();
        if claims.is_empty() {
            return Ok(());
        }
        let mut edges = self.store.project_edges(&edge.project_id)?;
        let ties = [(edge.source_id.clone(), edge.entity_id.clone())];
        self.inherit_claim_scope(&mut edges, &claims, &ties, &now_rfc3339())?;
        Ok(())
    }
}
