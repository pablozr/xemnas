//! Diagnostics sanitization against the real database: markers planted in
//! artifact content, decision text, job diagnostics, api-token and outbox files
//! must never appear in the exported document, while structural counts do.

use std::cell::RefCell;

use application::captures::{
    CaptureArtifactRecord, CaptureCheckpointRecord, CaptureReceiptRecord, CaptureRepository,
    CaptureWrite,
};
use application::diagnostics::{Diagnostics, DiagnosticsDocument};
use application::extract::{DecisionCandidateRecord, ExtractionStore};
use application::inbox::{CandidateEdits, Inbox};
use application::jobs::{JobRecord, JobRepository, JobState, ANALYZE_CAPTURE_KIND};
use application::profile::{
    build_preview, preview_hash, AiProfile, AiSettings, ConsentRecord, ProfileError, ProfileKind,
    ProfileStore, SecretStore,
};
use application::projects::{ProjectRecord, ProjectRepository};
use storage_sqlite::SqliteStore;

const LOCATION: &str = "C:/synthetic/SECRET-MARKER-PATH";
const MARKER_ARTIFACT: &str = "SECRET-MARKER-ARTIFACT";
const MARKER_RATIONALE: &str = "SECRET-MARKER-RATIONALE";
const MARKER_JOB: &str = "SECRET-MARKER-JOB";
const MARKER_TOKEN: &str = "SECRET-MARKER-TOKEN";
const MARKER_OUTBOX: &str = "SECRET-MARKER-OUTBOX";
const MARKER_PATH: &str = "SECRET-MARKER-PATH";
const MARKER_PW: &str = "SECRET-MARKER-PW";
const MARKER_QS: &str = "SECRET-MARKER-QS";

fn temporary_directory(tag: &str) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let directory = std::env::temp_dir().join(format!(
        "xemnas-diagnostics-{tag}-{}-{nanos}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir_all(&directory).expect("create temporary directory");
    directory
}

/// A legacy external profile with credential/query data and an old consent:
/// it must be blocked, and only its hostname may reach the document.
struct SeededProfiles(RefCell<Option<AiProfile>>);

impl SeededProfiles {
    fn external_with_credential_endpoint() -> Self {
        let mut profile = AiProfile {
            id: "profile-1".to_string(),
            kind: ProfileKind::OpenAiCompatible,
            model: "gpt-test".to_string(),
            endpoint: Some(format!(
                "https://user:{MARKER_PW}@exemplo.test/v1?chave={MARKER_QS}"
            )),
            max_input_chars: 4_096,
            external_calls_enabled: true,
            consent: None,
        };
        profile.consent = Some(ConsentRecord {
            granted_at: "2026-01-01T00:00:00Z".to_string(),
            preview_hash: preview_hash(&build_preview(&profile)),
        });
        Self(RefCell::new(Some(profile)))
    }
}

impl ProfileStore for SeededProfiles {
    fn load(&self) -> Result<Option<AiProfile>, ProfileError> {
        Ok(self.0.borrow().clone())
    }
    fn save(&self, profile: &AiProfile) -> Result<(), ProfileError> {
        *self.0.borrow_mut() = Some(profile.clone());
        Ok(())
    }
}

#[derive(Default)]
struct SomeSecrets;

impl SecretStore for SomeSecrets {
    fn set_secret(&self, _account: &str, _secret: &str) -> Result<(), ProfileError> {
        Ok(())
    }
    fn get_secret(&self, _account: &str) -> Result<Option<String>, ProfileError> {
        Ok(Some("sk-synthetic".to_string()))
    }
    fn delete_secret(&self, _account: &str) -> Result<(), ProfileError> {
        Ok(())
    }
}

fn artifact(capture_id: &str, artifact_id: &str, content: &str, seed: u8) -> CaptureArtifactRecord {
    CaptureArtifactRecord {
        capture_id: capture_id.to_string(),
        artifact_id: artifact_id.to_string(),
        kind: "diff_hunk".to_string(),
        content: content.to_string(),
        metadata: "{}".to_string(),
        fingerprint: format!("{:064x}", seed as u128),
    }
}

fn candidate(id: &str) -> DecisionCandidateRecord {
    DecisionCandidateRecord {
        id: id.to_string(),
        project_id: "project-1".to_string(),
        capture_id: "capture-1".to_string(),
        status: "pending".to_string(),
        question: "Adotar o índice?".to_string(),
        choice: "Sim".to_string(),
        rationale: format!("{MARKER_RATIONALE} justificativa"),
        signals: "[\"public_contract\"]".to_string(),
        confidence: 0.7,
        confidence_reason: "x".to_string(),
        evidence_refs: "[\"art-1\"]".to_string(),
        diff_summary: "{\"files\":[],\"artifacts\":2}".to_string(),
        dedup_hash: "dedup-cand-1".to_string(),
        created_at: "2026-01-03T00:00:00Z".to_string(),
        updated_at: "2026-01-03T00:00:00Z".to_string(),
    }
}

#[test]
fn exported_diagnostics_never_carry_content_markers() {
    let root = temporary_directory("sanitize");
    let store = SqliteStore::open(root.join("app.db")).expect("open store");

    // Project + capture with a marker inside artifact content.
    let project = ProjectRecord::new(
        "project-1".to_string(),
        LOCATION.to_string(),
        "2026-01-01T00:00:00Z".to_string(),
    );
    ProjectRepository::insert(&store, &project).expect("seed project");

    let timestamp = "2026-01-01T00:00:00Z".to_string();
    store
        .insert_capture(&CaptureWrite {
            receipt: CaptureReceiptRecord {
                capture_id: "capture-1".to_string(),
                idempotency_key: "key-capture-1".to_string(),
                canonical_path: LOCATION.to_string(),
                received_at: timestamp.clone(),
                artifact_count: 2,
            },
            artifacts: vec![
                artifact("capture-1", "art-1", MARKER_ARTIFACT, 1),
                artifact("capture-1", "art-2", "plain", 2),
            ],
            job: JobRecord {
                id: "job-capture".to_string(),
                kind: ANALYZE_CAPTURE_KIND.to_string(),
                payload: "capture-1".to_string(),
                state: JobState::Queued,
                idempotent: true,
                attempts: 0,
                last_error: None,
                created_at: timestamp.clone(),
                updated_at: timestamp.clone(),
            },
            checkpoint: CaptureCheckpointRecord {
                adapter: "opencode".to_string(),
                adapter_version: "0.1.0".to_string(),
                session_id: "session-1".to_string(),
                message_id: "message-1".to_string(),
                capture_id: "capture-1".to_string(),
                observed_at: timestamp.clone(),
                updated_at: timestamp.clone(),
            },
        })
        .expect("seed capture");

    // Candidate with a marker in its rationale, promoted to a decision.
    store
        .insert_candidates(&[candidate("cand-1")])
        .expect("insert candidate");
    Inbox::new(store.clone())
        .adjust(
            "cand-1",
            CandidateEdits {
                question: "Adotar o índice?".to_string(),
                choice: "Sim".to_string(),
                rationale: format!("{MARKER_RATIONALE} justificativa"),
            },
        )
        .expect("adjust");
    Inbox::new(store.clone())
        .confirm("cand-1", None)
        .expect("confirm");

    // A failed job whose diagnostic is hostile.
    let hostile = JobRecord {
        id: "job-hostile".to_string(),
        kind: ANALYZE_CAPTURE_KIND.to_string(),
        payload: "capture-1".to_string(),
        state: JobState::Failed,
        idempotent: true,
        attempts: 1,
        last_error: Some(MARKER_JOB.to_string()),
        created_at: "2026-01-04T00:00:00Z".to_string(),
        updated_at: "2026-01-04T00:00:00Z".to_string(),
    };
    JobRepository::insert(&store, &hostile).expect("insert hostile job");

    // Runtime files that must never be read into the document.
    let state_dir = root.join("state");
    std::fs::create_dir_all(&state_dir).expect("create state dir");
    std::fs::write(state_dir.join("api-token"), MARKER_TOKEN).expect("write token");

    // Outbox with one file per bucket; only counts may appear.
    let outbox = root.join("outbox");
    for (bucket, marker) in [
        ("pending", MARKER_OUTBOX),
        ("accepted", "accepted"),
        ("rejected", "rejected"),
    ] {
        let directory = outbox.join(bucket);
        std::fs::create_dir_all(&directory).expect("create bucket");
        std::fs::write(directory.join("item.json"), marker).expect("write item");
    }

    let diagnostics = Diagnostics::new(
        store.clone(),
        AiSettings::new(
            SeededProfiles::external_with_credential_endpoint(),
            SomeSecrets,
        ),
        &outbox,
    );
    let document = diagnostics.export().expect("export");
    let json = application::serde_json::to_string_pretty(&document).expect("serialize");

    // Sanitization by construction: no marker survives anywhere, including the
    // project path and the credential/query embedded in the endpoint.
    for marker in [
        MARKER_ARTIFACT,
        MARKER_RATIONALE,
        MARKER_JOB,
        MARKER_TOKEN,
        MARKER_OUTBOX,
        MARKER_PATH,
        MARKER_PW,
        MARKER_QS,
        "sk-synthetic",
    ] {
        assert!(
            !json.contains(marker),
            "diagnostics leaked a marker: {marker}"
        );
    }

    // Structural information is present and correct.
    assert_eq!(document.schema.migrations_version, 9);
    assert!(
        json.contains("\"metrics\""),
        "the metrics section is part of the sanitized sweep"
    );
    assert_eq!(document.counts.projects, 1);
    assert_eq!(document.counts.captures, 1);
    assert_eq!(document.counts.artifacts, 2);
    assert_eq!(document.counts.decisions, 1);
    assert_eq!(document.counts.revisions, 1);
    assert!(document
        .counts
        .candidates_by_status
        .contains_key("accepted"));
    assert_eq!(document.outbox.pending, 1);
    assert_eq!(document.outbox.accepted, 1);
    assert_eq!(document.outbox.rejected, 1);
    assert_eq!(
        document.recent_receipts[0].project_id.as_deref(),
        Some("project-1"),
        "the receipt exposes the project id instead of the path"
    );
    assert_eq!(document.ai_profile.kind, "openai-compatible");
    assert_eq!(
        document.ai_profile.provider.as_deref(),
        Some("exemplo.test"),
        "only the hostname survives the endpoint parsing"
    );
    assert!(document.ai_profile.has_secret);
    assert_eq!(
        document.ai_profile.consent_status,
        "configuração do provedor inválida"
    );
    assert_eq!(document.runtime.mode, "offline");
    let hostile = document
        .recent_jobs
        .iter()
        .find(|job| job.id == "job-hostile")
        .expect("hostile job tracked");
    assert_eq!(hostile.error_code.as_deref(), Some("failed"));
    assert!(document
        .recent_receipts
        .iter()
        .any(|receipt| receipt.capture_id == "capture-1"));

    let reparsed: DiagnosticsDocument =
        application::serde_json::from_str(&json).expect("round trip");
    assert_eq!(reparsed, document);

    let _ = std::fs::remove_dir_all(&root);
}
