//! Human-created relations between decisions, including supersession.

use domain::relations::{DecisionRelation, RelationError, RelationKind};

use crate::clock::now_rfc3339;
use crate::decisions::{DecisionStatus, DecisionStore, DecisionsError, StoredDecision};

/// A persisted relation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationRow {
    /// Source decision.
    pub from: String,
    /// Target decision.
    pub to: String,
    /// Persisted kind literal.
    pub kind: String,
    /// RFC 3339 creation time.
    pub created_at: String,
}

/// Result of inserting a relation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationInsert {
    /// The relation was stored (and the target superseded, when asked).
    Inserted,
    /// The target was no longer `accepted`; nothing was written.
    TargetNotAccepted,
    /// The same relation, or another supersession of the target, already exists.
    Duplicate,
}

/// Persistence port for decision relations.
pub trait RelationStore {
    /// Relations whose source decision belongs to `project_id`.
    fn project_relations(&self, project_id: &str) -> Result<Vec<RelationRow>, DecisionsError>;

    /// Relations where `decision_id` is the source or the target.
    fn decision_relations(&self, decision_id: &str) -> Result<Vec<RelationRow>, DecisionsError>;

    /// Stores the relation; with `supersede`, also marks the target `superseded`
    /// in the same transaction, only while it is still `accepted`.
    fn insert_relation(
        &self,
        relation: &DecisionRelation,
        created_at: &str,
        supersede: bool,
    ) -> Result<RelationInsert, DecisionsError>;
}

/// Which side of a relation a decision is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationDirection {
    /// The decision is the source (`this → other`).
    Outgoing,
    /// The decision is the target (`other → this`).
    Incoming,
}

/// A relation seen from one decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationView {
    /// Relation kind.
    pub kind: RelationKind,
    /// Side of the viewed decision.
    pub direction: RelationDirection,
    /// The other decision.
    pub other_id: String,
    /// RFC 3339 creation time.
    pub created_at: String,
}

/// Relation use case over a decision and relation store.
#[derive(Debug, Clone)]
pub struct DecisionRelations<S> {
    store: S,
}

impl<S: DecisionStore + RelationStore> DecisionRelations<S> {
    /// Wraps the store.
    pub fn new(store: S) -> Self {
        Self { store }
    }

    /// Records that `newer` replaces `older`, which becomes `superseded`.
    ///
    /// # Errors
    ///
    /// See [`DecisionRelations::relate`].
    pub fn supersede(&self, newer: &str, older: &str) -> Result<RelationView, DecisionsError> {
        self.relate(newer, older, RelationKind::Supersedes)
    }

    /// Records `from → to` between two accepted decisions of the same project.
    ///
    /// # Errors
    ///
    /// `not_found` for an unknown decision, `invalid_relation` for a relation
    /// the domain rejects or across projects, and `conflict` when a decision is
    /// no longer accepted.
    pub fn relate(
        &self,
        from: &str,
        to: &str,
        kind: RelationKind,
    ) -> Result<RelationView, DecisionsError> {
        let relation = DecisionRelation::new(from, to, kind).map_err(invalid)?;
        let source = self.accepted(relation.from())?;
        let target = self.accepted(relation.to())?;
        if source.project_id != target.project_id {
            return Err(DecisionsError::InvalidRelation(
                "as decisões pertencem a projetos diferentes".into(),
            ));
        }
        let existing = self
            .store
            .project_relations(&source.project_id)?
            .iter()
            .filter_map(to_domain)
            .collect::<Vec<_>>();
        relation.check_against(&existing).map_err(invalid)?;

        let created_at = now_rfc3339();
        let supersede = kind == RelationKind::Supersedes;
        match self
            .store
            .insert_relation(&relation, &created_at, supersede)?
        {
            RelationInsert::Inserted => Ok(RelationView {
                kind,
                direction: RelationDirection::Outgoing,
                other_id: if relation.from() == from {
                    relation.to().to_string()
                } else {
                    relation.from().to_string()
                },
                created_at,
            }),
            RelationInsert::TargetNotAccepted => Err(DecisionsError::Conflict),
            RelationInsert::Duplicate => Err(invalid(RelationError::Duplicate)),
        }
    }

    /// Relations of one decision, oldest first.
    ///
    /// # Errors
    ///
    /// `not_found` for an unknown decision, `storage` on query failure.
    pub fn of(&self, decision_id: &str) -> Result<Vec<RelationView>, DecisionsError> {
        self.store
            .get(decision_id)?
            .ok_or(DecisionsError::NotFound)?;
        let mut views: Vec<RelationView> = self
            .store
            .decision_relations(decision_id)?
            .into_iter()
            .filter_map(|row| {
                let kind = RelationKind::parse(&row.kind)?;
                let outgoing = row.from == decision_id;
                Some(RelationView {
                    kind,
                    direction: if outgoing || kind.is_symmetric() {
                        RelationDirection::Outgoing
                    } else {
                        RelationDirection::Incoming
                    },
                    other_id: if outgoing { row.to } else { row.from },
                    created_at: row.created_at,
                })
            })
            .collect();
        views.sort_by(|left, right| {
            (&left.created_at, &left.other_id).cmp(&(&right.created_at, &right.other_id))
        });
        Ok(views)
    }

    fn accepted(&self, id: &str) -> Result<StoredDecision, DecisionsError> {
        let decision = self.store.get(id)?.ok_or(DecisionsError::NotFound)?;
        if decision.status != DecisionStatus::Accepted {
            return Err(DecisionsError::Conflict);
        }
        Ok(decision)
    }
}

fn to_domain(row: &RelationRow) -> Option<DecisionRelation> {
    DecisionRelation::new(
        row.from.clone(),
        row.to.clone(),
        RelationKind::parse(&row.kind)?,
    )
    .ok()
}

fn invalid(error: RelationError) -> DecisionsError {
    DecisionsError::InvalidRelation(error.to_string())
}
