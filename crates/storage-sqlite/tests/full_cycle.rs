//! End-to-end of the full cycle, in process: register, capture, candidate,
//! Evidence, edit, confirmation, search and export — while proving the Project
//! directory is never mutated and no `.git` is created.

use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use application::serde_json::Map;
use application::{
    artifact_fingerprint, canonicalize_location, run_extraction, ArtifactKind, CandidateEdits,
    CandidateStatus, CaptureApi, CaptureEnvelope, CaptureIngest, CaptureSource, Decisions, Export,
    ExportFormat, FakeCandidateExtractor, Inbox, InboxFilter, ProjectRef, Projects, RunContext,
    SearchQuery, SourceArtifact,
};
use storage_sqlite::SqliteStore;

fn temporary_directory(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let directory = std::env::temp_dir().join(format!(
        "xemnas-full-cycle-{tag}-{}-{nanos}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir_all(&directory).expect("create temporary directory");
    directory
}

/// Collects `(relative path, content hash)` for every file under `root`.
fn snapshot(root: &Path) -> Vec<(String, u64)> {
    let mut entries = Vec::new();
    visit(root, root, &mut entries);
    entries.sort();
    entries
}

fn visit(root: &Path, directory: &Path, out: &mut Vec<(String, u64)>) {
    for entry in std::fs::read_dir(directory).expect("read directory") {
        let entry = entry.expect("directory entry");
        let path = entry.path();
        if path.is_dir() {
            visit(root, &path, out);
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .expect("relative path")
            .to_string_lossy()
            .into_owned();
        let bytes = std::fs::read(&path).expect("read file");
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        bytes.hash(&mut hasher);
        out.push((relative, hasher.finish()));
    }
}

/// A plausible capture: a Rust-relevant DDL diff plus a note.
fn envelope(canonical: &str, capture_id: &str) -> CaptureEnvelope {
    let diff = "diff --git a/src/schema.rs b/src/schema.rs\n+CREATE TABLE decisions (id TEXT);\n";
    let note = "Decidimos usar FTS5 para busca local.";
    CaptureEnvelope {
        schema_version: 1,
        capture_id: capture_id.to_string(),
        idempotency_key: format!("key-{capture_id}"),
        source: CaptureSource {
            adapter: "opencode".to_string(),
            adapter_version: "1.0.0".to_string(),
            session_id: "session-1".to_string(),
            message_id: "message-1".to_string(),
        },
        project: ProjectRef {
            canonical_path: canonical.to_string(),
        },
        observed_at: "2026-01-02T00:00:00Z".to_string(),
        artifacts: vec![
            SourceArtifact {
                artifact_id: "artifact-diff".to_string(),
                kind: ArtifactKind::DiffHunk,
                content: diff.to_string(),
                metadata: Map::new(),
                fingerprint: artifact_fingerprint(diff),
            },
            SourceArtifact {
                artifact_id: "artifact-notes".to_string(),
                kind: ArtifactKind::UserText,
                content: note.to_string(),
                metadata: Map::new(),
                fingerprint: artifact_fingerprint(note),
            },
        ],
    }
}

fn edited() -> CandidateEdits {
    CandidateEdits {
        question: "Adotar FTS5 para busca?".to_string(),
        choice: "Sim, com tabela virtual".to_string(),
        rationale: "Busca local exige índice.".to_string(),
    }
}

#[test]
fn full_cycle_covers_register_capture_evidence_edit_confirm_search_export() {
    let root = temporary_directory("cycle");
    let project_dir = root.join("project");
    std::fs::create_dir_all(project_dir.join("src")).expect("create project dir");
    std::fs::write(project_dir.join(".gitignore"), "target/\n").expect("write gitignore");
    std::fs::write(project_dir.join("README.md"), "# synthetic\n").expect("write readme");
    std::fs::write(project_dir.join("src").join("main.rs"), "fn main() {}\n").expect("write main");

    let before = snapshot(&project_dir);
    assert!(before.iter().any(|(name, _)| name == ".gitignore"));

    let store = SqliteStore::open(root.join("app.db")).expect("open store");

    let canonical = canonicalize_location(&project_dir.to_string_lossy()).expect("canonical");
    Projects::new(store.clone())
        .register(&project_dir)
        .expect("register project");

    let envelope = envelope(&canonical, "capture-full-cycle");
    let outcome = CaptureIngest::new(store.clone())
        .ingest(&envelope, &envelope.idempotency_key)
        .expect("ingest");
    assert!(!outcome.replayed);
    let capture_id = outcome.receipt.capture_id.clone();

    run_extraction(
        &store,
        &FakeCandidateExtractor,
        &capture_id,
        &RunContext::for_tests(),
    )
    .expect("extract");
    let inbox = Inbox::new(store.clone());
    let page = inbox.list(&InboxFilter::new()).expect("list");
    assert_eq!(page.candidates.len(), 1, "one pending candidate");
    let candidate_id = page.candidates[0].id.clone();
    assert_eq!(page.candidates[0].status, CandidateStatus::Pending);

    let detail = inbox.detail(&candidate_id).expect("detail");
    let evidence: Vec<&str> = detail
        .artifacts
        .iter()
        .map(|artifact| artifact.artifact_id.as_str())
        .collect();
    assert_eq!(evidence, vec!["artifact-diff", "artifact-notes"]);

    inbox.adjust(&candidate_id, edited()).expect("adjust");
    assert_eq!(
        inbox.detail(&candidate_id).expect("detail").summary.status,
        CandidateStatus::Pending
    );

    let confirmed = inbox
        .confirm(&candidate_id, Some(edited()))
        .expect("confirm");
    assert_eq!(confirmed.status, CandidateStatus::EditedAndAccepted);

    let decisions = Decisions::new(store.clone());
    let hits = decisions
        .search(&SearchQuery {
            query: "FTS5".to_string(),
            ..SearchQuery::default()
        })
        .expect("search");
    assert!(
        hits.iter()
            .any(|hit| hit.decision_id == confirmed.decision_id),
        "the confirmed decision must be searchable"
    );

    let export_dir = root.join("exports");
    std::fs::create_dir_all(&export_dir).expect("create export dir");
    let export = Export::new(store.clone());
    let document = export
        .preview(&confirmed.decision_id, ExportFormat::Markdown)
        .expect("preview");
    let destination = export_dir.join("decision.md");
    let written = export.write(&document, &destination, false).expect("write");
    assert_eq!(written.bytes, document.bytes);
    assert_eq!(
        std::fs::read_to_string(&destination).expect("read export"),
        document.content,
        "preview must equal the written file"
    );

    let decision = decisions.detail(&confirmed.decision_id).expect("detail");
    assert!(
        decision
            .revisions
            .iter()
            .any(|revision| revision.version == 1),
        "the birth revision must stay readable"
    );

    let after = snapshot(&project_dir);
    assert_eq!(
        before, after,
        "the project directory must not change during the cycle"
    );
    assert!(!project_dir.join(".git").exists(), "no .git may be created");

    let _ = std::fs::remove_dir_all(&root);
}
