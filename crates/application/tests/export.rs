//! Export use-case tests: fixed Markdown/JSON layouts, determinism, the
//! preview==file guarantee and destination validation.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use application::decisions::{
    DecisionQuery, DecisionRevisionRow, DecisionSearchRow, DecisionStatus, DecisionStore,
    DecisionsError, EvidenceLinkRow, StoredDecision,
};
use application::export::{Export, ExportFormat, ExportedDecision};

/// A store with one fully populated decision.
struct FakeStore {
    decision: StoredDecision,
    revisions: Vec<DecisionRevisionRow>,
    evidence: Vec<EvidenceLinkRow>,
}

impl DecisionStore for FakeStore {
    fn list(&self, _query: &DecisionQuery) -> Result<Vec<StoredDecision>, DecisionsError> {
        Ok(vec![self.decision.clone()])
    }

    fn get(&self, id: &str) -> Result<Option<StoredDecision>, DecisionsError> {
        Ok((id == self.decision.decision_id).then(|| self.decision.clone()))
    }

    fn revisions(&self, _id: &str) -> Result<Vec<DecisionRevisionRow>, DecisionsError> {
        Ok(self.revisions.clone())
    }

    fn evidence(&self, _id: &str) -> Result<Vec<EvidenceLinkRow>, DecisionsError> {
        Ok(self.evidence.clone())
    }

    fn search(
        &self,
        _match_query: &str,
        _project_id: Option<&str>,
        _limit: usize,
    ) -> Result<Vec<DecisionSearchRow>, DecisionsError> {
        Ok(Vec::new())
    }

    fn revise(
        &self,
        _id: &str,
        _content: &application::decisions::DecisionContent,
        _version: i64,
        _updated_at: &str,
    ) -> Result<bool, DecisionsError> {
        Ok(false)
    }
}

fn revision(version: i64, question: &str, created_at: &str) -> DecisionRevisionRow {
    DecisionRevisionRow {
        qualifiers: "[]".into(),
        version,
        created_at: created_at.to_string(),
        question: question.to_string(),
        choice: "c".to_string(),
        rationale: "r".to_string(),
        assumptions: "[]".to_string(),
        reconsider_when: "[]".to_string(),
        scope: "[]".to_string(),
        consequences: "[]".to_string(),
    }
}

fn store() -> FakeStore {
    FakeStore {
        decision: StoredDecision {
            qualifiers: "[]".into(),
            decision_id: "decision-1".to_string(),
            candidate_id: "candidate-1".to_string(),
            project_id: "project-1".to_string(),
            project_location: "C:/synthetic/project".to_string(),
            capture_id: Some("capture-1".to_string()),
            status: DecisionStatus::Accepted,
            question: "Usar FTS5?".to_string(),
            choice: "Sim, com índice explícito".to_string(),
            rationale: "Busca local é requisito".to_string(),
            assumptions: "[\"sqlite bundled\", \"sem triggers\"]".to_string(),
            reconsider_when: "[\"se o volume crescer\"]".to_string(),
            scope: "[\"application\", \"storage-sqlite\"]".to_string(),
            consequences: "[]".to_string(),
            version: 2,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            confirmed_at: "2026-01-01T00:00:01Z".to_string(),
            updated_at: "2026-01-02T00:00:00Z".to_string(),
        },
        revisions: vec![
            revision(2, "Usar FTS5?", "2026-01-02T00:00:00Z"),
            revision(1, "Usar FTS5?", "2026-01-01T00:00:00Z"),
        ],
        evidence: vec![
            EvidenceLinkRow {
                artifact_id: "art-2".to_string(),
                kind: Some("diff_hunk".to_string()),
                position: 0,
            },
            EvidenceLinkRow {
                artifact_id: "art-1".to_string(),
                kind: None,
                position: 1,
            },
        ],
    }
}

/// Asserts the directory holds no leftover `*.tmp` sibling.
fn assert_no_temporary(directory: &Path) {
    for entry in std::fs::read_dir(directory).expect("read directory") {
        let name = entry
            .expect("directory entry")
            .file_name()
            .to_string_lossy()
            .into_owned();
        assert!(
            !name.ends_with(".tmp"),
            "orphan temporary left behind: {name}"
        );
    }
}

fn temporary_directory(tag: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let directory = std::env::temp_dir().join(format!(
        "xemnas-export-{tag}-{}-{nanos}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir_all(&directory).expect("create temporary directory");
    directory
}

#[test]
fn markdown_contains_every_section_and_value() {
    let export = Export::new(store());
    let document = export
        .preview("decision-1", ExportFormat::Markdown)
        .expect("preview");
    let content = &document.content;

    assert!(content.starts_with("# Usar FTS5?\n"), "{content}");
    for heading in [
        "## Escolha",
        "## Justificativa",
        "## Premissas",
        "## Escopo",
        "## Consequências",
        "## Reconsiderar quando",
        "## Proveniência",
        "## Evidências",
        "## Histórico de revisões",
    ] {
        assert!(content.contains(heading), "missing {heading}: {content}");
    }
    assert!(content.contains("Sim, com índice explícito"));
    assert!(content.contains("- sqlite bundled"));
    assert!(content.contains("- se o volume crescer"));
    assert!(content.contains("Projeto: C:/synthetic/project"));
    assert!(content.contains("Candidato: candidate-1"));
    assert!(content.contains("- art-2 (diff_hunk)"));
    assert!(content.contains("- art-1"));
    assert!(content.contains("- v2 — 2026-01-02T00:00:00Z"));
    assert!(
        content.contains("- _(nenhuma)_"),
        "empty consequence list uses the fixed placeholder"
    );
    assert!(content.ends_with('\n'));
}

#[test]
fn json_round_trips_every_field() {
    let export = Export::new(store());
    let document = export
        .preview("decision-1", ExportFormat::Json)
        .expect("preview");
    let parsed: ExportedDecision = serde_json::from_str(&document.content).expect("parse");

    assert_eq!(parsed.decision_id, "decision-1");
    assert_eq!(parsed.status, "accepted");
    assert_eq!(parsed.question, "Usar FTS5?");
    assert_eq!(parsed.assumptions, vec!["sqlite bundled", "sem triggers"]);
    assert_eq!(parsed.consequences, Vec::<String>::new());
    assert_eq!(parsed.version, 2);
    assert_eq!(parsed.capture_id.as_deref(), Some("capture-1"));
    assert_eq!(parsed.evidence.len(), 2);
    assert_eq!(parsed.evidence[0].artifact_id, "art-2");
    assert_eq!(parsed.revisions.len(), 2);
    assert_eq!(parsed.revisions[0].version, 2);
    assert!(document.content.ends_with('\n'));
}

#[test]
fn preview_is_deterministic() {
    let export = Export::new(store());
    let first = export
        .preview("decision-1", ExportFormat::Markdown)
        .expect("first");
    let second = export
        .preview("decision-1", ExportFormat::Markdown)
        .expect("second");
    assert_eq!(first.content, second.content);
    assert_eq!(first.bytes, second.content.len());
}

#[test]
fn preview_matches_the_written_file() {
    let root = temporary_directory("roundtrip");
    let destination = root.join("decision.md");
    let export = Export::new(store());
    let document = export
        .preview("decision-1", ExportFormat::Markdown)
        .expect("preview");

    let result = export.write(&document, &destination, false).expect("write");
    assert_eq!(result.bytes, document.bytes);
    let written = std::fs::read_to_string(&destination).expect("read");
    assert_eq!(written, document.content);
    assert_no_temporary(&root);

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn write_refuses_existing_without_overwrite_and_allows_it() {
    let root = temporary_directory("overwrite");
    let destination = root.join("decision.json");
    std::fs::write(&destination, "old").expect("seed file");
    let export = Export::new(store());
    let document = export
        .preview("decision-1", ExportFormat::Json)
        .expect("preview");

    let error = export
        .write(&document, &destination, false)
        .expect_err("must refuse");
    assert_eq!(error.code(), "destination_exists");

    export
        .write(&document, &destination, true)
        .expect("overwrite");
    assert_eq!(
        std::fs::read_to_string(&destination).expect("read"),
        document.content
    );
    assert_no_temporary(&root);

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn write_never_truncates_a_pre_existing_temporary_like_file() {
    let root = temporary_directory("temp-clash");
    let planted = root.join("decision.md.00000000-0000-0000-0000-000000000000.tmp");
    std::fs::write(&planted, "planted").expect("plant");
    let export = Export::new(store());
    let document = export
        .preview("decision-1", ExportFormat::Markdown)
        .expect("preview");

    export
        .write(&document, &root.join("decision.md"), true)
        .expect("write");

    assert_eq!(
        std::fs::read_to_string(&planted).expect("read planted"),
        "planted"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("decision.md")).expect("read"),
        document.content
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn overwrite_false_rejects_a_destination_created_after_preview() {
    let root = temporary_directory("toctou");
    let destination = root.join("decision.md");
    let export = Export::new(store());
    let document = export
        .preview("decision-1", ExportFormat::Markdown)
        .expect("preview");

    std::fs::write(&destination, "raced").expect("create destination");
    let error = export
        .write(&document, &destination, false)
        .expect_err("must refuse");
    assert_eq!(error.code(), "destination_exists");
    assert_eq!(
        std::fs::read_to_string(&destination).expect("read"),
        "raced",
        "the raced destination must stay intact"
    );
    assert_no_temporary(&root);

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn write_rejects_a_missing_parent_directory() {
    let root = temporary_directory("missing-parent");
    let destination = root.join("nested").join("decision.md");
    let export = Export::new(store());
    let document = export
        .preview("decision-1", ExportFormat::Markdown)
        .expect("preview");

    let error = export
        .write(&document, &destination, false)
        .expect_err("must refuse");
    assert_eq!(error.code(), "destination_invalid");

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn write_rejects_a_directory_target() {
    let root = temporary_directory("directory-target");
    let destination = root.join("folder");
    std::fs::create_dir_all(&destination).expect("create dir");
    let export = Export::new(store());
    let document = export
        .preview("decision-1", ExportFormat::Markdown)
        .expect("preview");

    let error = export
        .write(&document, &destination, false)
        .expect_err("must refuse");
    assert_eq!(error.code(), "destination_exists");

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn preview_of_an_unknown_decision_is_not_found() {
    let export = Export::new(store());
    let error = export
        .preview("missing", ExportFormat::Markdown)
        .expect_err("must fail");
    assert_eq!(error.code(), "not_found");
}
