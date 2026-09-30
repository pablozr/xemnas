//! Domain layer: the decisional core of xemnas.
#![warn(missing_docs)]

pub mod claims;
pub mod projects;
pub mod relations;
pub mod time;

pub use claims::{ClaimError, ClaimKind, ContextClaim, MAX_STATEMENT_CHARS};
pub use projects::{Project, ProjectId, ProjectLocation, ProjectSummary};
pub use relations::{DecisionRelation, RelationError, RelationKind};
pub use time::Timestamp;

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
