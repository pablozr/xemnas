//! Robustness of the pipeline found by an end-to-end run: the map exists from
//! the first analysis and the reason a run failed is kept without content.

mod support;

use application::documents::Documents;
use application::extract::{failure_detail, ExtractError, FAILURE_DETAIL_MAX_CHARS};
use application::graph::{map_preparer, GraphStore};
use application::link_suggestions::candidate_components;
use application::projects::{ProjectRecord, ProjectRepository, Projects};

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
fn indexing_documents_prepares_the_map() {
    let test = support::open("robust-index", &[]);
    project_with_adr(&test);
    let documents =
        Documents::new(test.store.clone()).with_map_preparer(map_preparer(test.store.clone()));

    assert!(component_names(&test, "rb-p").is_empty());
    documents.index("rb-p").expect("index");
    assert_eq!(component_names(&test, "rb-p"), vec!["core", "workspace"]);
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
