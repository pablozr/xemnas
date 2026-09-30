//! Domain layer: the decisional core of xemnas.
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
