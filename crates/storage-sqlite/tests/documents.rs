//! Project documentation over SQLite: indexing the project folder, change
//! detection, listing and removal with the project.

mod support;

use application::documents::{DocumentError, DocumentKind, Documents, DOCUMENT_ADAPTER};
use application::extract::{
    run_extraction, CandidateExtractor, CandidateKind, CandidateProposal, DecisionEvidence,
    ExtractError, FakeCandidateExtractor, RelevanceSignal, RunContext,
};
use application::projects::{ProjectRecord, ProjectRepository, Projects};
use rusqlite::Connection;

fn project_with_docs(test: &support::TestStore) -> std::path::PathBuf {
    let repo = test.root.join("repo");
    std::fs::create_dir_all(repo.join("docs/adr")).expect("docs");
    std::fs::write(
        repo.join("README.md"),
        "# Xemnas\n\nGuarda decisões de engenharia localmente.\n\n## Instalação\n\nBaixe o app.\n\n\
         ## Arquitetura\n\nDecidimos guardar tudo em SQLite em vez de um servidor. Nunca \
         gravamos segredos no banco.\n",
    )
    .expect("readme");
    std::fs::write(
        repo.join("docs/adr/0001-sqlite.md"),
        "# Usar SQLite\n\n## Contexto\nUm arquivo local.\n## Decisão\nSQLite.\n",
    )
    .expect("adr");
    test.store
        .insert(&ProjectRecord::new(
            "docs-p".into(),
            repo.to_string_lossy().replace('\\', "/"),
            "2026-01-01T00:00:00Z".into(),
        ))
        .expect("project");
    repo
}

#[test]
fn indexing_reads_the_folder_and_notices_changes() {
    let test = support::open("documents", &[]);
    let repo = project_with_docs(&test);
    let documents = Documents::new(test.store.clone());

    let first = documents.index("docs-p").expect("index");
    assert_eq!((first.total, first.changed, first.removed), (2, 2, 0));
    let listed = documents.list("docs-p").expect("list");
    assert_eq!(listed[0].kind, DocumentKind::Adr, "ADRs first");
    assert_eq!(listed[0].title, "Usar SQLite");
    assert_eq!(listed[0].headings, vec!["Contexto", "Decisão"]);
    assert_eq!(listed[1].kind, DocumentKind::Readme);
    assert_eq!(
        listed[1].excerpt,
        "Guarda decisões de engenharia localmente."
    );

    let again = documents.index("docs-p").expect("same files");
    assert_eq!((again.changed, again.removed), (0, 0));

    std::fs::write(repo.join("README.md"), "# Xemnas\n\nOutro texto.\n").expect("edit");
    std::fs::remove_file(repo.join("docs/adr/0001-sqlite.md")).expect("remove");
    let changed = documents.index("docs-p").expect("changed");
    assert_eq!((changed.total, changed.changed, changed.removed), (1, 1, 1));

    Projects::new(test.store.clone())
        .remove_with_data("docs-p", "repo")
        .expect("remove project");
    assert!(documents.list("docs-p").expect("gone").is_empty());
}

#[test]
fn a_missing_folder_is_reported_not_indexed() {
    let test = support::open("documents-missing", &["p1"]);
    assert_eq!(
        Documents::new(test.store.clone()).index("p1"),
        Err(DocumentError::FolderUnavailable)
    );
    assert_eq!(
        Documents::new(test.store.clone()).index("nope"),
        Err(DocumentError::ProjectNotFound)
    );
}

/// A model that proposes one decision citing whatever it was given.
struct Scripted;

impl CandidateExtractor for Scripted {
    fn extract(
        &self,
        input: &DecisionEvidence,
        signals: &[RelevanceSignal],
    ) -> Result<Vec<CandidateProposal>, ExtractError> {
        assert!(input
            .artifacts
            .iter()
            .all(|artifact| artifact.kind == "document"));
        Ok(vec![CandidateProposal {
            question: "Onde guardar os dados?".into(),
            choice: "Em SQLite.".into(),
            rationale: "Um arquivo local, sem servidor.".into(),
            confidence: 0.9,
            confidence_reason: "O ADR registra a escolha.".into(),
            signals: signals.to_vec(),
            evidence_refs: input
                .artifacts
                .iter()
                .map(|artifact| artifact.artifact_id.clone())
                .collect(),
            diff_summary: String::new(),
            kind: CandidateKind::Decision,
            significance: 0.8,
            criteria: vec!["data_or_contract".into()],
        }])
    }
}

#[test]
fn documents_go_to_review_once_per_version_tied_to_the_code_they_cite() {
    let test = support::open("documents-propose", &[]);
    let repo = project_with_docs(&test);
    std::fs::write(
        repo.join("docs/adr/0001-sqlite.md"),
        "# Usar SQLite\n\n## Decisão\nGuardar tudo em SQLite, em \
         crates/storage-sqlite/src/store.rs.\n",
    )
    .expect("adr");
    let documents = Documents::new(test.store.clone());
    documents.index("docs-p").expect("index");

    assert_eq!(
        documents.propose("docs-p", 1).expect("limit"),
        1,
        "the limit holds"
    );
    assert_eq!(documents.propose("docs-p", 10).expect("rest"), 1);
    assert_eq!(
        documents.propose("docs-p", 10).expect("again"),
        0,
        "a version is queued once"
    );

    let connection = Connection::open(test.root.join("app.db")).expect("raw");
    let captures: Vec<String> = connection
        .prepare(
            "SELECT capture_id FROM adapter_checkpoints WHERE adapter = ?1              ORDER BY message_id LIKE 'docs/%' DESC",
        )
        .expect("prepare")
        .query_map([DOCUMENT_ADAPTER], |row| row.get(0))
        .expect("query")
        .collect::<Result<_, _>>()
        .expect("rows");
    assert_eq!(captures.len(), 2);
    let queued: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM jobs WHERE state = ?1 AND kind = ?2",
            [
                application::jobs::JobState::Queued.as_str(),
                application::jobs::ANALYZE_CAPTURE_KIND,
            ],
            |row| row.get(0),
        )
        .expect("jobs");
    assert!(queued >= 2, "each document waits for analysis");

    // The ADR (listed first) goes through the same extraction as a
    // conversation; its candidate cites the document and the code it names.
    let report = run_extraction(
        &test.store,
        &Scripted,
        &captures[0],
        &RunContext::for_tests(),
    )
    .expect("extract");
    assert_eq!(report.inserted, 1, "{report:?}");
    let summary: String = connection
        .query_row(
            "SELECT diff_summary FROM decision_candidates WHERE capture_id = ?1",
            [&captures[0]],
            |row| row.get(0),
        )
        .expect("candidate");
    assert!(summary.contains("docs/adr/0001-sqlite.md"), "{summary}");
    assert!(
        summary.contains("crates/storage-sqlite/src/store.rs"),
        "{summary}"
    );

    // Without a model nothing is invented from a document.
    let report = run_extraction(
        &test.store,
        &FakeCandidateExtractor,
        &captures[1],
        &RunContext::for_tests(),
    )
    .expect("offline");
    let _ = report;
    let offline: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM decision_candidates WHERE capture_id = ?1",
            [&captures[1]],
            |row| row.get(0),
        )
        .expect("count");
    assert_eq!(offline, 0);

    // A new version of the README is queued again.
    std::fs::write(
        repo.join("README.md"),
        "# Xemnas\n\n## Arquitetura\n\nDecidimos guardar tudo localmente em vez de na nuvem.\n",
    )
    .expect("edit");
    documents.index("docs-p").expect("reindex");
    assert_eq!(documents.propose("docs-p", 10).expect("changed"), 1);

    // A README with only install steps has no central part, and a changelog
    // is not a source of decisions: neither is queued.
    std::fs::write(
        repo.join("README.md"),
        "# Xemnas\n\n## Instalação\n\nBaixe o app.\n\n## Uso\n\nAbra.\n",
    )
    .expect("plain readme");
    std::fs::write(
        repo.join("docs/CHANGELOG.md"),
        "# Mudanças\n\nDecidimos tudo.\n",
    )
    .expect("changelog");
    documents.index("docs-p").expect("reindex plain");
    assert_eq!(documents.propose("docs-p", 10).expect("noise"), 0);
}
