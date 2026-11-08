//! Project documentation over SQLite: indexing the project folder, change
//! detection, listing and removal with the project.

mod support;

use application::documents::{DocumentError, DocumentKind, Documents};
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

fn count(connection: &Connection, table: &str) -> i64 {
    let filter = if table == "jobs" {
        " WHERE kind = 'analyze_document'"
    } else {
        ""
    };
    connection
        .query_row(
            &format!("SELECT COUNT(*) FROM {table}{filter}"),
            [],
            |row| row.get(0),
        )
        .expect("count")
}

#[test]
fn documents_are_indexed_and_never_become_captures_or_jobs() {
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
    assert_eq!(documents.propose("docs-p", 10).expect("propose"), 0);

    // An edit refreshes the index, and still queues nothing.
    std::fs::write(
        repo.join("README.md"),
        "# Xemnas\n\n## Arquitetura\n\nDecidimos guardar tudo localmente em vez de na nuvem.\n",
    )
    .expect("edit");
    assert_eq!(documents.refresh_changed("docs-p", 10).expect("refresh"), 0);
    let readme = documents
        .list("docs-p")
        .expect("list")
        .into_iter()
        .find(|document| document.path == "README.md")
        .expect("readme");
    assert!(readme.headings.contains(&"Arquitetura".to_string()));

    let connection = Connection::open(test.root.join("app.db")).expect("raw");
    assert_eq!(count(&connection, "capture_receipts"), 0);
    assert_eq!(count(&connection, "capture_artifacts"), 0);
    assert_eq!(count(&connection, "jobs"), 0);
}

fn git(directory: &std::path::Path, args: &[&str]) -> bool {
    std::process::Command::new("git")
        .arg("-C")
        .arg(directory)
        .args(["-c", "user.name=t", "-c", "user.email=t@t"])
        .args(args)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn git_repo(path: &std::path::Path) {
    std::fs::create_dir_all(path).expect("dir");
    assert!(git(path, &["init", "-q"]));
    assert!(git(path, &["commit", "-q", "--allow-empty", "-m", "init"]));
}

const WORKTREE_ADR: &str = "# Perfil de turno\n\n## Decisão\nGuardar o perfil em SQLite, em \
                            crates/storage-sqlite/src/store.rs.\n";

#[test]
fn a_document_of_a_sibling_worktree_is_imported_and_indexed() {
    if !git(std::path::Path::new("."), &["--version"]) {
        eprintln!("skipped: git is unavailable");
        return;
    }
    let test = support::open("documents-import", &[]);
    let main = test.root.join("main");
    git_repo(&main);
    let sibling = test.root.join("sibling");
    assert!(git(
        &main,
        &["worktree", "add", "-q", &sibling.to_string_lossy()]
    ));
    test.store
        .insert(&ProjectRecord::new(
            "wt-p".into(),
            main.to_string_lossy().replace('\\', "/"),
            "2026-01-01T00:00:00Z".into(),
        ))
        .expect("project");
    let file = sibling.join("docs/adr/0014-turn-profile.md");
    std::fs::create_dir_all(file.parent().expect("parent")).expect("dirs");
    std::fs::write(&file, WORKTREE_ADR).expect("adr");
    let documents = Documents::new(test.store.clone());
    let identity = application::repo_identity::GitRepoIdentity;

    let first = documents
        .import_file(&identity, "wt-p", &file)
        .expect("import");
    assert_eq!(first.document.path, "docs/adr/0014-turn-profile.md");
    assert_eq!(first.document.kind, DocumentKind::Adr);
    assert!(!first.queued, "a document is indexed, not queued");

    let again = documents
        .import_file(&identity, "wt-p", &file)
        .expect("same content");
    assert!(!again.queued);
    assert_eq!(documents.list("wt-p").expect("list").len(), 1);

    // Reading the folder again keeps what was imported.
    documents.index("wt-p").expect("index");
    assert_eq!(documents.list("wt-p").expect("kept").len(), 1);

    // Changed content replaces the stored version.
    std::fs::write(&file, format!("{WORKTREE_ADR}\nAtualizado.\n")).expect("edit");
    let changed = documents
        .import_file(&identity, "wt-p", &file)
        .expect("changed");
    assert!(!changed.queued);
    let listed = documents.list("wt-p").expect("list");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].fingerprint, changed.document.fingerprint);

    let connection = Connection::open(test.root.join("app.db")).expect("raw");
    assert_eq!(count(&connection, "capture_receipts"), 0);
    assert_eq!(count(&connection, "jobs"), 0);
}

#[test]
fn imports_outside_the_repository_or_not_documents_are_refused_without_a_trace() {
    if !git(std::path::Path::new("."), &["--version"]) {
        eprintln!("skipped: git is unavailable");
        return;
    }
    let test = support::open("documents-import-refused", &[]);
    let main = test.root.join("main");
    git_repo(&main);
    let unrelated = test.root.join("unrelated");
    git_repo(&unrelated);
    test.store
        .insert(&ProjectRecord::new(
            "wt-p".into(),
            main.to_string_lossy().replace('\\', "/"),
            "2026-01-01T00:00:00Z".into(),
        ))
        .expect("project");
    std::fs::create_dir_all(unrelated.join("docs")).expect("dirs");
    std::fs::write(unrelated.join("docs/notes.md"), WORKTREE_ADR).expect("notes");
    std::fs::write(main.join("lib.rs"), "fn main() {}\n").expect("code");
    std::fs::create_dir_all(main.join("docs")).expect("dirs");
    std::fs::write(main.join("docs/blob.md"), [0x23, 0x20, 0x00, 0xff, 0xfe]).expect("binary");
    std::fs::write(main.join("docs/huge.md"), vec![b'a'; 600 * 1024]).expect("huge");
    let documents = Documents::new(test.store.clone());
    let identity = application::repo_identity::GitRepoIdentity;
    let import = |path: std::path::PathBuf| documents.import_file(&identity, "wt-p", &path);

    assert_eq!(
        import(unrelated.join("docs/notes.md")).unwrap_err(),
        DocumentError::OutsideRepository
    );
    assert_eq!(
        import(main.join("lib.rs")).unwrap_err(),
        DocumentError::NotDocumentation
    );
    assert_eq!(
        import(main.join("docs/blob.md")).unwrap_err(),
        DocumentError::NotText
    );
    assert_eq!(
        import(main.join("docs/huge.md")).unwrap_err(),
        DocumentError::NotText
    );
    assert_eq!(
        import(main.join("docs/missing.md")).unwrap_err(),
        DocumentError::FileUnavailable
    );
    assert!(documents.list("wt-p").expect("list").is_empty());
}

#[test]
fn archived_and_history_documents_are_still_indexed() {
    let test = support::open("documents-archived", &[]);
    let repo = test.root.join("repo");
    for dir in ["docs/adr", "docs/archive-x", "docs/historico", "docs/spike"] {
        std::fs::create_dir_all(repo.join(dir)).expect("docs");
    }
    let body = "# Titulo\n\n## Decisão\nDecidimos guardar tudo em SQLite em vez de um servidor, \
                em crates/storage-sqlite/src/store.rs.\n";
    for path in [
        "docs/adr/0001-sqlite.md",
        "docs/archive-x/0002-antigo.md",
        "docs/historico/0003-antigo.md",
        "docs/spike/0004-teste.md",
    ] {
        std::fs::write(repo.join(path), body).expect("doc");
    }
    test.store
        .insert(&ProjectRecord::new(
            "docs-p".into(),
            repo.to_string_lossy().replace('\\', "/"),
            "2026-01-01T00:00:00Z".into(),
        ))
        .expect("project");
    let documents = Documents::new(test.store.clone());
    assert_eq!(documents.index("docs-p").expect("index").total, 4);
    assert_eq!(documents.list("docs-p").expect("list").len(), 4);
    assert_eq!(documents.propose("docs-p", 10).expect("propose"), 0);
}
