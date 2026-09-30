//! End-to-end Project persistence tests: identity and location survive a
//! restart, removing tracking never touches the directory on disk, and the
//! canonicalization rules collapse textual variants into one row.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use application::projects::{ProjectError, Projects};
use storage_sqlite::{default_data_dir, default_db_path, SqliteStore};

/// Creates a unique temporary directory for a test.
fn temporary_directory(tag: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let directory = std::env::temp_dir().join(format!(
        "xemnas-projects-{tag}-{}-{nanos}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir_all(&directory).expect("create temporary directory");
    directory
}

/// Creates a temporary project directory inside `root` and returns its path.
fn tracked_directory(root: &Path, name: &str) -> PathBuf {
    let directory = root.join(name);
    std::fs::create_dir_all(&directory).expect("create tracked directory");
    directory
}

#[test]
fn location_and_identity_survive_reopen() {
    let root = temporary_directory("restart");
    let database = root.join("xemnas.sqlite3");
    let project_directory = tracked_directory(&root, "tracked");

    let (id, location) = {
        let store = SqliteStore::open(&database).expect("open store");
        let projects = Projects::new(store);
        let project = projects
            .register(&project_directory)
            .expect("register project");
        (
            project.id().as_str().to_string(),
            project.location().to_string(),
        )
    };

    let store = SqliteStore::open(&database).expect("reopen store");
    let projects = Projects::new(store);
    let listed = projects.list().expect("list projects");

    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id().as_str(), id);
    assert_eq!(listed[0].location().to_string(), location);
    assert_eq!(
        projects
            .get(&id)
            .expect("get project")
            .location()
            .to_string(),
        location
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn removing_tracking_keeps_directory_on_disk() {
    let root = temporary_directory("remove");
    let database = root.join("xemnas.sqlite3");
    let project_directory = tracked_directory(&root, "kept");
    let sentinel = project_directory.join("keep-me.txt");
    std::fs::write(&sentinel, b"do not delete").expect("write sentinel");

    let store = SqliteStore::open(&database).expect("open store");
    let projects = Projects::new(store);
    let project = projects
        .register(&project_directory)
        .expect("register project");

    assert!(projects.remove(project.id().as_str()).expect("remove"));
    assert!(sentinel.exists(), "sentinel file must survive removal");
    assert!(project_directory.exists(), "directory must survive removal");
    assert!(projects.list().expect("list").is_empty());
    assert_eq!(
        projects.get(project.id().as_str()),
        Err(ProjectError::NotFound)
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn duplicate_location_forms_are_rejected() {
    let root = temporary_directory("duplicate");
    let database = root.join("xemnas.sqlite3");
    let project_directory = tracked_directory(&root, "project");
    let store = SqliteStore::open(&database).expect("open store");
    let projects = Projects::new(store);

    projects
        .register(&project_directory)
        .expect("first registration");

    let trailing_separator = PathBuf::from(format!(
        "{}{}",
        project_directory.display(),
        std::path::MAIN_SEPARATOR
    ));
    assert_eq!(
        projects.register(&trailing_separator),
        Err(ProjectError::AlreadyRegistered)
    );

    let nested = project_directory.join("nested");
    std::fs::create_dir_all(&nested).expect("create nested");
    let parent_via_dotdot = nested.join("..");
    assert_eq!(
        projects.register(&parent_via_dotdot),
        Err(ProjectError::AlreadyRegistered)
    );

    #[cfg(windows)]
    {
        let upper_cased = PathBuf::from(project_directory.to_string_lossy().to_uppercase());
        assert_eq!(
            projects.register(&upper_cased),
            Err(ProjectError::AlreadyRegistered)
        );
    }

    assert_eq!(projects.list().expect("list").len(), 1);

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn invalid_location_is_rejected_and_nothing_persisted() {
    let root = temporary_directory("invalid");
    let database = root.join("xemnas.sqlite3");
    let store = SqliteStore::open(&database).expect("open store");
    let projects = Projects::new(store);

    let missing = root.join("does-not-exist");
    assert_eq!(
        projects.register(&missing),
        Err(ProjectError::InvalidLocation)
    );

    let file = root.join("a-file.txt");
    std::fs::write(&file, b"not a directory").expect("write file");
    assert_eq!(projects.register(&file), Err(ProjectError::InvalidLocation));

    assert!(projects.list().expect("list").is_empty());

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn migration_is_idempotent_on_reopen() {
    let root = temporary_directory("migration");
    let database = root.join("xemnas.sqlite3");

    SqliteStore::open(&database).expect("first open");
    SqliteStore::open(&database).expect("second open");

    let connection = rusqlite::Connection::open(&database).expect("open raw connection");
    let versions: i64 = connection
        .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .expect("count migrations");
    let distinct: i64 = connection
        .query_row(
            "SELECT COUNT(DISTINCT version) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
        .expect("count distinct migrations");
    assert_eq!(versions, distinct, "no migration may be recorded twice");
    assert!(versions >= 2, "0001 and 0002 must both be applied");

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn default_data_dir_respects_environment_override() {
    let expected = temporary_directory("data-dir");
    std::env::set_var("XEMNAS_DATA_DIR", &expected);
    let actual = default_data_dir();
    let database = default_db_path();
    std::env::remove_var("XEMNAS_DATA_DIR");

    assert_eq!(actual, expected);
    assert_eq!(database, expected.join("state").join("app.db"));
    let _ = std::fs::remove_dir_all(&expected);
}
