//! SQLite persistence: infrastructure that implements the application's storage ports.
//!
//! Per ARCH-001 infrastructure implements ports defined by the application.
//! This crate owns the embedded migrations, the single shared `rusqlite`
//! connection and the [`SqliteStore`] that fulfils both the Project and Job
//! repository ports through one write queue (spec §9).
//!
//! Embedded migrations (0001..0006) run forward-only when the database opens,
//! covering projects, jobs, captures, adapter checkpoints, the decision
//! candidates produced by extraction and the extraction assessments recorded for
//! provenance.
#![warn(missing_docs)]

mod assessments;
mod captures;
mod decisions;
mod diagnostics;
mod extraction;
mod inbox;
mod jobs;
mod projects;
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
