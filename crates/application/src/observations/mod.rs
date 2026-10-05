//! Frozen descriptive observation contracts; no filesystem or persistence implementation.

use serde::{Deserialize, Serialize};

pub mod reader;
pub mod refresh;

/// Maximum manifest candidates in one refresh.
pub const MAX_SOURCES: usize = 64;
/// Maximum bytes read from one manifest.
pub const MAX_SOURCE_BYTES: usize = 256 * 1024;
/// Maximum total bytes read in one refresh.
pub const MAX_REFRESH_BYTES: usize = 2 * 1024 * 1024;
/// Maximum descriptive facts emitted by one refresh.
pub const MAX_OBSERVATIONS: usize = 1024;

macro_rules! dto {
    ($(#[$meta:meta])* $name:ident { $($(#[$field_meta:meta])* $field:ident : $ty:ty),* $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
        pub struct $name { $($(#[$field_meta])* #[doc = stringify!($field)] pub $field: $ty),* }
    };
}

/// Supported manifest syntax, not an assertion about installed packages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManifestKind {
    /// Cargo.toml.
    Cargo,
    /// package.json.
    PackageJson,
}

/// Typed descriptive declaration subject.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ObservationSubject {
    /// Package declared by the source manifest.
    Package {
        /// Declared package name.
        name: String,
    },
    /// Dependency declared by the source manifest.
    Dependency {
        /// Declared dependency name.
        name: String,
    },
}

/// Dependency declaration category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyCategory {
    /// Regular dependency.
    Runtime,
    /// Development dependency.
    Development,
    /// Build dependency.
    Build,
    /// Optional dependency.
    Optional,
    /// Peer dependency.
    Peer,
}

dto! { /// Typed value; a requirement is never a resolved version.
    ObservationValue {
        declared_version: Option<String>,
        dependency_category: Option<DependencyCategory>,
        version_requirement: Option<String>,
        target: Option<String>
    }
}
dto! { /// Source identity and latest bounded check.
    ObservationSource {
        source_id: String, project_id: String, project_relative_path: String,
        manifest_kind: ManifestKind, sha256: Option<String>, parser_policy_version: String,
        last_checked_at: Option<String>, last_check_status: CheckStatus,
        #[serde(default)] semantic_cache: Option<ManifestSemanticCache>
    }
}

/// Bounded declarations needed to resolve workspace inheritance and membership.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestSemanticCache {
    /// Explicit member manifest paths; None means discovery was incomplete.
    pub members: Option<Vec<String>>,
    /// Declared workspace package version.
    pub package_version: Option<String>,
    /// Workspace dependency name and declarative requirement.
    pub dependencies: Vec<(String, Option<String>)>,
}

/// Serializable check result; unsuccessful reads do not prove absence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    /// Manifest is no longer in the explicitly declared workspace scope.
    OutOfScope,
    /// Successfully parsed.
    Verified,
    /// Known source absent.
    Missing,
    /// Read failed.
    Unreadable,
    /// Unsupported syntax.
    Unsupported,
    /// Bounded quota reached.
    QuotaExceeded,
}

/// Serializable lifecycle of a versioned observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordStatus {
    /// Supported by latest verified source.
    Current,
    /// Invalidated by a successful check or confirmed missing source.
    Invalidated,
}

dto! { /// Immutable provenance, with unknown commit represented as None.
    ObservationProvenance {
        source_id: String, source_sha256: String, parser_policy_version: String,
        #[serde(default)] supporting_sources: Vec<ObservationSupport>,
        field_pointer: String, capture_trigger: String, commit: Option<String>
    }
}
dto! { /// Exact supporting workspace declaration, not an installed version.
    ObservationSupport { source_id: String, source_sha256: String, field_pointer: String }
}
dto! { /// Stable identity with explicitly versioned descriptive content.
    ObservationRecord {
        observation_id: String, project_id: String, version: i64,
        subject: ObservationSubject, value: ObservationValue, path_scope: Vec<String>,
        provenance: ObservationProvenance, observed_at: String,
        status: RecordStatus, invalidated_at: Option<String>, invalidation_reason: Option<String>
    }
}

/// Authority must never be interpreted as a confirmed normative claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationAuthority {
    /// Parser-derived descriptive fact only.
    Descriptive,
}
dto! { /// An O-citation selected for current context, never historical inference.
    PackObservation { record: ObservationRecord, authority: ObservationAuthority }
}

/// Coverage of bounded observation selection; default means unknown, not complete.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationCoverage {
    /// Whether collection or selection omitted eligible sources or facts.
    pub partial: bool,
    /// Whether coverage is unknown (including historical requests).
    pub unknown: bool,
    /// Successfully verified sources.
    pub verified_sources: usize,
    /// Sources confirmed missing.
    pub missing_sources: usize,
    /// Sources unreadable or unsupported.
    pub failed_sources: usize,
    /// Sources omitted by quotas.
    pub quota_exceeded_sources: usize,
}

impl Default for ObservationCoverage {
    fn default() -> Self {
        Self {
            partial: false,
            unknown: true,
            verified_sources: 0,
            missing_sources: 0,
            failed_sources: 0,
            quota_exceeded_sources: 0,
        }
    }
}

impl ObservationRecord {
    /// Domain kind derived from the typed subject, avoiding contradictory tags.
    pub fn kind(&self) -> domain::observations::ObservationKind {
        match self.subject {
            ObservationSubject::Package { .. } => {
                domain::observations::ObservationKind::PackageDeclared
            }
            ObservationSubject::Dependency { .. } => {
                domain::observations::ObservationKind::DependencyDeclared
            }
        }
    }
}

impl From<CheckStatus> for domain::observations::SourceCheckStatus {
    fn from(value: CheckStatus) -> Self {
        match value {
            CheckStatus::OutOfScope => Self::OutOfScope,
            CheckStatus::Verified => Self::Verified,
            CheckStatus::Missing => Self::Missing,
            CheckStatus::Unreadable => Self::Unreadable,
            CheckStatus::Unsupported => Self::Unsupported,
            CheckStatus::QuotaExceeded => Self::QuotaExceeded,
        }
    }
}

impl From<RecordStatus> for domain::observations::ObservationStatus {
    fn from(value: RecordStatus) -> Self {
        match value {
            RecordStatus::Current => Self::Current,
            RecordStatus::Invalidated => Self::Invalidated,
        }
    }
}

dto! { /// Exact original version delivered to an agent.
    DeliveredObservation {
        observation_id: String, version: i64, source_id: String, project_id: String
    }
}
dto! { /// Invalidation of a previously delivered version, not a normative correction.
    ObservationCorrection {
        original: DeliveredObservation, invalidated_at: String, reason: String,
        replacement: Option<DeliveredObservation>
    }
}
dto! { /// Refresh trigger; generation requests coalesce per project.
    RefreshRequest { project_id: String, capture_trigger: String, requested_at: String }
}
dto! { /// Persistent dirty generation; workers must commit with generation CAS.
    RefreshGeneration { project_id: String, generation: i64, dirty: bool }
}
dto! { /// Transaction payload: source checks and complete facts for verified sources.
    RefreshBatch {
        project_id: String, expected_generation: i64, sources: Vec<ObservationSource>,
        observations: Vec<ObservationRecord>, coverage: ObservationCoverage
    }
}
dto! { /// Current snapshot; historical callers must omit these observations.
    ObservationSnapshot {
        observations: Vec<ObservationRecord>, sources: Vec<ObservationSource>,
        coverage: ObservationCoverage
    }
}

/// Atomic refresh commit result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyRefreshResult {
    /// Committed all source checks and changes atomically.
    Applied,
    /// Newer request superseded the worker; no mutation committed.
    Superseded,
    /// Adapter does not support observation refresh yet.
    Unsupported,
}

/// Diagnostic persistence/reader failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationError(pub String);

impl std::fmt::Display for ObservationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "falha de observação: {}", self.0)
    }
}
impl std::error::Error for ObservationError {}

/// Refresh persistence port; defaults are empty/unsupported, never fabricated success.
pub trait ObservationStore {
    /// Known sources for bounded refresh.
    fn current_sources(
        &self,
        _project_id: &str,
    ) -> Result<Vec<ObservationSource>, ObservationError> {
        Ok(Vec::new())
    }
    /// Candidate manifests relative to the project, bounded by limit.
    fn candidates(
        &self,
        _project_id: &str,
        _limit: usize,
    ) -> Result<Vec<ObservationSource>, ObservationError> {
        Ok(Vec::new())
    }
    /// Mark dirty and atomically increment/coalesce project generation; None means unsupported.
    fn request_refresh(
        &self,
        _request: &RefreshRequest,
    ) -> Result<Option<RefreshGeneration>, ObservationError> {
        Ok(None)
    }
    /// Atomically CAS generation and apply all source/fact invalidations.
    fn apply_refresh(&self, _batch: &RefreshBatch) -> Result<ApplyRefreshResult, ObservationError> {
        Ok(ApplyRefreshResult::Unsupported)
    }
    /// Current-only snapshot; failures/unknown coverage cannot establish absence.
    fn snapshot(&self, _project_id: &str) -> Result<ObservationSnapshot, ObservationError> {
        Ok(ObservationSnapshot {
            observations: Vec::new(),
            sources: Vec::new(),
            coverage: ObservationCoverage {
                unknown: true,
                ..Default::default()
            },
        })
    }
}

dto! { /// Reader request shared by filesystem and fake implementations.
    SourceReadRequest {
        project_id: String, project_root: String, project_relative_path: String,
        max_bytes: usize
    }
}
dto! { /// Bounded read; non-Verified statuses carry no evidentiary absence except Missing.
    SourceReadResult { status: CheckStatus, bytes: Vec<u8> }
}

/// Filesystem boundary, independent of persistence and parsing.
pub trait ObservationReader {
    /// Read a project-contained source without exceeding max_bytes.
    fn read_source(
        &self,
        request: &SourceReadRequest,
    ) -> Result<SourceReadResult, ObservationError>;
}

/// Session-scoped observation delivery history, separate from normative ItemKind.
pub trait ObservationDeliveryStore {
    /// Exact original delivered versions, isolated by session, project and delivery mode.
    fn delivered_observations(
        &self,
        _session_id: &str,
        _project_id: &str,
        _mode: crate::injection::InjectionMode,
    ) -> Result<Vec<DeliveredObservation>, ObservationError> {
        Ok(Vec::new())
    }
    /// Pending invalidations for original versions in this delivery scope.
    fn observation_corrections(
        &self,
        _session_id: &str,
        _project_id: &str,
        _mode: crate::injection::InjectionMode,
    ) -> Result<Vec<ObservationCorrection>, ObservationError> {
        Ok(Vec::new())
    }
    /// Record versions and acknowledged corrections atomically; false means unsupported.
    fn record_observation_delivery(
        &self,
        _session_id: &str,
        _project_id: &str,
        _mode: crate::injection::InjectionMode,
        _observations: &[DeliveredObservation],
        _corrections: &[ObservationCorrection],
    ) -> Result<bool, ObservationError> {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct UnsupportedStore;
    impl ObservationStore for UnsupportedStore {}
    impl ObservationDeliveryStore for UnsupportedStore {}

    #[test]
    fn defaults_never_assert_coverage_or_success() {
        let store = UnsupportedStore;
        let snapshot = store.snapshot("p").unwrap();
        assert!(snapshot.observations.is_empty());
        assert!(snapshot.coverage.unknown);
        assert_eq!(
            store
                .request_refresh(&RefreshRequest {
                    project_id: "p".into(),
                    capture_trigger: "capture".into(),
                    requested_at: "2026-10-04T00:00:00Z".into(),
                })
                .unwrap(),
            None
        );
        assert!(!store
            .record_observation_delivery(
                "s",
                "p",
                crate::injection::InjectionMode::Inject,
                &[],
                &[]
            )
            .unwrap());
    }

    #[test]
    fn correction_round_trip_preserves_original_version_and_scope() {
        let correction = ObservationCorrection {
            original: DeliveredObservation {
                observation_id: "o".into(),
                version: 3,
                source_id: "source".into(),
                project_id: "p".into(),
            },
            invalidated_at: "2026-10-04T00:00:00Z".into(),
            reason: "declaration removed".into(),
            replacement: None,
        };
        let json = serde_json::to_string(&correction).unwrap();
        assert_eq!(
            serde_json::from_str::<ObservationCorrection>(&json).unwrap(),
            correction
        );
    }

    #[test]
    fn legacy_pack_has_unknown_observation_coverage() {
        let pack: crate::context::ContextPack = serde_json::from_str(
            r#"{
            "project_id":"p","task":"t","as_of":"2026-10-04T00:00:00Z",
            "budget_chars":8000,"used_chars":0,"decisions":[],"claims":[],"omitted":0
        }"#,
        )
        .unwrap();
        assert!(pack.observations.is_empty());
        assert!(pack.observation_coverage.unknown);
    }
}
