//! Project documentation over SQLite: indexing the project folder, change
//! detection, listing and removal with the project.

mod support;

use application::documents::{DocumentError, DocumentKind, Documents};
use application::projects::{ProjectRecord, ProjectRepository, Projects};

fn project_with_docs(test: &support::TestStore) -> std::path::PathBuf {
    let repo = test.root.join("repo");
    std::fs::create_dir_all(repo.join("docs/adr")).expect("docs");
    std::fs::write(
        repo.join("README.md"),
        "# Xemnas\n\nGuarda decisões de engenharia localmente.\n",
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
