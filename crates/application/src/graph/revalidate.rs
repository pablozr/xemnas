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

use super::derive::{
    best_mention, cited_dependency, cited_symbol, cites_an_owner, decision_texts,
    shared_dependency, DecisionTies, OwnedDependency, DEPENDENCY_REASON, SYMBOL_REASON,
};
use super::mention::{
    code_words_of, dependency_term, doubt_of, mention_quote, CodeWord, Folded, Term,
};
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
    /// support. Idempotent; returns how many edges it invalidated and the ids of
    /// those the rules dropped for lack of support.
    ///
    /// * A link made from a mention (suggested, or confirmed by the rules or
    ///   the AI judge) whose decision no longer mentions the part
    ///   affirmatively is invalidated as `Rules`. A mention in doubt still
    ///   supports it: the word that may negate it is the judge's to read.
    /// * A link made from a cited dependency or symbol, suggested or confirmed
    ///   by the rules, is invalidated as `Rules` when the decision no longer
    ///   cites it, or cites it only near a word that may negate it and the
    ///   reason has no alarm: the derivation then writes it again as a
    ///   pending link carrying the alarm, which goes to the judge. What the
    ///   judge or a person confirmed is not touched.
    /// * A link made from a dependency two components declare is invalidated
    ///   as `Rules` once the decision is tied to one of the owners by other
    ///   evidence: that owner is what the decision is about, and the order in
    ///   which the evidence arrived must not matter.
    /// * A tie a rule inherited from its decision, when the decision has no
    ///   confirmed live tie to that component any more, as `Inherited`.
    pub(super) fn revalidate(
        &self,
        project_id: &str,
        edges: &mut Vec<EdgeRecord>,
        decisions: &[DecisionNode],
        named: &[(&EntityRecord, EdgeKind, Vec<Term>)],
        owned: &[OwnedDependency],
        claims: &[ClaimRecord],
    ) -> Result<(usize, BTreeSet<String>), GraphError> {
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
        let terms_of: BTreeMap<&str, &Vec<Term>> = named
            .iter()
            .map(|(entity, _, terms)| (entity.entity_id.as_str(), terms))
            .collect();
        let mut readings: BTreeMap<&str, Reading> = BTreeMap::new();
        let ties = DecisionTies::of(edges);
        let mut cited: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
        let mut invalidated = 0;

        let mut stale: Vec<String> = Vec::new();
        for edge in edges.iter() {
            if !guess(edge)
                || edge.source_kind != NodeKind::Decision
                || !matches!(edge.kind, EdgeKind::Affects | EdgeKind::Uses)
            {
                continue;
            }
            let mention = mention_quote(&edge.reason).is_some();
            let structural = edge.reason.starts_with(DEPENDENCY_REASON)
                || edge.reason.starts_with(SYMBOL_REASON);
            // A dependency or a symbol is only revisited while the rules own
            // it: a person's or the judge's word on it stays.
            let rules_own =
                edge.confirmed_at.is_none() || edge.confirmed_by == Some(EdgeActor::Rules);
            if !(mention || (structural && rules_own)) {
                continue;
            }
            let Some(decision) = by_decision.get(edge.source_id.as_str()) else {
                continue;
            };
            let reading = readings
                .entry(decision.decision_id.as_str())
                .or_insert_with(|| Reading::of(decision));
            let support = if mention {
                let Some(terms) = terms_of.get(edge.entity_id.as_str()) else {
                    continue;
                };
                mention_support(terms, &reading.texts)
            } else if shared_dependency(&edge.reason)
                && owners_of(owned, &edge.reason).is_some_and(|owners| {
                    let cited = cited
                        .entry(decision.decision_id.as_str())
                        .or_insert_with(|| ties.cited(edges, &decision.decision_id, edges.len()));
                    cites_an_owner(owners, cited)
                })
            {
                Support::Gone
            } else {
                structural_support(&edge.reason, reading)
            };
            // Only in doubt, with no alarm yet, and owned by the rules: it is
            // written again with the alarm, for the judge.
            let stale_edge = match support {
                Support::Gone => true,
                Support::Clean => false,
                Support::Doubtful => rules_own && doubt_of(&edge.reason).is_none(),
            };
            if stale_edge {
                stale.push(edge.edge_id.clone());
            }
        }
        let mut dropped = BTreeSet::new();
        for edge_id in &stale {
            // Another edge may have taken this one down already (a cascade).
            if self.invalidate_as(edge_id, EdgeActor::Rules).is_ok() {
                invalidated += 1;
                dropped.insert(edge_id.clone());
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
        Ok((invalidated, dropped))
    }
}

/// How the text of a decision stands behind the evidence of a link.
enum Support {
    /// The text no longer names it, or names it only to exclude it.
    Gone,
    /// The text names it with no doubt about its polarity.
    Clean,
    /// The text names it near a word that may negate it.
    Doubtful,
}

/// The owners of the dependency a reason cites, when it is still owned.
fn owners_of<'a>(owned: &'a [OwnedDependency], reason: &str) -> Option<&'a [(String, String)]> {
    let name = cited_dependency(reason)?;
    owned
        .iter()
        .find(|dependency| dependency.name == name)
        .map(|dependency| dependency.owners.as_slice())
}

/// A decision's text, ready to be read, and its code words once asked for.
struct Reading {
    texts: Vec<Folded>,
    words: std::cell::OnceCell<Vec<CodeWord>>,
}

impl Reading {
    fn of(decision: &DecisionNode) -> Self {
        Self {
            texts: decision_texts(decision),
            words: std::cell::OnceCell::new(),
        }
    }
}

fn mention_support(terms: &[Term], texts: &[Folded]) -> Support {
    if terms
        .iter()
        .any(|term| texts.iter().any(|text| text.affirms(term)))
    {
        Support::Clean
    } else if terms
        .iter()
        .any(|term| texts.iter().any(|text| text.mention(term).is_some()))
    {
        Support::Doubtful
    } else {
        Support::Gone
    }
}

/// How the text of a decision supports the dependency or the symbol a reason
/// cites. A reason that cannot be read counts as clean: it is left alone.
fn structural_support(reason: &str, reading: &Reading) -> Support {
    if let Some(name) = cited_dependency(reason) {
        let Some(term) = dependency_term(name) else {
            return Support::Clean;
        };
        return match best_mention(&reading.texts, &term) {
            None => Support::Gone,
            Some(found) if found.doubt.is_none() => Support::Clean,
            Some(_) => Support::Doubtful,
        };
    }
    let Some(word) = cited_symbol(reason) else {
        return Support::Clean;
    };
    let words = reading.words.get_or_init(|| code_words_of(&reading.texts));
    match words.iter().find(|(known, _)| known == word) {
        None => Support::Gone,
        Some((_, None)) => Support::Clean,
        Some((_, Some(_))) => Support::Doubtful,
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
