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

use domain::entities::{pattern_matches, EdgeActor, EdgeKind, EdgeOrigin, EntityKind, NodeKind};

use crate::claim_suggestions::CLAIM_JOB_KIND;
use crate::claims::ClaimStore;
use crate::extract::CandidateKind;
use crate::graph::{added_dependencies, GraphError, GraphStore, KnowledgeGraph, LinkRequest};
use crate::inbox::{CandidateEdits, Inbox, InboxError, InboxStore};
use crate::jobs::{JobRecord, JobRepository, JobState};
use crate::projects::ProjectRepository;
use crate::relation_suggestions::RELATION_JOB_KIND;
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

    /// Like [`AdoptionApi::adopt`] on behalf of `by`: what the automatic review
    /// adopts is not a person's word, so its ties are derived edges that name
    /// the actor that confirmed them.
    fn adopt_as(
        &self,
        candidate_id: &str,
        edits: Option<CandidateEdits>,
        kept: &[ProposedLink],
        declined: &[ProposedLink],
        by: EdgeActor,
    ) -> Result<AdoptOutcome, AdoptionError>;

    /// Adopts only the exact displayed candidate snapshot.
    fn adopt_reviewed(
        &self,
        reviewed: &crate::CandidateDetail,
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
    S: InboxStore
        + GraphStore
        + RelationStore
        + ClaimStore
        + ProjectRepository
        + JobRepository
        + Clone,
{
    /// Wraps the store.
    pub fn new(store: S) -> Self {
        Self { store }
    }

    /// Ties the candidate's own evidence points to: components whose
    /// patterns cover the files it cites (`affects`, or `applies_to` for a
    /// rule) and, for a decision, technologies its cited hunks add (`uses`).
    /// Documentation files it cites are neither tied nor listed as uncovered.
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
        // Documentation is evidence, not the part a decision affects. The
        // files must exist in the project folder; for a document, the text
        // must cite them.
        let documents = detail
            .artifacts
            .iter()
            .any(|artifact| artifact.kind == crate::documents::DOCUMENT_ARTIFACT);
        let hunks = detail
            .artifacts
            .iter()
            .any(|artifact| artifact.kind == "diff_hunk");
        let texts: Vec<crate::graph::Folded> = [
            &detail.summary.question,
            &detail.summary.choice,
            &detail.rationale,
        ]
        .into_iter()
        .map(|text| crate::graph::Folded::new(text))
        .collect();
        let repo = crate::graph::repo_files(std::path::Path::new(&detail.summary.project_location));
        let files = crate::graph::counted_files(
            &detail.diff_summary.files,
            documents && !hunks,
            &texts,
            repo.as_ref().as_ref(),
        );
        for file in &files {
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
        self.adopt_as(candidate_id, edits, kept, declined, EdgeActor::Person)
    }

    /// [`Adoption::adopt`] on behalf of `by`.
    ///
    /// # Errors
    ///
    /// As [`Adoption::adopt`].
    pub fn adopt_as(
        &self,
        candidate_id: &str,
        edits: Option<CandidateEdits>,
        kept: &[ProposedLink],
        declined: &[ProposedLink],
        by: EdgeActor,
    ) -> Result<AdoptOutcome, AdoptionError> {
        let reviewed = Inbox::new(self.store.clone())
            .detail(candidate_id)
            .map_err(AdoptionError::Inbox)?;
        self.adopt_reviewed_as(&reviewed, edits, kept, declined, by)
    }

    /// Confirms the displayed snapshot before graph writes. Graph updates retain
    /// the existing partial-failure boundary; they are not one atomic adoption.
    pub fn adopt_reviewed(
        &self,
        reviewed: &crate::CandidateDetail,
        edits: Option<CandidateEdits>,
        kept: &[ProposedLink],
        declined: &[ProposedLink],
    ) -> Result<AdoptOutcome, AdoptionError> {
        self.adopt_reviewed_as(reviewed, edits, kept, declined, EdgeActor::Person)
    }

    /// [`Adoption::adopt_reviewed`] on behalf of `by`. A person's ties are
    /// `human` edges; anyone else's are `derived` ones that keep the reason
    /// (the file or dependency) and name `by`.
    ///
    /// # Errors
    ///
    /// As [`Adoption::adopt_reviewed`].
    pub fn adopt_reviewed_as(
        &self,
        reviewed: &crate::CandidateDetail,
        edits: Option<CandidateEdits>,
        kept: &[ProposedLink],
        declined: &[ProposedLink],
        by: EdgeActor,
    ) -> Result<AdoptOutcome, AdoptionError> {
        let preview = self.preview(&reviewed.summary.id)?;
        if kept
            .iter()
            .chain(declined)
            .any(|link| !preview.links.contains(link))
        {
            return Err(AdoptionError::Inbox(InboxError::InvalidState));
        }
        let project = reviewed.summary.project_id.clone();
        // What the extractor said about the candidate, and the text the adopted
        // record will have (the edits, else the review) to check its quotes
        // against; the text is only copied when there is something to check.
        let named = InboxStore::candidate_components(&self.store, &reviewed.summary.id)
            .map_err(AdoptionError::Inbox)?;
        let texts: Vec<String> = if named.is_empty() {
            Vec::new()
        } else if let Some(edits) = &edits {
            vec![
                edits.question.clone(),
                edits.choice.clone(),
                edits.rationale.clone(),
            ]
        } else {
            vec![
                reviewed.summary.question.clone(),
                reviewed.summary.choice.clone(),
                reviewed.rationale.clone(),
            ]
        };
        let confirmed = Inbox::new(self.store.clone())
            .confirm_reviewed(reviewed, edits)
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
            let request = LinkRequest {
                kind: link.kind,
                source_kind,
                source_id: confirmed.decision_id.clone(),
                entity_id: link.entity_id.clone(),
            };
            let made = if by == EdgeActor::Person {
                graph.link(request)
            } else {
                graph.link_as(request, EdgeOrigin::Derived, &link.reason, by)
            };
            match made {
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
                        .invalidate_as(&edge.edge_id, by)
                        .map_err(AdoptionError::Graph)?;
                    suggested = suggested.saturating_sub(1);
                }
            }
        }
        suggested += self.suggest_extracted(
            &project,
            (source_kind, &confirmed.decision_id),
            named,
            &texts,
        )?;
        // A new decision may depend on, conflict with or replace earlier ones:
        // a background job looks for that without holding the adoption.
        if confirmed.rule {
            // A standing rule no file tied to the map asks the AI which
            // components it governs, like a decision does.
            let edges = self
                .store
                .project_edges(&project)
                .map_err(AdoptionError::Graph)?;
            let claim = ClaimStore::get_claim(&self.store, &confirmed.decision_id)
                .map_err(|error| AdoptionError::Graph(GraphError::Storage(error.to_string())))?;
            let at = domain::time::Timestamp::parse(&crate::clock::now_rfc3339());
            if let (Some(claim), Some(at)) = (claim, at) {
                if crate::link_suggestions::claim_needs_links(&edges, &claim, &at) {
                    crate::link_suggestions::queue_link_job(&self.store, &claim.claim_id);
                }
            }
        } else {
            let now = crate::clock::now_rfc3339();
            // Relations with earlier decisions and the context this one
            // states are looked for in the background.
            for kind in [RELATION_JOB_KIND, CLAIM_JOB_KIND] {
                let queued = JobRepository::insert(
                    &self.store,
                    &JobRecord {
                        id: uuid::Uuid::now_v7().to_string(),
                        kind: kind.to_string(),
                        payload: confirmed.decision_id.clone(),
                        state: JobState::Queued,
                        idempotent: true,
                        attempts: 0,
                        last_error: None,
                        created_at: now.clone(),
                        updated_at: now.clone(),
                    },
                );
                // They are suggestions; the adoption stands either way.
                let _ = queued;
            }
            crate::search_terms::queue_search_terms(&self.store, &project);
            // A decision no file or dependency tied to the map (it came from
            // an ADR, a spec or a conversation) asks the AI which components
            // it governs. The edges are read after the refresh above, so
            // weak mention suggestions do not count as ties.
            let edges = self
                .store
                .project_edges(&project)
                .map_err(AdoptionError::Graph)?;
            if crate::link_suggestions::needs_links(&edges, &confirmed.decision_id) {
                crate::link_suggestions::queue_link_job(&self.store, &confirmed.decision_id);
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

impl<S> Adoption<S>
where
    S: InboxStore + GraphStore + Clone,
{
    /// Turns the components the extractor named for the candidate into
    /// suggestions of the AI's kind, so the separate link call has nothing
    /// left to ask. Only components that are still live, that no row already
    /// ties to the record (a structural link, a decline of the person) and
    /// whose quote the final text still has. Returns how many were written.
    fn suggest_extracted(
        &self,
        project: &str,
        source: (NodeKind, &str),
        named: Vec<crate::extract::CandidateComponent>,
        texts: &[String],
    ) -> Result<usize, AdoptionError> {
        if named.is_empty() {
            return Ok(0);
        }
        let texts = crate::link_suggestions::normalized_texts(
            &texts.iter().map(String::as_str).collect::<Vec<_>>(),
        );
        let entities = self
            .store
            .project_entities(project)
            .map_err(AdoptionError::Graph)?;
        let named: Vec<_> = named
            .into_iter()
            .filter(|component| {
                entities.iter().any(|entity| {
                    entity.entity_id == component.entity_id
                        && entity.kind == EntityKind::Component
                        && entity.retired_at.is_none()
                }) && crate::link_suggestions::quote_matches(&texts, &component.quote)
            })
            .collect();
        if named.is_empty() {
            return Ok(0);
        }
        let kind = if source.0 == NodeKind::Claim {
            EdgeKind::AppliesTo
        } else {
            EdgeKind::Affects
        };
        let edges = self
            .store
            .project_edges(project)
            .map_err(AdoptionError::Graph)?;
        let now = crate::clock::now_rfc3339();
        let mut written = 0;
        for component in named {
            if crate::graph::edge_exists(&edges, kind, source, &component.entity_id) {
                continue;
            }
            let record = crate::graph::EdgeRecord::pending_derived(
                project,
                kind,
                source,
                &component.entity_id,
                crate::graph::ai_link_reason(&component.quote, crate::graph::EXTRACTED_LINK_WHY),
                &now,
            );
            self.store
                .insert_edge(&record)
                .map_err(AdoptionError::Graph)?;
            written += 1;
        }
        Ok(written)
    }
}

impl<S> AdoptionApi for Adoption<S>
where
    S: InboxStore
        + GraphStore
        + RelationStore
        + ClaimStore
        + ProjectRepository
        + JobRepository
        + Clone
        + Send
        + Sync,
{
    fn adopt_reviewed(
        &self,
        reviewed: &crate::CandidateDetail,
        edits: Option<CandidateEdits>,
        kept: &[ProposedLink],
        declined: &[ProposedLink],
    ) -> Result<AdoptOutcome, AdoptionError> {
        Adoption::adopt_reviewed(self, reviewed, edits, kept, declined)
    }
    fn preview(&self, candidate_id: &str) -> Result<AdoptionPreview, AdoptionError> {
        Adoption::preview(self, candidate_id)
    }

    fn adopt_as(
        &self,
        candidate_id: &str,
        edits: Option<CandidateEdits>,
        kept: &[ProposedLink],
        declined: &[ProposedLink],
        by: EdgeActor,
    ) -> Result<AdoptOutcome, AdoptionError> {
        Adoption::adopt_as(self, candidate_id, edits, kept, declined, by)
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
