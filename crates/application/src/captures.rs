//! Capture ingest use case: authorize the Project, persist the receipt, the
//! artifacts, the analysis job and the adapter checkpoint in one transaction,
//! and deduplicate replays.
//!
//! Deduplication is identity-driven: the `idempotency_key` names the event, so a
//! replay returns the stored receipt without rewriting artifacts or scheduling
//! another job. Artifacts inside one payload are deduplicated by
//! `(kind, fingerprint)`; the same fingerprint in a different capture is
//! legitimate, because those are different events. Analysis is scheduled as an
//! [`ANALYZE_CAPTURE_KIND`] job with no handler yet (ticket 12): it stays
//! `queued` by design, and the response never waits for it.

use std::collections::HashSet;

use integration_contracts::capture::{artifact_fingerprint, CaptureEnvelope};

use crate::clock::now_rfc3339;
use crate::jobs::{JobRecord, JobState, ANALYZE_CAPTURE_KIND};
use crate::projects::{canonicalize_location, ProjectRepository};

/// A persisted capture receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureReceiptRecord {
    /// Identifier of the capture (the receipt id returned to the adapter).
    pub capture_id: String,
    /// Adapter idempotency key; unique across receipts.
    pub idempotency_key: String,
    /// Normalized canonical project path the capture was accepted for.
    pub canonical_path: String,
    /// RFC 3339 timestamp of acceptance.
    pub received_at: String,
    /// Number of distinct `(kind, fingerprint)` artifacts persisted.
    pub artifact_count: i64,
}

/// A persisted capture artifact row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureArtifactRecord {
    /// Capture this artifact belongs to.
    pub capture_id: String,
    /// Adapter-provided artifact identifier (UUID v7).
    pub artifact_id: String,
    /// Persisted `snake_case` artifact kind.
    pub kind: String,
    /// Opaque artifact content.
    pub content: String,
    /// Serialized metadata object.
    pub metadata: String,
    /// Lowercase SHA-256 of the content.
    pub fingerprint: String,
}

/// The adapter high-water mark written with an accepted capture.
///
/// `(adapter, session_id)` identifies the adapter session; the row records the
/// last accepted message and capture so the adapter can resume without
/// replaying. It is upserted in the same transaction as the receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureCheckpointRecord {
    /// Adapter name, for example `opencode`.
    pub adapter: String,
    /// Adapter session identifier.
    pub session_id: String,
    /// Last accepted message identifier for the session.
    pub message_id: String,
    /// Last accepted capture for the session.
    pub capture_id: String,
    /// RFC 3339 timestamp the adapter observed for the capture.
    pub observed_at: String,
    /// RFC 3339 timestamp of this checkpoint update.
    pub updated_at: String,
}

/// Everything one capture writes in a single transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureWrite {
    /// The receipt row.
    pub receipt: CaptureReceiptRecord,
    /// The deduplicated artifact rows.
    pub artifacts: Vec<CaptureArtifactRecord>,
    /// The analysis job row.
    pub job: JobRecord,
    /// The adapter checkpoint upsert.
    pub checkpoint: CaptureCheckpointRecord,
}

/// Failure modes of capture persistence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureError {
    /// The `idempotency_key` already belongs to a receipt (unique constraint).
    DuplicateIdempotencyKey,
    /// The payload repeats an `artifact_id`; the transaction rolled back.
    DuplicateArtifact,
    /// The project was not registered at write time; the transaction rolled back.
    ProjectNotRegistered,
    /// The storage backend failed; the message is diagnostic only.
    Storage(String),
}

impl std::fmt::Display for CaptureError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateIdempotencyKey => {
                formatter.write_str("capture receipt already exists for this idempotency key")
            }
            Self::DuplicateArtifact => formatter.write_str("duplicate artifact id in capture"),
            Self::ProjectNotRegistered => formatter.write_str("capture project is not registered"),
            Self::Storage(message) => write!(formatter, "storage error: {message}"),
        }
    }
}

impl std::error::Error for CaptureError {}

/// Port that persists captures.
///
/// Infrastructure crates implement this trait; the application never knows how
/// the rows are stored. `insert_capture` is one transaction: a failure leaves no
/// receipt, artifact or job behind.
pub trait CaptureRepository {
    /// Inserts the receipt, its artifacts and the analysis job atomically.
    fn insert_capture(&self, write: &CaptureWrite) -> Result<(), CaptureError>;

    /// Returns the receipt for a capture identifier, if any.
    fn find_receipt(&self, capture_id: &str) -> Result<Option<CaptureReceiptRecord>, CaptureError>;

    /// Returns the receipt for an idempotency key, if any.
    fn find_receipt_by_idempotency_key(
        &self,
        idempotency_key: &str,
    ) -> Result<Option<CaptureReceiptRecord>, CaptureError>;
}

/// Receipt returned to the adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Receipt {
    /// Identifier of the capture.
    pub capture_id: String,
    /// Adapter idempotency key.
    pub idempotency_key: String,
    /// RFC 3339 timestamp of acceptance.
    pub received_at: String,
    /// Number of persisted artifacts.
    pub artifact_count: i64,
}

/// Outcome of an ingest call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngestOutcome {
    /// The stored (or replayed) receipt.
    pub receipt: Receipt,
    /// `true` when the idempotency key was already known.
    pub replayed: bool,
}

/// Failure modes of the ingest use case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IngestError {
    /// The project is not registered, or its path cannot be canonicalized.
    ///
    /// Both cases share one variant on purpose: the API must not leak whether a
    /// path exists (§13).
    Forbidden,
    /// The payload repeats an artifact identifier.
    Conflict,
    /// An artifact's declared fingerprint is not the SHA-256 of its content.
    InvalidFingerprint,
    /// No receipt exists for the requested capture.
    NotFound,
    /// The storage backend failed; the message is diagnostic only.
    Storage(String),
}

impl std::fmt::Display for IngestError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Forbidden => {
                formatter.write_str("o projeto não está registrado ou o caminho é inválido")
            }
            Self::Conflict => formatter.write_str("o payload repete um identificador de artefato"),
            Self::InvalidFingerprint => {
                formatter.write_str("o fingerprint de um artefato não corresponde ao conteúdo")
            }
            Self::NotFound => formatter.write_str("captura não encontrada"),
            Self::Storage(message) => write!(formatter, "falha de armazenamento: {message}"),
        }
    }
}

impl std::error::Error for IngestError {}

/// The interface the local API depends on.
///
/// Implemented by [`CaptureIngest`]; keeping it a trait lets the API test a
/// deterministic timeout with a fake that blocks.
pub trait CaptureApi: Send + Sync {
    /// Validates the project, persists the capture and schedules analysis.
    fn ingest(
        &self,
        envelope: &CaptureEnvelope,
        idempotency_key: &str,
    ) -> Result<IngestOutcome, IngestError>;

    /// Returns the receipt for a capture identifier.
    fn receipt(&self, capture_id: &str) -> Result<Receipt, IngestError>;
}

/// Concrete capture ingest use case over the Project and Capture ports.
pub struct CaptureIngest<R> {
    repository: R,
}

impl<R> CaptureIngest<R> {
    /// Wraps a repository with the ingest use case.
    pub fn new(repository: R) -> Self {
        Self { repository }
    }
}

impl<R> CaptureIngest<R>
where
    R: ProjectRepository + CaptureRepository,
{
    /// Builds the transactional write, validating the project allow-list and
    /// deduplicating the payload.
    fn build_write(
        &self,
        envelope: &CaptureEnvelope,
        idempotency_key: &str,
    ) -> Result<CaptureWrite, IngestError> {
        // §7.3 steps 3-4: canonicalize the path, then require it to be
        // registered. A failed canonicalization is the same 403 as an unknown
        // project, with no existence oracle.
        let canonical_path = canonicalize_location(&envelope.project.canonical_path)
            .map_err(|_| IngestError::Forbidden)?;
        let project = self
            .repository
            .find_by_location(&canonical_path)
            .map_err(|error| IngestError::Storage(error.to_string()))?
            .ok_or(IngestError::Forbidden)?;
        let _ = project;

        // A repeated artifact id inside one payload is a client error.
        let mut seen_ids = HashSet::new();
        for artifact in &envelope.artifacts {
            if !seen_ids.insert(artifact.artifact_id.as_str()) {
                return Err(IngestError::Conflict);
            }
        }

        // Recompute the SHA-256 of each content and require the declared
        // fingerprint to match, then deduplicate by the *computed* fingerprint.
        // Two different contents that declare the same fingerprint are rejected
        // instead of silently dropping one.
        let mut seen_pairs = HashSet::new();
        let mut artifacts = Vec::new();
        for artifact in &envelope.artifacts {
            let computed = artifact_fingerprint(&artifact.content);
            if computed != artifact.fingerprint {
                return Err(IngestError::InvalidFingerprint);
            }
            if seen_pairs.insert((artifact.kind.as_str(), computed.clone())) {
                let metadata = serde_json::to_string(&artifact.metadata)
                    .map_err(|error| IngestError::Storage(error.to_string()))?;
                artifacts.push(CaptureArtifactRecord {
                    capture_id: envelope.capture_id.clone(),
                    artifact_id: artifact.artifact_id.clone(),
                    kind: artifact.kind.as_str().to_string(),
                    content: artifact.content.clone(),
                    metadata,
                    fingerprint: computed,
                });
            }
        }
        let artifact_count = artifacts.len() as i64;

        let received_at = now_rfc3339();
        let job = JobRecord {
            id: uuid::Uuid::now_v7().to_string(),
            kind: ANALYZE_CAPTURE_KIND.to_string(),
            payload: envelope.capture_id.clone(),
            state: JobState::Queued,
            idempotent: true,
            attempts: 0,
            last_error: None,
            created_at: received_at.clone(),
            updated_at: received_at.clone(),
        };
        let checkpoint = CaptureCheckpointRecord {
            adapter: envelope.source.adapter.clone(),
            session_id: envelope.source.session_id.clone(),
            message_id: envelope.source.message_id.clone(),
            capture_id: envelope.capture_id.clone(),
            observed_at: envelope.observed_at.clone(),
            updated_at: received_at.clone(),
        };

        Ok(CaptureWrite {
            receipt: CaptureReceiptRecord {
                capture_id: envelope.capture_id.clone(),
                idempotency_key: idempotency_key.to_string(),
                canonical_path,
                received_at,
                artifact_count,
            },
            artifacts,
            job,
            checkpoint,
        })
    }
}

impl<R> CaptureApi for CaptureIngest<R>
where
    R: ProjectRepository + CaptureRepository + Send + Sync,
{
    fn ingest(
        &self,
        envelope: &CaptureEnvelope,
        idempotency_key: &str,
    ) -> Result<IngestOutcome, IngestError> {
        let write = self.build_write(envelope, idempotency_key)?;
        match self.repository.insert_capture(&write) {
            Ok(()) => Ok(IngestOutcome {
                receipt: receipt_from(&write.receipt),
                replayed: false,
            }),
            Err(CaptureError::DuplicateIdempotencyKey) => {
                // A replay returns the stored receipt unchanged; artifacts and
                // the job were rolled back with the failed transaction.
                let existing = self
                    .repository
                    .find_receipt_by_idempotency_key(idempotency_key)
                    .map_err(|error| IngestError::Storage(error.to_string()))?
                    .ok_or_else(|| {
                        IngestError::Storage(
                            "receipt missing after idempotency conflict".to_string(),
                        )
                    })?;
                Ok(IngestOutcome {
                    receipt: receipt_from(&existing),
                    replayed: true,
                })
            }
            Err(CaptureError::DuplicateArtifact) => Err(IngestError::Conflict),
            Err(CaptureError::ProjectNotRegistered) => Err(IngestError::Forbidden),
            Err(CaptureError::Storage(message)) => Err(IngestError::Storage(message)),
        }
    }

    fn receipt(&self, capture_id: &str) -> Result<Receipt, IngestError> {
        self.repository
            .find_receipt(capture_id)
            .map_err(|error| IngestError::Storage(error.to_string()))?
            .map(|record| receipt_from(&record))
            .ok_or(IngestError::NotFound)
    }
}

/// Projects a persisted receipt into its public shape.
fn receipt_from(record: &CaptureReceiptRecord) -> Receipt {
    Receipt {
        capture_id: record.capture_id.clone(),
        idempotency_key: record.idempotency_key.clone(),
        received_at: record.received_at.clone(),
        artifact_count: record.artifact_count,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CaptureApi, CaptureError, CaptureIngest, CaptureReceiptRecord, CaptureWrite, IngestError,
    };
    use crate::projects::{ProjectRecord, ProjectRepository};
    use integration_contracts::capture::{artifact_fingerprint, ArtifactKind, CaptureEnvelope};
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeRepository {
        projects: Mutex<Vec<ProjectRecord>>,
        receipts: Mutex<HashMap<String, CaptureReceiptRecord>>,
    }

    impl ProjectRepository for FakeRepository {
        fn insert(&self, record: &ProjectRecord) -> Result<(), crate::projects::ProjectError> {
            self.projects
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(record.clone());
            Ok(())
        }
        fn list(&self) -> Result<Vec<ProjectRecord>, crate::projects::ProjectError> {
            Ok(self
                .projects
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone())
        }
        fn get(&self, id: &str) -> Result<Option<ProjectRecord>, crate::projects::ProjectError> {
            Ok(self
                .projects
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .iter()
                .find(|record| record.id == id)
                .cloned())
        }
        fn find_by_location(
            &self,
            location: &str,
        ) -> Result<Option<ProjectRecord>, crate::projects::ProjectError> {
            Ok(self
                .projects
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .iter()
                .find(|record| record.location == location)
                .cloned())
        }
        fn remove(&self, _id: &str) -> Result<bool, crate::projects::ProjectError> {
            Ok(false)
        }
    }

    impl super::CaptureRepository for FakeRepository {
        fn insert_capture(&self, write: &CaptureWrite) -> Result<(), CaptureError> {
            let mut receipts = self
                .receipts
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if receipts.contains_key(&write.receipt.idempotency_key) {
                return Err(CaptureError::DuplicateIdempotencyKey);
            }
            receipts.insert(write.receipt.idempotency_key.clone(), write.receipt.clone());
            Ok(())
        }
        fn find_receipt(
            &self,
            capture_id: &str,
        ) -> Result<Option<CaptureReceiptRecord>, CaptureError> {
            Ok(self
                .receipts
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .values()
                .find(|receipt| receipt.capture_id == capture_id)
                .cloned())
        }
        fn find_receipt_by_idempotency_key(
            &self,
            idempotency_key: &str,
        ) -> Result<Option<CaptureReceiptRecord>, CaptureError> {
            Ok(self
                .receipts
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .get(idempotency_key)
                .cloned())
        }
    }

    fn project_at(location: &str) -> ProjectRecord {
        ProjectRecord::new(
            "project-1".to_string(),
            location.to_string(),
            "2026-01-01T00:00:00Z".to_string(),
        )
    }

    fn envelope_with(artifacts: Vec<(ArtifactKind, &str, &str)>) -> CaptureEnvelope {
        CaptureEnvelope {
            schema_version: 1,
            capture_id: "018f2d3c-4b5a-7c6d-8e9f-000000000001".to_string(),
            idempotency_key: "opencode:synthetic:key".to_string(),
            source: integration_contracts::capture::CaptureSource {
                adapter: "opencode".to_string(),
                adapter_version: "0.0.0".to_string(),
                session_id: "session".to_string(),
                message_id: "message".to_string(),
            },
            project: integration_contracts::capture::ProjectRef {
                canonical_path: std::env::temp_dir().to_string_lossy().into_owned(),
            },
            observed_at: "2026-01-01T00:00:00Z".to_string(),
            artifacts: artifacts
                .into_iter()
                .map(|(kind, artifact_id, content)| {
                    integration_contracts::capture::SourceArtifact {
                        artifact_id: artifact_id.to_string(),
                        kind,
                        content: content.to_string(),
                        metadata: serde_json::Map::new(),
                        fingerprint: artifact_fingerprint(content),
                    }
                })
                .collect(),
        }
    }

    fn fake() -> FakeRepository {
        let repository = FakeRepository::default();
        // Register the canonical form, exactly as the ingest use case will look
        // it up.
        let location =
            crate::projects::canonicalize_location(&std::env::temp_dir().to_string_lossy())
                .expect("temp dir canonicalizes");
        repository
            .projects
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(project_at(&location));
        repository
    }

    const ARTIFACT_ID_1: &str = "018f2d3c-4b5a-7c6d-8e9f-000000000101";
    const ARTIFACT_ID_2: &str = "018f2d3c-4b5a-7c6d-8e9f-000000000102";

    #[test]
    fn payload_is_deduplicated_by_kind_and_computed_fingerprint() {
        let repository = fake();
        let ingest = CaptureIngest::new(repository);
        let envelope = envelope_with(vec![
            (ArtifactKind::UserText, ARTIFACT_ID_1, "identical content"),
            (ArtifactKind::UserText, ARTIFACT_ID_2, "identical content"),
        ]);
        let outcome = ingest
            .ingest(&envelope, &envelope.idempotency_key)
            .expect("ingest");
        assert_eq!(outcome.receipt.artifact_count, 1);
    }

    #[test]
    fn repeated_artifact_id_in_one_payload_is_a_conflict() {
        let repository = fake();
        let ingest = CaptureIngest::new(repository);
        let mut envelope = envelope_with(vec![
            (ArtifactKind::UserText, ARTIFACT_ID_1, "first content"),
            (ArtifactKind::AssistantText, ARTIFACT_ID_1, "second content"),
        ]);
        // Keep both fingerprints honest; the conflict is the repeated id.
        envelope.artifacts[0].fingerprint = artifact_fingerprint("first content");
        envelope.artifacts[1].fingerprint = artifact_fingerprint("second content");
        assert_eq!(
            ingest.ingest(&envelope, &envelope.idempotency_key),
            Err(IngestError::Conflict)
        );
    }

    #[test]
    fn tampered_fingerprint_is_rejected() {
        let repository = fake();
        let ingest = CaptureIngest::new(repository);
        let mut envelope = envelope_with(vec![(ArtifactKind::UserText, ARTIFACT_ID_1, "content")]);
        envelope.artifacts[0].fingerprint = "0".repeat(64);
        assert_eq!(
            ingest.ingest(&envelope, &envelope.idempotency_key),
            Err(IngestError::InvalidFingerprint)
        );
    }

    #[test]
    fn same_declared_fingerprint_for_different_contents_is_rejected() {
        let repository = fake();
        let ingest = CaptureIngest::new(repository);
        let declared = artifact_fingerprint("first content");
        let mut envelope = envelope_with(vec![
            (ArtifactKind::UserText, ARTIFACT_ID_1, "first content"),
            (ArtifactKind::UserText, ARTIFACT_ID_2, "second content"),
        ]);
        envelope.artifacts[1].fingerprint = declared;
        assert_eq!(
            ingest.ingest(&envelope, &envelope.idempotency_key),
            Err(IngestError::InvalidFingerprint)
        );
    }

    #[test]
    fn replay_returns_the_stored_receipt() {
        let repository = fake();
        let ingest = CaptureIngest::new(repository);
        let envelope = envelope_with(vec![(
            ArtifactKind::UserText,
            ARTIFACT_ID_1,
            "replay content",
        )]);
        let first = ingest
            .ingest(&envelope, &envelope.idempotency_key)
            .expect("first");
        assert!(!first.replayed);
        let replay = ingest
            .ingest(&envelope, &envelope.idempotency_key)
            .expect("replay");
        assert!(replay.replayed);
        assert_eq!(first.receipt, replay.receipt);
    }

    #[test]
    fn unregistered_project_is_forbidden() {
        let repository = FakeRepository::default();
        let ingest = CaptureIngest::new(repository);
        let envelope = envelope_with(vec![(ArtifactKind::UserText, ARTIFACT_ID_1, "content")]);
        assert_eq!(
            ingest.ingest(&envelope, &envelope.idempotency_key),
            Err(IngestError::Forbidden)
        );
    }
}
