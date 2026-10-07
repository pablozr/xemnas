//! Resolving a conflict the automatic review left for the person.
//!
//! The judge names the other side of a contradiction (see
//! [`crate::auto_approval::Conflict`]); this module shows both sides and
//! applies the person's pick through the use cases a press of Confirmar or
//! Rejeitar would run: adoption, rejection, supersession and revision.
//!
//! The other side is read again at the moment of the pick, because it may
//! have been adopted or dropped since the judge looked at it.

use std::sync::Arc;

use domain::relations::RelationKind;

use crate::adoption::AdoptionApi;
use crate::auto_approval::{Conflict, ConflictKind, Entry, ItemKind, ReviewError, Verdict};
use crate::decisions::{
    DecisionEdits, DecisionQuery, DecisionStatus, DecisionStore, Decisions, StoredDecision,
};
use crate::extract::CandidateKind;
use crate::inbox::{
    ArtifactView, CandidateDetail, CandidateEdits, CandidateStatus, Inbox, InboxStore,
    MAX_PAGE_LIMIT,
};
use crate::qualifiers::{KnowledgeQualifier, QualifierKind};
use crate::relations::{DecisionRelations, RelationStore};

/// Decisions read to find an adopted candidate among those in force.
const IN_FORCE_READ: usize = 300;

/// What one side of a conflict is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SideKind {
    /// A decision.
    Decision,
    /// A project rule.
    Rule,
}

/// One side of a conflict, as the person sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Side {
    /// Decision or rule.
    pub kind: SideKind,
    /// Candidate id, or decision id when it is in force.
    pub id: String,
    /// Whether it stands today (a decision in force) or still waits.
    pub in_force: bool,
    /// The question (a rule's subject).
    pub question: String,
    /// The choice (a rule's statement).
    pub choice: String,
    /// The file the evidence came from, when it came from one; otherwise it
    /// came from a captured session.
    pub origin: Option<String>,
    /// RFC 3339 creation moment.
    pub created_at: String,
    /// Scope lines already recorded.
    pub scope: Vec<String>,
}

/// The two sides of a conflict and why the judge left it for the person.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictView {
    /// The candidate selected in Review.
    pub this: Side,
    /// What it contradicts.
    pub other: Side,
    /// The judge's sentence.
    pub reason: String,
    /// Whether this side can replace the other: a rule cannot supersede a
    /// decision in force, which only another decision replaces.
    pub can_replace: bool,
}

/// What the person picked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    /// Keep this one; it replaces the other when that stands, and the other
    /// is rejected when it still waits.
    KeepThis,
    /// Keep the other; this one is rejected.
    KeepOther,
    /// Both stand, each with a one-line scope.
    KeepBoth {
        /// Scope of this candidate.
        this_scope: String,
        /// Scope of the other.
        other_scope: String,
    },
}

/// The other side, read again.
enum Other {
    Candidate(Box<CandidateDetail>),
    InForce(Box<StoredDecision>),
}

/// The use case over the stores and the adoption path.
pub struct Conflicts<S> {
    store: S,
    adoption: Arc<dyn AdoptionApi>,
}

fn apply(error: impl std::fmt::Display) -> ReviewError {
    ReviewError::Apply(error.to_string())
}

fn list(array: &str) -> Vec<String> {
    serde_json::from_str(array).unwrap_or_default()
}

/// The file an artifact came from, when its metadata recorded one.
fn origin_of(artifacts: &[ArtifactView]) -> Option<String> {
    artifacts.iter().find_map(|artifact| {
        let metadata: serde_json::Value = serde_json::from_str(&artifact.metadata).ok()?;
        metadata
            .get("file")
            .or_else(|| metadata.get("path"))
            .and_then(|value| value.as_str())
            .filter(|path| !path.is_empty() && *path != "unknown")
            .map(str::to_owned)
    })
}

fn candidate_side(detail: &CandidateDetail) -> Side {
    let summary = &detail.summary;
    Side {
        kind: if summary.kind == CandidateKind::Rule {
            SideKind::Rule
        } else {
            SideKind::Decision
        },
        id: summary.id.clone(),
        in_force: false,
        question: summary.question.clone(),
        choice: summary.choice.clone(),
        origin: origin_of(&detail.artifacts),
        created_at: summary.created_at.clone(),
        scope: detail
            .qualifiers
            .iter()
            .filter(|qualifier| qualifier.kind == QualifierKind::Scope)
            .map(|qualifier| qualifier.text.clone())
            .collect(),
    }
}

/// A scope line: one trimmed line, never empty.
fn scope_line(raw: &str) -> Result<String, ReviewError> {
    let line = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if line.is_empty() {
        return Err(ReviewError::Apply("escopo vazio".into()));
    }
    Ok(line)
}

impl<S> Conflicts<S>
where
    S: InboxStore + DecisionStore + RelationStore + Clone,
{
    /// Wraps the store and the adoption path.
    pub fn new(store: S, adoption: Arc<dyn AdoptionApi>) -> Self {
        Self { store, adoption }
    }

    fn inbox(&self) -> Inbox<S> {
        Inbox::new(self.store.clone())
    }

    /// The conflict of a ledger entry; `None` when the entry has none or
    /// it no longer stands (a side was resolved or dropped meanwhile).
    pub fn view(&self, entry: &Entry) -> Result<Option<ConflictView>, ReviewError> {
        let Some(conflict) = &entry.conflicts_with else {
            return Ok(None);
        };
        if entry.kind != ItemKind::Candidate
            || entry.verdict != Verdict::NeedsHuman
            || entry.undone_at.is_some()
        {
            return Ok(None);
        }
        let Ok(this) = self.pending(&entry.item_id) else {
            return Ok(None);
        };
        let Ok(other) = self.other(&this.summary.project_id, conflict) else {
            return Ok(None);
        };
        let this_side = candidate_side(&this);
        let other_side = match &other {
            Other::Candidate(detail) => candidate_side(detail),
            Other::InForce(decision) => self.decision_side(decision)?,
        };
        let can_replace = !(this_side.kind == SideKind::Rule && other_side.in_force);
        Ok(Some(ConflictView {
            this: this_side,
            other: other_side,
            reason: entry.reason.clone(),
            can_replace,
        }))
    }

    /// Applies the pick for the candidate `this_id` against `conflict`.
    pub fn resolve(
        &self,
        this_id: &str,
        conflict: &Conflict,
        resolution: &Resolution,
    ) -> Result<(), ReviewError> {
        let this = self.pending(this_id)?;
        let other = self.other(&this.summary.project_id, conflict)?;
        match resolution {
            Resolution::KeepThis => self.keep(&this, &other),
            Resolution::KeepOther => self.keep_other(&this, &other),
            Resolution::KeepBoth {
                this_scope,
                other_scope,
            } => self.keep_both(
                &this,
                &other,
                scope_line(this_scope)?,
                scope_line(other_scope)?,
            ),
        }
    }

    /// The candidate, still waiting for review.
    fn pending(&self, id: &str) -> Result<CandidateDetail, ReviewError> {
        let detail = self.inbox().detail(id).map_err(|_| ReviewError::Stale)?;
        if detail.summary.status != CandidateStatus::Pending {
            return Err(ReviewError::Stale);
        }
        Ok(detail)
    }

    /// The other side as it is now: a candidate still waiting, or a decision
    /// in force (also when the named candidate was adopted since).
    fn other(&self, project_id: &str, conflict: &Conflict) -> Result<Other, ReviewError> {
        let in_force = |decision: StoredDecision| {
            (decision.status == DecisionStatus::Accepted)
                .then(|| Other::InForce(Box::new(decision)))
                .ok_or(ReviewError::Stale)
        };
        match conflict.kind {
            ConflictKind::Decision => {
                let decision = DecisionStore::get(&self.store, &conflict.id)
                    .map_err(apply)?
                    .ok_or(ReviewError::Stale)?;
                in_force(decision)
            }
            ConflictKind::Candidate => {
                let detail = self
                    .inbox()
                    .detail(&conflict.id)
                    .map_err(|_| ReviewError::Stale)?;
                match detail.summary.status {
                    CandidateStatus::Pending => Ok(Other::Candidate(Box::new(detail))),
                    CandidateStatus::Accepted | CandidateStatus::EditedAndAccepted => {
                        let adopted = DecisionStore::list(
                            &self.store,
                            &DecisionQuery {
                                project_id: Some(project_id.to_owned()),
                                statuses: vec![DecisionStatus::Accepted],
                                limit: IN_FORCE_READ.min(MAX_PAGE_LIMIT),
                                before: None,
                            },
                        )
                        .map_err(apply)?
                        .into_iter()
                        .find(|decision| decision.candidate_id == conflict.id)
                        .ok_or(ReviewError::Stale)?;
                        in_force(adopted)
                    }
                    _ => Err(ReviewError::Stale),
                }
            }
        }
    }

    fn decision_side(&self, decision: &StoredDecision) -> Result<Side, ReviewError> {
        let detail = Decisions::new(self.store.clone())
            .detail(&decision.decision_id)
            .map_err(apply)?;
        let origin = Decisions::new(self.store.clone())
            .sources(&detail)
            .map_err(apply)?
            .into_iter()
            .filter_map(|source| source.artifact)
            .collect::<Vec<_>>();
        Ok(Side {
            kind: SideKind::Decision,
            id: decision.decision_id.clone(),
            in_force: true,
            question: decision.question.clone(),
            choice: decision.choice.clone(),
            origin: origin_of(&origin),
            created_at: decision.confirmed_at.clone(),
            scope: list(&decision.scope),
        })
    }

    /// Adopts a candidate with the ties its evidence points to, as the
    /// automatic review does; the id of what it created.
    fn adopt(
        &self,
        id: &str,
        edits: Option<CandidateEdits>,
    ) -> Result<crate::adoption::AdoptOutcome, ReviewError> {
        let links = self
            .adoption
            .preview(id)
            .map(|preview| preview.links)
            .unwrap_or_default();
        self.adoption.adopt(id, edits, &links, &[]).map_err(apply)
    }

    fn reject(&self, id: &str) -> Result<(), ReviewError> {
        self.inbox().reject(id).map_err(apply)
    }

    fn keep(&self, this: &CandidateDetail, other: &Other) -> Result<(), ReviewError> {
        let this_is_rule = this.summary.kind == CandidateKind::Rule;
        if this_is_rule && matches!(other, Other::InForce(_)) {
            return Err(ReviewError::Apply(
                "uma regra não substitui uma decisão em vigor".into(),
            ));
        }
        let adopted = self.adopt(&this.summary.id, None)?;
        match other {
            Other::Candidate(other) => self.reject(&other.summary.id),
            Other::InForce(older) => DecisionRelations::new(self.store.clone())
                .relate(&adopted.id, &older.decision_id, RelationKind::Supersedes)
                .map(|_| ())
                .map_err(apply),
        }
    }

    fn keep_other(&self, this: &CandidateDetail, other: &Other) -> Result<(), ReviewError> {
        if let Other::Candidate(other) = other {
            self.adopt(&other.summary.id, None)?;
        }
        self.reject(&this.summary.id)
    }

    fn keep_both(
        &self,
        this: &CandidateDetail,
        other: &Other,
        this_scope: String,
        other_scope: String,
    ) -> Result<(), ReviewError> {
        self.adopt(&this.summary.id, Some(scoped(this, this_scope)))?;
        match other {
            Other::Candidate(other) => {
                self.adopt(&other.summary.id, Some(scoped(other, other_scope)))?;
            }
            Other::InForce(decision) => {
                let mut scope = list(&decision.scope);
                if !scope.contains(&other_scope) {
                    scope.push(other_scope);
                }
                Decisions::new(self.store.clone())
                    .revise_version(
                        &decision.decision_id,
                        decision.version,
                        DecisionEdits {
                            scope: Some(scope),
                            ..DecisionEdits::default()
                        },
                    )
                    .map_err(apply)?;
            }
        }
        Ok(())
    }
}

/// The candidate as it stands, plus one scope the reviewer declares.
fn scoped(detail: &CandidateDetail, scope: String) -> CandidateEdits {
    let mut qualifiers = detail.qualifiers.clone();
    qualifiers.push(KnowledgeQualifier {
        kind: QualifierKind::Scope,
        text: scope,
        artifact_id: None,
    });
    CandidateEdits {
        qualifiers,
        question: detail.summary.question.clone(),
        choice: detail.summary.choice.clone(),
        rationale: detail.rationale.clone(),
    }
}
