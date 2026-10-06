//! Shared fixtures for the Fase 3 storage tests.
#![allow(dead_code)]

use application::captures::{
    CaptureArtifactRecord, CaptureCheckpointRecord, CaptureReceiptRecord, CaptureRepository,
    CaptureWrite,
};
use application::decisions::DecisionEdits;
use application::extract::{DecisionCandidateRecord, ExtractionStore};
use application::inbox::{CandidateEdits, Inbox};
use application::jobs::{JobRecord, JobState, ANALYZE_CAPTURE_KIND};
use application::projects::{ProjectRecord, ProjectRepository};
use storage_sqlite::SqliteStore;

/// A fresh store in a unique temporary directory.
pub struct TestStore {
    pub store: SqliteStore,
    pub root: std::path::PathBuf,
}

impl Drop for TestStore {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// Opens a store with one capture in each of `projects`.
pub fn open(tag: &str, projects: &[&str]) -> TestStore {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let root = std::env::temp_dir().join(format!(
        "xemnas-fase3-{tag}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("create temporary directory");
    let store = SqliteStore::open(root.join("app.db")).expect("open store");
    for project in projects {
        seed_project(&store, project);
    }
    TestStore { store, root }
}

fn seed_project(store: &SqliteStore, project: &str) {
    let location = format!("C:/synthetic/{project}");
    let at = "2026-01-01T00:00:00Z".to_string();
    store
        .insert(&ProjectRecord::new(
            project.to_string(),
            location.clone(),
            at.clone(),
        ))
        .expect("seed project");
    let capture = format!("capture-{project}");
    store
        .insert_capture(&CaptureWrite {
            receipt: CaptureReceiptRecord {
                capture_id: capture.clone(),
                idempotency_key: format!("key-{capture}"),
                canonical_path: location,
                received_at: at.clone(),
                artifact_count: 1,
            },
            artifacts: vec![CaptureArtifactRecord {
                capture_id: capture.clone(),
                artifact_id: format!("art-{project}"),
                kind: "diff_hunk".to_string(),
                content: "diff --git a/src/lib.rs b/src/lib.rs".to_string(),
                metadata: "{}".to_string(),
                fingerprint: format!("{:0>64}", project.len()),
            }],
            job: JobRecord {
                id: format!("job-{capture}"),
                kind: ANALYZE_CAPTURE_KIND.to_string(),
                payload: capture.clone(),
                state: JobState::Queued,
                idempotent: true,
                attempts: 0,
                last_error: None,
                created_at: at.clone(),
                updated_at: at.clone(),
            },
            checkpoint: CaptureCheckpointRecord {
                adapter: "opencode".to_string(),
                adapter_version: "0.1.0".to_string(),
                session_id: format!("session-{project}"),
                message_id: format!("message-{project}"),
                capture_id: capture,
                observed_at: at.clone(),
                updated_at: at,
            },
        })
        .expect("seed capture");
}

/// Confirms a new candidate in `project` and returns the decision id.
pub fn decision(
    store: &SqliteStore,
    project: &str,
    key: &str,
    question: &str,
    choice: &str,
) -> String {
    let id = format!("cand-{key}");
    store
        .insert_candidates(&[DecisionCandidateRecord {
            qualifiers: "[]".into(),
            id: id.clone(),
            project_id: project.to_string(),
            capture_id: format!("capture-{project}"),
            status: "pending".to_string(),
            question: question.to_string(),
            choice: choice.to_string(),
            rationale: format!("motivo de {key}"),
            signals: "[\"public_contract\"]".to_string(),
            confidence: 0.7,
            confidence_reason: "sintético".to_string(),
            evidence_refs: format!("[\"art-{project}\"]"),
            diff_summary: "{\"files\":[],\"artifacts\":1}".to_string(),
            dedup_hash: format!("dedup-{key}"),
            created_at: "2026-01-02T00:00:00Z".to_string(),
            updated_at: "2026-01-02T00:00:00Z".to_string(),
            kind: "decision".to_string(),
            significance: 1.0,
            criteria: "[]".to_string(),
        }])
        .expect("insert candidate");
    let edits: Option<CandidateEdits> = None;
    Inbox::new(store.clone())
        .confirm(&id, edits)
        .expect("confirm candidate")
        .decision_id
}

/// Confirms a decision whose own capture changed `files` and carries `diff`
/// as a `diff_hunk`, and returns the decision id.
pub fn decision_with_diff(
    store: &SqliteStore,
    project: &str,
    key: &str,
    question: &str,
    files: &[&str],
    diff: &str,
) -> String {
    let location = format!("C:/synthetic/{project}");
    decision_at(store, project, &location, key, question, files, diff)
}

/// [`decision_with_diff`] for a project registered at a real `location`.
pub fn decision_at(
    store: &SqliteStore,
    project: &str,
    location: &str,
    key: &str,
    question: &str,
    files: &[&str],
    diff: &str,
) -> String {
    let capture = format!("capture-{key}");
    let at = "2026-01-02T00:00:00Z".to_string();
    store
        .insert_capture(&CaptureWrite {
            receipt: CaptureReceiptRecord {
                capture_id: capture.clone(),
                idempotency_key: format!("key-{capture}"),
                canonical_path: location.to_string(),
                received_at: at.clone(),
                artifact_count: 1,
            },
            artifacts: vec![CaptureArtifactRecord {
                capture_id: capture.clone(),
                artifact_id: format!("art-{key}"),
                kind: "diff_hunk".to_string(),
                content: diff.to_string(),
                metadata: "{}".to_string(),
                fingerprint: format!("{:0>64}", key.len() + 7),
            }],
            job: JobRecord {
                id: format!("job-{capture}"),
                kind: ANALYZE_CAPTURE_KIND.to_string(),
                payload: capture.clone(),
                state: JobState::Queued,
                idempotent: true,
                attempts: 0,
                last_error: None,
                created_at: at.clone(),
                updated_at: at.clone(),
            },
            checkpoint: CaptureCheckpointRecord {
                adapter: "opencode".to_string(),
                adapter_version: "0.1.0".to_string(),
                session_id: format!("session-{key}"),
                message_id: format!("message-{key}"),
                capture_id: capture.clone(),
                observed_at: at.clone(),
                updated_at: at.clone(),
            },
        })
        .expect("seed capture");
    let id = format!("cand-{key}");
    let files: Vec<String> = files.iter().map(|file| format!("\"{file}\"")).collect();
    store
        .insert_candidates(&[DecisionCandidateRecord {
            qualifiers: "[]".into(),
            id: id.clone(),
            project_id: project.to_string(),
            capture_id: capture,
            status: "pending".to_string(),
            question: question.to_string(),
            choice: format!("escolha de {key}"),
            rationale: format!("motivo de {key}"),
            signals: "[\"public_contract\"]".to_string(),
            confidence: 0.7,
            confidence_reason: "sintético".to_string(),
            evidence_refs: format!("[\"art-{key}\"]"),
            diff_summary: format!("{{\"files\":[{}],\"artifacts\":1}}", files.join(",")),
            dedup_hash: format!("dedup-{key}"),
            created_at: at.clone(),
            updated_at: at,
            kind: "decision".to_string(),
            significance: 1.0,
            criteria: "[]".to_string(),
        }])
        .expect("insert candidate");
    let edits: Option<CandidateEdits> = None;
    Inbox::new(store.clone())
        .confirm(&id, edits)
        .expect("confirm candidate")
        .decision_id
}

/// Edits with only the rationale set.
pub fn rationale(text: &str) -> DecisionEdits {
    DecisionEdits {
        rationale: Some(text.to_string()),
        ..DecisionEdits::default()
    }
}
