//! Scope of standing rules: a claim tied to components applies only when the
//! task touches one of them; a claim with no component tie has no scope
//! informed (it enters a pack only when matched or explicitly global).
//! Derived claims follow the component ties of their source decision.

use std::collections::{BTreeMap, BTreeSet};

use domain::entities::{
    normalize_path, pattern_matches, EdgeActor, EdgeKind, EdgeOrigin, EntityKind, NodeKind,
};
use domain::time::Timestamp;

use super::mention::{entity_terms, Folded};
use super::{EdgeRecord, GraphError, GraphStore, KnowledgeGraph};
use crate::claims::{ClaimRecord, ClaimStore};
use crate::clock::now_rfc3339;
use crate::projects::ProjectRepository;
use crate::relations::RelationStore;

/// Claims tied to live components, by whether the task touches one of them.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ClaimScopes {
    /// Tied to a component the task touches.
    pub in_scope: BTreeSet<String>,
    /// Tied to components, none of which the task touches.
    pub out_of_scope: BTreeSet<String>,
}

/// Reason of an edge a claim inherited from its source decision.
pub(crate) const INHERITED_REASON: &str = "herdado da decisão de origem";

/// What it takes to tell which of the components a decision is tied to the
/// text of a rule names: the entities, the project's names, the dependencies
/// the members declare and, when the text cites code, the listing.
struct NarrowContext {
    entities: Vec<super::EntityRecord>,
    keys: std::sync::Arc<BTreeSet<String>>,
    owned: Vec<super::derive::OwnedDependency>,
    root: std::path::PathBuf,
}

impl NarrowContext {
    /// Of `tied`, the components the `texts` name (by name, path, dependency
    /// or symbol); all of `tied` when they name none.
    fn narrow(&self, tied: &[String], texts: &[&str]) -> Vec<String> {
        if tied.len() < 2 {
            return tied.to_vec();
        }
        let folded: Vec<Folded> = texts.iter().map(|text| Folded::new(text)).collect();
        let mut named: BTreeSet<&str> = BTreeSet::new();
        for id in tied {
            let Some(entity) = self.entities.iter().find(|entity| entity.entity_id == *id) else {
                continue;
            };
            if entity_terms(entity, &self.keys)
                .iter()
                .any(|term| folded.iter().any(|text| text.mention(term).is_some()))
            {
                named.insert(id);
            }
        }
        for dependency in &self.owned {
            if folded
                .iter()
                .any(|text| text.mention(&dependency.term).is_some())
            {
                for (owner, _) in &dependency.owners {
                    if let Some(id) = tied.iter().find(|id| *id == owner) {
                        named.insert(id);
                    }
                }
            }
        }
        let words: Vec<String> = folded.iter().flat_map(Folded::code_words).collect();
        if !words.is_empty() {
            let listing = super::repo_files(&self.root);
            if let Some(repo) = listing.as_ref().as_ref().filter(|repo| !repo.partial()) {
                let live: Vec<&super::EntityRecord> = self
                    .entities
                    .iter()
                    .filter(|entity| entity.retired_at.is_none())
                    .collect();
                let reserved: BTreeSet<String> =
                    live.iter().flat_map(|entity| entity.keys()).collect();
                let cited = super::derive::Cited {
                    words: &words,
                    components: &live,
                    reserved: &reserved,
                    repo,
                    root: &self.root,
                };
                for word in &words {
                    if let Some((owner, _)) = super::derive::symbol_owner(&cited, word) {
                        if let Some(id) = tied.iter().find(|id| **id == owner) {
                            named.insert(id);
                        }
                    }
                }
            }
        }
        if named.is_empty() {
            tied.to_vec()
        } else {
            tied.iter()
                .filter(|id| named.contains(id.as_str()))
                .cloned()
                .collect()
        }
    }
}

impl<S> KnowledgeGraph<S>
where
    S: GraphStore + RelationStore + ClaimStore + ProjectRepository,
{
    /// Splits the claims tied to components (live `applies_to` edges) by
    /// whether the task touches one of them. The task touches a component
    /// when a file matches its patterns or the text mentions it (never
    /// through a decision of the pack: one decision would drag in every rule
    /// of its component); a component also counts as touched through its
    /// sub-components. Claims with no component tie are in neither set:
    /// their scope is not informed.
    ///
    /// One pass over the project's entities and edges, whatever the claims.
    ///
    /// # Errors
    ///
    /// `invalid_request` for a bad date, `storage`.
    pub fn claims_by_scope(
        &self,
        project_id: &str,
        task: &str,
        files: &[String],
        as_of: &str,
    ) -> Result<ClaimScopes, GraphError> {
        let at = super::resolve_as_of(Some(as_of))?;
        let project =
            ProjectRepository::get(&self.store, project_id)?.ok_or(GraphError::ProjectNotFound)?;
        let keys = super::discover::project_keys(std::path::Path::new(&project.location));
        let edges = self.store.project_edges(project_id)?;
        let entities = self.store.project_entities(project_id)?;
        Ok(scopes_in(&at, &edges, entities.iter(), task, files, &keys))
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

    /// Of the components `tied` (the ones a decision is confirmed on), the
    /// ones the `texts` of a rule name; all of them when they name none.
    pub(crate) fn narrow_scope(
        &self,
        project_id: &str,
        tied: &[String],
        texts: &[&str],
    ) -> Result<Vec<String>, GraphError> {
        if tied.len() < 2 {
            return Ok(tied.to_vec());
        }
        Ok(self.narrow_context(project_id)?.narrow(tied, texts))
    }

    fn narrow_context(&self, project_id: &str) -> Result<NarrowContext, GraphError> {
        let project =
            ProjectRepository::get(&self.store, project_id)?.ok_or(GraphError::ProjectNotFound)?;
        let root = std::path::PathBuf::from(&project.location);
        let entities = self.store.project_entities(project_id)?;
        let mut declared = super::declared_components(&root);
        let infrastructure = super::infrastructure_components(&root, &declared);
        declared.extend(infrastructure);
        let live: Vec<&super::EntityRecord> = entities
            .iter()
            .filter(|entity| entity.retired_at.is_none())
            .collect();
        let owned = Self::dependency_owners(&live, &declared);
        Ok(NarrowContext {
            keys: super::project_keys(&root),
            owned,
            entities,
            root,
        })
    }

    /// Gives each valid claim derived from a decision the `applies_to` edge
    /// to the components the decision is tied to that the rule's own text
    /// names (all of them when it names none). A row that exists in any
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
        let mut context: Option<NarrowContext> = None;
        for claim in claims.iter().filter(|claim| claim.is_valid_at(&at)) {
            let Some(decision) = claim.source_decision_id.as_deref() else {
                continue;
            };
            let wanted: Vec<&(String, String)> = ties
                .iter()
                .filter(|(source, _)| source == decision)
                .collect();
            if wanted.is_empty() {
                continue;
            }
            // The components the decision is confirmed on, whatever `ties`
            // lists, so a rule is narrowed against all of them.
            let mut tied: Vec<String> = Self::decision_ties(edges)
                .into_iter()
                .filter(|(source, _)| source == decision)
                .map(|(_, entity)| entity)
                .collect();
            tied.sort();
            tied.dedup();
            let narrowed = if tied.len() < 2 {
                tied
            } else {
                if context.is_none() {
                    context = Some(self.narrow_context(&claim.project_id)?);
                }
                context
                    .as_ref()
                    .map(|context| context.narrow(&tied, &[claim.statement.as_str()]))
                    .unwrap_or(tied)
            };
            for (_, entity_id) in wanted
                .into_iter()
                .filter(|(_, entity)| narrowed.contains(entity))
            {
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
                    confirmed_by: Some(EdgeActor::Inherited),
                    invalidated_by: None,
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

/// The task's scope over claims already loaded: see
/// [`KnowledgeGraph::claims_by_scope`].
pub(super) fn scopes_in<'a>(
    at: &Timestamp,
    edges: &[EdgeRecord],
    entities: impl Iterator<Item = &'a super::EntityRecord>,
    task: &str,
    files: &[String],
    project_keys: &BTreeSet<String>,
) -> ClaimScopes {
    let mut targets: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for edge in edges.iter().filter(|edge| edge.holds_at(at)) {
        if edge.source_kind == NodeKind::Claim && edge.kind == EdgeKind::AppliesTo {
            targets
                .entry(edge.source_id.as_str())
                .or_default()
                .insert(edge.entity_id.as_str());
        }
    }
    if targets.is_empty() {
        return ClaimScopes::default();
    }

    let components: BTreeMap<&str, &super::EntityRecord> = entities
        .filter(|entity| entity.kind == EntityKind::Component && entity.alive_at(at))
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
            || entity_terms(entity, project_keys)
                .iter()
                .any(|term| text.mention(term).is_some())
        {
            touched.insert(id);
        }
    }
    // A touched sub-component touches the components that contain it.
    let parent: BTreeMap<&str, &str> = edges
        .iter()
        .filter(|edge| edge.holds_at(at) && edge.kind == EdgeKind::PartOf)
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

    let mut scopes = ClaimScopes::default();
    for (claim, ties) in targets {
        let live: Vec<&&str> = ties
            .iter()
            .filter(|id| components.contains_key(**id))
            .collect();
        if live.is_empty() {
            continue;
        }
        let set = if live.iter().any(|id| touched.contains(**id)) {
            &mut scopes.in_scope
        } else {
            &mut scopes.out_of_scope
        };
        set.insert(claim.to_string());
    }
    scopes
}
