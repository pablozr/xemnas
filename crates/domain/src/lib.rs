//! Domain layer: the decisional core of xemnas.
//!
//! This crate holds the product's core concepts (projects, capture, decisions,
//! context) and their rules. Per ARCH-001 it must not know about UI, SQLite,
//! HTTP, OpenCode or AI providers; those are reached through ports defined by
//! the application layer. Concrete domain types land in ticket 06 and beyond.
#![warn(missing_docs)]

pub mod projects;

pub use projects::{Project, ProjectId, ProjectLocation, ProjectSummary};

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
