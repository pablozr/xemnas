//! Reviewing what the machine linked before, without AI. A link the rules, the
//! AI judge or an inheritance made is a guess; when the text it rested on no
//! longer supports it under the current rules (the part is negated in the
//! text, is the product's own name, or is only a segment of another path) or
//! the tie it was inherited from is gone, it is invalidated. A link a person
//! confirmed is never touched, and neither is one the AI proposed (its
//! quote is the AI's own judgement, not a rule).
//!
//! It also lists the components whose folder no longer exists in the project:
//! phantoms, such as `examples/**` born from a path an ADR cited. Nothing is
//! retired here; a person decides.

use std::collections::{BTreeMap, BTreeSet};

use domain::entities::{pattern_matches, EdgeActor, EdgeKind, EntityKind, NodeKind};

use super::derive::decision_texts;
use super::mention::{mention_quote, Folded, Term};
use super::repo_files::{RepoFiles, Resolved};
use super::{DecisionNode, EdgeRecord, EntityRecord, GraphError, GraphStore, KnowledgeGraph};
use crate::claims::{ClaimRecord, ClaimStore};
use crate::documents::is_documentation_path;
use crate::jobs::JobRepository;
use crate::projects::ProjectRepository;
use crate::relations::RelationStore;

/// A live component whose folder is gone from the project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaleComponent {
    /// The entity.
    pub entity_id: String,
    /// Its name.
    pub name: String,
    /// The literal patterns whose folder does not exist.
    pub patterns: Vec<String>,
    /// The name of the component where the files its links cite resolve, when
    /// there is one: where its decisions really belong.
    pub owner: Option<String>,
}

impl<S> KnowledgeGraph<S>
where
    S: GraphStore + RelationStore + ClaimStore + ProjectRepository + JobRepository,
{
    /// Invalidates the machine's links that the current rules no longer
    /// support. Idempotent; returns how many edges it invalidated.
    ///
    /// * A link made from a mention (suggested, or confirmed by the rules or
    ///   the AI judge) whose decision no longer mentions the part
    ///   affirmatively is invalidated as `Rules`.
    /// * A tie a rule inherited from its decision, when the decision has no
    ///   confirmed live tie to that component any more, as `Inherited`.
    pub(super) fn revalidate(
        &self,
        project_id: &str,
        edges: &mut Vec<EdgeRecord>,
        decisions: &[DecisionNode],
        named: &[(&EntityRecord, EdgeKind, Vec<Term>)],
        claims: &[ClaimRecord],
    ) -> Result<usize, GraphError> {
        let guess = |edge: &EdgeRecord| {
            edge.is_live()
                && (edge.confirmed_at.is_none()
                    || matches!(
                        edge.confirmed_by,
                        Some(EdgeActor::Rules | EdgeActor::Ai | EdgeActor::Inherited)
                    ))
        };
        let by_decision: BTreeMap<&str, &DecisionNode> = decisions
            .iter()
            .map(|decision| (decision.decision_id.as_str(), decision))
            .collect();
        let mut texts: BTreeMap<&str, Vec<Folded>> = BTreeMap::new();
        let mut invalidated = 0;

        let mut stale: Vec<String> = Vec::new();
        for edge in edges.iter() {
            if !guess(edge)
                || edge.source_kind != NodeKind::Decision
                || !matches!(edge.kind, EdgeKind::Affects | EdgeKind::Uses)
                || mention_quote(&edge.reason).is_none()
            {
                continue;
            }
            let (Some(decision), Some((_, _, terms))) = (
                by_decision.get(edge.source_id.as_str()),
                named
                    .iter()
                    .find(|(entity, _, _)| entity.entity_id == edge.entity_id),
            ) else {
                continue;
            };
            let folded = texts
                .entry(decision.decision_id.as_str())
                .or_insert_with(|| decision_texts(decision));
            let supported = terms
                .iter()
                .any(|term| folded.iter().any(|text| text.mention(term).is_some()));
            if !supported {
                stale.push(edge.edge_id.clone());
            }
        }
        for edge_id in &stale {
            // Another edge may have taken this one down already (a cascade).
            if self.invalidate_as(edge_id, EdgeActor::Rules).is_ok() {
                invalidated += 1;
            }
        }
        if !stale.is_empty() {
            *edges = self.store.project_edges(project_id)?;
        }

        // Ties a rule inherited whose decision no longer holds them.
        let ties: BTreeSet<(String, String)> = Self::decision_ties(edges).into_iter().collect();
        let orphans: Vec<String> = edges
            .iter()
            .filter(|edge| {
                guess(edge)
                    && edge.kind == EdgeKind::AppliesTo
                    && edge.source_kind == NodeKind::Claim
                    && edge.confirmed_by == Some(EdgeActor::Inherited)
            })
            .filter(|edge| {
                claims
                    .iter()
                    .find(|claim| claim.claim_id == edge.source_id)
                    .and_then(|claim| claim.source_decision_id.clone())
                    .is_some_and(|decision| !ties.contains(&(decision, edge.entity_id.clone())))
            })
            .map(|edge| edge.edge_id.clone())
            .collect();
        for edge_id in &orphans {
            if self.invalidate_as(edge_id, EdgeActor::Inherited).is_ok() {
                invalidated += 1;
            }
        }
        if !orphans.is_empty() {
            *edges = self.store.project_edges(project_id)?;
        }
        Ok(invalidated)
    }
}

/// The live components that no manifest declares and whose literal folder
/// does not exist in the project, with where their links' files really are.
/// Needs a complete listing; with a cut one nothing can be said.
pub(super) fn stale_components(
    live: &[&EntityRecord],
    declared_patterns: &BTreeSet<String>,
    edges: &[EdgeRecord],
    decisions: &[DecisionNode],
    repo: &RepoFiles,
) -> Vec<StaleComponent> {
    if repo.partial() {
        return Vec::new();
    }
    let components: Vec<&&EntityRecord> = live
        .iter()
        .filter(|entity| entity.kind == EntityKind::Component)
        .collect();
    let mut found = Vec::new();
    for entity in &components {
        if entity
            .patterns
            .iter()
            .any(|pattern| declared_patterns.contains(pattern))
        {
            continue;
        }
        let missing: Vec<String> = entity
            .patterns
            .iter()
            .filter(|pattern| {
                let literal = pattern
                    .trim_end_matches("/**")
                    .trim_end_matches("/*")
                    .trim_end_matches('/');
                !literal.is_empty() && !literal.contains(['*', '?']) && !repo.has_path(literal)
            })
            .cloned()
            .collect();
        if missing.is_empty() {
            continue;
        }
        // Where the files its links cite resolve: the reasons that are paths
        // and the files of the decisions it was tied to.
        let mut votes: BTreeMap<&str, usize> = BTreeMap::new();
        for edge in edges.iter().filter(|edge| {
            edge.entity_id == entity.entity_id && edge.source_kind == NodeKind::Decision
        }) {
            let mut cited: Vec<&str> = Vec::new();
            if mention_quote(&edge.reason).is_none()
                && !edge.reason.starts_with(super::DEPENDENCY_REASON)
                && !edge.reason.starts_with(super::SYMBOL_REASON)
                && !edge.reason.is_empty()
                && super::ai_link_quote(&edge.reason).is_none()
            {
                cited.push(edge.reason.as_str());
            }
            if let Some(decision) = decisions
                .iter()
                .find(|decision| decision.decision_id == edge.source_id)
            {
                cited.extend(
                    decision
                        .files
                        .iter()
                        .filter(|file| !is_documentation_path(file))
                        .map(String::as_str),
                );
            }
            for file in cited {
                let (Resolved::Exact(path) | Resolved::Unique(path)) = repo.resolve(file) else {
                    continue;
                };
                for owner in components.iter().filter(|other| {
                    other.entity_id != entity.entity_id
                        && other
                            .patterns
                            .iter()
                            .any(|pattern| pattern_matches(pattern, &path))
                }) {
                    *votes.entry(owner.name.as_str()).or_default() += 1;
                }
            }
        }
        let owner = votes
            .into_iter()
            .max_by(|left, right| left.1.cmp(&right.1).then(right.0.cmp(left.0)))
            .map(|(name, _)| name.to_string());
        found.push(StaleComponent {
            entity_id: entity.entity_id.clone(),
            name: entity.name.clone(),
            patterns: missing,
            owner,
        });
    }
    found
}
