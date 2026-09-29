//! Application layer: use-case orchestration and the ports infrastructure implements.
//!
//! Per ARCH-001 this crate depends only on [`domain`] and defines the
//! interfaces (ports) it needs, so infrastructure can implement them without
//! the domain knowing about storage or transport. It holds the Project and Job
//! use cases and the port traits their repositories implement.
#![warn(missing_docs)]

pub mod captures;
mod clock;
pub mod jobs;
pub mod projects;

pub use captures::{
    CaptureApi, CaptureArtifactRecord, CaptureCheckpointRecord, CaptureError, CaptureIngest,
    CaptureReceiptRecord, CaptureRepository, CaptureWrite, IngestError, IngestOutcome, Receipt,
};
pub use jobs::{
    install_panic_sanitizer, job_panic_is_sanitized, write_sanitized_panic_report, JobError,
    JobEvent, JobFailure, JobOutcome, JobRecord, JobRepository, JobState, Jobs, RecoveryReport,
    WorkerHandle, ANALYZE_CAPTURE_KIND, INTERRUPTED_NON_IDEMPOTENT,
};
pub use projects::{
    canonicalize_location, ProjectError, ProjectRecord, ProjectRepository, Projects,
};

/// Returns a stable identifier for this crate, used by wiring smoke tests.
pub fn crate_name() -> &'static str {
    "application"
}

#[cfg(test)]
mod tests {
    use super::crate_name;

    #[test]
    fn smoke() {
        assert_eq!(crate_name(), "application");
    }

    #[test]
    fn depends_on_domain() {
        assert_eq!(domain::crate_name(), "domain");
    }
}
