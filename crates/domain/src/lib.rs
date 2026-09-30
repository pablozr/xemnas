//! Domain layer: the decisional core of xemnas.
#![warn(missing_docs)]

pub mod projects;
pub mod relations;

pub use projects::{Project, ProjectId, ProjectLocation, ProjectSummary};
pub use relations::{DecisionRelation, RelationError, RelationKind};

/// Returns a stable identifier for this crate, used by wiring smoke tests.
pub fn crate_name() -> &'static str {
    "domain"
}

#[cfg(test)]
mod tests {
    use super::crate_name;

    #[test]
    fn smoke() {
        assert_eq!(crate_name(), "domain");
    }
}
