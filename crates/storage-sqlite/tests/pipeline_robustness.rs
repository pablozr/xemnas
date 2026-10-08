//! Robustness of the pipeline found by an end-to-end run: the map exists from
//! the first analysis, a failed document analysis runs again (a bounded
//! number of times), and the reason a run failed is kept without content.

mod support;

use application::documents::{Documents, MAX_AUTOMATIC_ATTEMPTS};
use application::extract::{
    failure_detail, run_extraction, CandidateExtractor, CandidateProposal, DecisionEvidence,
    ExtractError, RelevanceSignal, RunContext, FAILURE_DETAIL_MAX_CHARS,
};
use application::graph::{map_preparer, GraphStore};
use application::jobs::{JobState, Jobs, ANALYZE_DOCUMENT_KIND};
use application::link_suggestions::candidate_components;
use application::projects::{ProjectRecord, ProjectRepository, Projects};
use rusqlite::Connection;

fn component_names(test: &support::TestStore, project: &str) -> Vec<String> {
    let entities = test.store.project_entities(project).expect("entities");
    let mut names: Vec<String> = candidate_components(&entities)
        .into_iter()
        .map(|entity| entity.name.clone())
        .collect();
    names.sort();
    names
}

#[test]
fn registering_a_cargo_workspace_gives_the_map_its_components() {
    let test = support::open("robust-cargo", &[]);
    let repo = test.root.join("cargo-repo");
    for name in ["core", "api"] {
        std::fs::create_dir_all(repo.join("crates").join(name)).expect("dirs");
        std::fs::write(
            repo.join("crates").join(name).join("Cargo.toml"),
            format!("[package]\nname = \"{name}\"\n"),
        )
        .expect("member");
    }
    std::fs::write(
        repo.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/*\"]\n",
    )
    .expect("root");

    let project = Projects::new(test.store.clone())
        .with_map_preparer(map_preparer(test.store.clone()))
        .register(&repo)
        .expect("register");

    // What a `suggest_links` job reads: it no longer returns early. The
    // workspace root is a component too.
    assert_eq!(
        component_names(&test, project.id().as_str()),
        vec!["api", "core", "workspace"]
    );
}

#[test]
fn registering_an_npm_workspace_gives_the_map_its_components() {
    let test = support::open("robust-npm", &[]);
    let repo = test.root.join("npm-repo");
    for name in ["web", "server"] {
        std::fs::create_dir_all(repo.join("packages").join(name)).expect("dirs");
        std::fs::write(
            repo.join("packages").join(name).join("package.json"),
            format!("{{\"name\": \"{name}\"}}"),
        )
        .expect("member");
    }
    std::fs::write(
        repo.join("package.json"),
        "{\"name\": \"root\", \"workspaces\": [\"packages/*\"]}",
    )
    .expect("root");

    let projects =
        Projects::new(test.store.clone()).with_map_preparer(map_preparer(test.store.clone()));
    let project = projects.register(&repo).expect("register");
    assert_eq!(
        component_names(&test, project.id().as_str()),
        vec!["server", "web", "workspace"]
    );

    // Idempotent: preparing again changes nothing.
    map_preparer(test.store.clone())(project.id().as_str());
    assert_eq!(
        component_names(&test, project.id().as_str()),
        vec!["server", "web", "workspace"]
    );
}

#[test]
fn registering_without_the_preparer_leaves_the_map_to_its_screen() {
    let test = support::open("robust-plain", &[]);
    let repo = test.root.join("plain-repo");
    std::fs::create_dir_all(repo.join("crates/core")).expect("dirs");
    std::fs::write(
        repo.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/*\"]\n",
    )
    .expect("root");
    std::fs::write(
        repo.join("crates/core/Cargo.toml"),
        "[package]\nname = \"core\"\n",
    )
    .expect("member");
    let project = Projects::new(test.store.clone())
        .register(&repo)
        .expect("register");
    assert!(component_names(&test, project.id().as_str()).is_empty());
}

fn project_with_adr(test: &support::TestStore) -> std::path::PathBuf {
    let repo = test.root.join("repo");
    std::fs::create_dir_all(repo.join("crates/core")).expect("dirs");
    std::fs::create_dir_all(repo.join("docs/adr")).expect("docs");
    std::fs::write(
        repo.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/*\"]\n",
    )
    .expect("root");
    std::fs::write(
        repo.join("crates/core/Cargo.toml"),
        "[package]\nname = \"core\"\n",
    )
    .expect("member");
    std::fs::write(
        repo.join("docs/adr/0001-sqlite.md"),
        "# Usar SQLite\n\n## Decisão\nGuardar tudo em SQLite, em crates/core/src/lib.rs. \
         SEGREDO-DO-DOCUMENTO.\n",
    )
    .expect("adr");
    test.store
        .insert(&ProjectRecord::new(
            "rb-p".into(),
            repo.to_string_lossy().replace('\\', "/"),
            "2026-01-01T00:00:00Z".into(),
        ))
        .expect("project");
    repo
}

#[test]
fn indexing_documents_prepares_the_map_before_the_analysis_they_start() {
    let test = support::open("robust-index", &[]);
    project_with_adr(&test);
    let documents =
        Documents::new(test.store.clone()).with_map_preparer(map_preparer(test.store.clone()));

    assert!(component_names(&test, "rb-p").is_empty());
    documents.index("rb-p").expect("index");
    assert_eq!(component_names(&test, "rb-p"), vec!["core", "workspace"]);
}

fn set_job(test: &support::TestStore, state: &str, attempts: i64) {
    Connection::open(test.root.join("app.db"))
        .expect("raw")
        .execute(
            "UPDATE jobs SET state = ?1, attempts = ?2, last_error = 'x' WHERE kind = ?3",
            rusqlite::params![state, attempts, ANALYZE_DOCUMENT_KIND],
        )
        .expect("update");
}

fn job_state(test: &support::TestStore) -> (String, Option<String>) {
    Connection::open(test.root.join("app.db"))
        .expect("raw")
        .query_row(
            "SELECT state, last_error FROM jobs WHERE kind = ?1",
            [ANALYZE_DOCUMENT_KIND],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("job")
}

#[test]
fn a_failed_document_analysis_is_requeued_a_bounded_number_of_times() {
    let test = support::open("robust-retry", &[]);
    project_with_adr(&test);
    let documents = Documents::new(test.store.clone());
    documents.index("rb-p").expect("index");

    assert_eq!(documents.propose("rb-p", 10).expect("first"), 1);
    assert_eq!(documents.propose("rb-p", 10).expect("queued"), 0);

    // Still waiting or running: left alone.
    set_job(&test, "running", 1);
    assert_eq!(documents.propose("rb-p", 10).expect("running"), 0);

    // Failed once: the next run queues it again, with the diagnostic cleared.
    set_job(&test, "failed", 1);
    assert_eq!(documents.propose("rb-p", 10).expect("retry"), 1);
    assert_eq!(job_state(&test), ("queued".to_string(), None));
    assert_eq!(documents.propose("rb-p", 10).expect("not twice"), 0);

    // Failed again and again: not endlessly.
    set_job(&test, "failed", i64::from(MAX_AUTOMATIC_ATTEMPTS));
    assert_eq!(documents.propose("rb-p", 10).expect("capped"), 0);
    assert_eq!(job_state(&test).0, "failed");

    // The person importing that very file asks again: it runs.
    let file = test.root.join("repo/docs/adr/0001-sqlite.md");
    let imported = documents
        .import_file(&application::repo_identity::GitRepoIdentity, "rb-p", &file)
        .expect("import");
    assert!(imported.queued);
    assert_eq!(job_state(&test), ("queued".to_string(), None));
}

#[test]
fn reprocess_requeues_a_failed_document_analysis() {
    let test = support::open("robust-reprocess", &[]);
    project_with_adr(&test);
    let documents = Documents::new(test.store.clone());
    documents.index("rb-p").expect("index");
    assert_eq!(documents.propose("rb-p", 10).expect("queued"), 1);
    set_job(&test, "failed", MAX_AUTOMATIC_ATTEMPTS.into());

    let id: String = Connection::open(test.root.join("app.db"))
        .expect("raw")
        .query_row(
            "SELECT id FROM jobs WHERE kind = ?1",
            [ANALYZE_DOCUMENT_KIND],
            |row| row.get(0),
        )
        .expect("id");
    Jobs::new(test.store.clone())
        .reprocess(&id)
        .expect("Reprocessar works for document analyses");
    assert_eq!(job_state(&test), (JobState::Queued.as_str().into(), None));
}

/// Fails the way a model does when its proposal breaks the contract.
struct Rejected;

impl CandidateExtractor for Rejected {
    fn extract(
        &self,
        _: &DecisionEvidence,
        _: &[RelevanceSignal],
    ) -> Result<Vec<CandidateProposal>, ExtractError> {
        Err(ExtractError::Validation(
            "qualificador sem citação literal verificável".into(),
        ))
    }
}

#[test]
fn a_failed_run_keeps_its_reason_and_never_the_content() {
    let test = support::open("robust-detail", &[]);
    project_with_adr(&test);
    let documents = Documents::new(test.store.clone());
    documents.index("rb-p").expect("index");
    assert_eq!(documents.propose("rb-p", 10).expect("queued"), 1);
    let connection = Connection::open(test.root.join("app.db")).expect("raw");
    let capture: String = connection
        .query_row(
            "SELECT payload FROM jobs WHERE kind = ?1",
            [ANALYZE_DOCUMENT_KIND],
            |row| row.get(0),
        )
        .expect("capture");

    let error = run_extraction(&test.store, &Rejected, &capture, &RunContext::for_tests())
        .expect_err("fails");
    assert_eq!(error.code(), "validation", "{error}");

    let (code, detail): (String, String) = connection
        .query_row(
            "SELECT error_code, error_detail FROM assessments WHERE outcome = 'failed'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("assessment");
    assert_eq!(code, "validation");
    assert_eq!(
        detail,
        "proposta inválida: qualificador sem citação literal verificável"
    );
    assert!(!detail.contains("SEGREDO-DO-DOCUMENTO"));

    // Progress of a capture reads it back for Diagnostics.
    let content: String = connection
        .query_row("SELECT content FROM capture_artifacts", [], |row| {
            row.get(0)
        })
        .expect("content");
    assert!(
        content.contains("SEGREDO-DO-DOCUMENTO"),
        "the fixture holds it"
    );
}

#[test]
fn the_failure_detail_is_one_bounded_line_with_secrets_masked() {
    // Built in pieces so no scanner mistakes the fixture for a real key.
    let fake_key = ["s", "k-abcdefghijklmnopqrstuvwxyz0123456789"].concat();
    let long = format!("falha{}{fake_key}", "\npalavra ".repeat(100));
    let detail = failure_detail(&ExtractError::Extractor(long));
    assert!(detail.chars().count() <= FAILURE_DETAIL_MAX_CHARS);
    assert!(!detail.contains('\n'));
    assert!(detail.ends_with('…'));

    let short = failure_detail(&ExtractError::Extractor(format!("chave {fake_key} vazou")));
    assert!(!short.contains("abcdefghijklmnop"), "{short}");
    assert!(short.ends_with("vazou"));
}
