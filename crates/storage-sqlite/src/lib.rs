//! SQLite persistence: infrastructure that implements the application's storage ports.
#![warn(missing_docs)]

mod agent;
mod assessments;
mod captures;
mod claims;
mod context;
mod context_settings;
mod decisions;
mod diagnostics;
mod extraction;
mod graph;
mod inbox;
mod injections;
mod integration;
mod jobs;
mod projects;
mod relations;
mod store;

pub use store::{default_data_dir, default_db_path, SqliteStore, StorageError};

/// Returns a stable identifier for this crate, used by wiring smoke tests.
pub fn crate_name() -> &'static str {
    "storage-sqlite"
}

#[cfg(test)]
mod tests {
    use super::crate_name;

    #[test]
    fn smoke() {
        assert_eq!(crate_name(), "storage-sqlite");
    }

    #[test]
    fn depends_on_application_and_domain() {
        assert_eq!(application::crate_name(), "application");
        assert_eq!(domain::crate_name(), "domain");
    }
}
