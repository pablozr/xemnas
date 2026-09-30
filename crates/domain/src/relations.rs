//! Typed, human-created relations between Engineering Decisions.

use std::collections::{BTreeMap, BTreeSet};

/// Kind of relation from one decision to another.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RelationKind {
    /// The source replaces the target, which becomes `superseded`.
    Supersedes,
    /// The source only holds while the target holds.
    DependsOn,
    /// The two decisions cannot both hold.
    ConflictsWith,
}

impl RelationKind {
    /// Every kind, in a stable order.
    pub const ALL: [Self; 3] = [Self::Supersedes, Self::DependsOn, Self::ConflictsWith];

    /// Persisted literal.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Supersedes => "supersedes",
            Self::DependsOn => "depends_on",
            Self::ConflictsWith => "conflicts_with",
        }
    }

    /// Parses a persisted literal.
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == value)
    }

    /// Whether `a → b` means the same as `b → a`.
    pub fn is_symmetric(&self) -> bool {
        matches!(self, Self::ConflictsWith)
    }
}

/// Why a relation is not acceptable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationError {
    /// A decision identifier is blank.
    EmptyId,
    /// A decision cannot relate to itself.
    SelfReference,
    /// The same relation already exists.
    Duplicate,
    /// The relation would close a `supersedes` or `depends_on` cycle.
    Cycle,
}

impl RelationError {
    /// Short, stable code.
    pub fn code(&self) -> &'static str {
        match self {
            Self::EmptyId => "empty_id",
            Self::SelfReference => "self_reference",
            Self::Duplicate => "duplicate",
            Self::Cycle => "cycle",
        }
    }
}

impl std::fmt::Display for RelationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::EmptyId => "a relação exige duas decisões",
            Self::SelfReference => "uma decisão não pode se relacionar consigo mesma",
            Self::Duplicate => "essa relação já existe",
            Self::Cycle => "a relação criaria um ciclo",
        })
    }
}

impl std::error::Error for RelationError {}

/// A directed relation `from → to`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DecisionRelation {
    from: String,
    to: String,
    kind: RelationKind,
}

impl DecisionRelation {
    /// Builds a relation; symmetric kinds are stored with the smaller id first.
    pub fn new(
        from: impl Into<String>,
        to: impl Into<String>,
        kind: RelationKind,
    ) -> Result<Self, RelationError> {
        let (mut from, mut to) = (from.into(), to.into());
        if from.trim().is_empty() || to.trim().is_empty() {
            return Err(RelationError::EmptyId);
        }
        if from == to {
            return Err(RelationError::SelfReference);
        }
        if kind.is_symmetric() && to < from {
            std::mem::swap(&mut from, &mut to);
        }
        Ok(Self { from, to, kind })
    }

    /// Source decision.
    pub fn from(&self) -> &str {
        &self.from
    }

    /// Target decision.
    pub fn to(&self) -> &str {
        &self.to
    }

    /// Relation kind.
    pub fn kind(&self) -> RelationKind {
        self.kind
    }

    /// Checks `self` against the relations that already exist.
    pub fn check_against(&self, existing: &[DecisionRelation]) -> Result<(), RelationError> {
        if existing.contains(self) {
            return Err(RelationError::Duplicate);
        }
        if !self.kind.is_symmetric() && reaches(existing, self.kind, &self.to, &self.from) {
            return Err(RelationError::Cycle);
        }
        Ok(())
    }
}

/// Whether `start` already reaches `goal` through relations of `kind`.
fn reaches(existing: &[DecisionRelation], kind: RelationKind, start: &str, goal: &str) -> bool {
    let mut edges: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for relation in existing.iter().filter(|relation| relation.kind == kind) {
        edges
            .entry(relation.from.as_str())
            .or_default()
            .push(relation.to.as_str());
    }
    let mut seen = BTreeSet::new();
    let mut stack = vec![start];
    while let Some(node) = stack.pop() {
        if node == goal {
            return true;
        }
        if seen.insert(node) {
            stack.extend(edges.get(node).into_iter().flatten().copied());
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::{DecisionRelation, RelationError, RelationKind};

    fn relation(from: &str, to: &str, kind: RelationKind) -> DecisionRelation {
        DecisionRelation::new(from, to, kind).expect("valid relation")
    }

    #[test]
    fn literals_round_trip() {
        for kind in RelationKind::ALL {
            assert_eq!(RelationKind::parse(kind.as_str()), Some(kind));
        }
        assert_eq!(RelationKind::parse("replaces"), None);
    }

    #[test]
    fn rejects_blank_and_self_relations() {
        assert_eq!(
            DecisionRelation::new(" ", "b", RelationKind::DependsOn),
            Err(RelationError::EmptyId)
        );
        assert_eq!(
            DecisionRelation::new("a", "a", RelationKind::Supersedes),
            Err(RelationError::SelfReference)
        );
    }

    #[test]
    fn symmetric_relations_are_normalized() {
        let forward = relation("b", "a", RelationKind::ConflictsWith);
        assert_eq!((forward.from(), forward.to()), ("a", "b"));
        assert_eq!(
            forward.check_against(&[relation("a", "b", RelationKind::ConflictsWith)]),
            Err(RelationError::Duplicate)
        );
        let directed = relation("b", "a", RelationKind::DependsOn);
        assert_eq!((directed.from(), directed.to()), ("b", "a"));
    }

    #[test]
    fn detects_cycles_per_kind() {
        let existing = vec![
            relation("c", "b", RelationKind::Supersedes),
            relation("b", "a", RelationKind::Supersedes),
        ];
        assert_eq!(
            relation("a", "c", RelationKind::Supersedes).check_against(&existing),
            Err(RelationError::Cycle)
        );
        assert_eq!(
            relation("a", "c", RelationKind::DependsOn).check_against(&existing),
            Ok(()),
            "a cycle only counts inside the same kind"
        );
        assert_eq!(
            relation("d", "c", RelationKind::Supersedes).check_against(&existing),
            Ok(())
        );
    }

    #[test]
    fn duplicate_directed_relation_is_rejected() {
        let existing = vec![relation("b", "a", RelationKind::DependsOn)];
        assert_eq!(
            relation("b", "a", RelationKind::DependsOn).check_against(&existing),
            Err(RelationError::Duplicate)
        );
    }
}
