//! Per-project context settings over SQLite and migration 0013.

mod support;

use application::context_settings::{ContextMode, ContextSettings};
use application::projects::Projects;
use rusqlite::Connection;

#[test]
fn settings_default_to_off_and_are_saved_per_project() {
    let test = support::open("settings", &["p1", "p2"]);
    let settings = ContextSettings::new(test.store.clone());

    let initial = settings.get("p1").expect("get");
    assert_eq!(initial.mode, ContextMode::Off);
    assert_eq!((initial.budget_tokens, initial.updated_at), (None, None));

    let saved = settings
        .set("p1", ContextMode::Shadow, Some(150))
        .expect("set");
    assert!(saved.updated_at.is_some());
    let read = settings.get("p1").expect("get");
    assert_eq!(
        (read.mode, read.budget_tokens),
        (ContextMode::Shadow, Some(150))
    );

    settings
        .set("p1", ContextMode::Inject, None)
        .expect("change");
    let read = settings.get("p1").expect("get");
    assert_eq!((read.mode, read.budget_tokens), (ContextMode::Inject, None));
    assert_eq!(settings.get("p2").expect("other").mode, ContextMode::Off);
}

#[test]
fn invalid_budget_and_unknown_projects_are_rejected() {
    let test = support::open("settings-invalid", &["p1"]);
    let settings = ContextSettings::new(test.store.clone());
    let code = |result: Result<_, application::context::ContextError>| {
        result.map(|_| ()).map_err(|error| error.code())
    };
    assert_eq!(
        code(settings.set("p1", ContextMode::Inject, Some(10))),
        Err("invalid_request")
    );
    assert_eq!(
        code(settings.set("p1", ContextMode::Inject, Some(5_000))),
        Err("invalid_request")
    );
    assert_eq!(
        code(settings.set("nope", ContextMode::Inject, None)),
        Err("project_not_found")
    );
    assert_eq!(code(settings.get("nope")), Err("project_not_found"));
}

#[test]
fn settings_go_away_with_the_project() {
    let test = support::open("settings-removal", &["p1"]);
    ContextSettings::new(test.store.clone())
        .set("p1", ContextMode::Inject, None)
        .expect("set");
    Projects::new(test.store.clone())
        .remove_with_data("p1", "p1")
        .expect("remove");
    let rows: i64 = Connection::open(test.root.join("app.db"))
        .expect("raw")
        .query_row("SELECT COUNT(*) FROM project_context_settings", [], |row| {
            row.get(0)
        })
        .expect("count");
    assert_eq!(rows, 0);
}

#[test]
fn migration_0013_upgrades_a_version_12_database() {
    let test = support::open("settings-upgrade", &["p1"]);
    let database = test.root.join("app.db");
    Connection::open(&database)
        .expect("raw")
        .execute_batch(
            "DROP TABLE project_context_settings; DELETE FROM schema_migrations WHERE version = 13;",
        )
        .expect("simulate version 12");
    let reopened = storage_sqlite::SqliteStore::open(&database).expect("reopen and migrate");
    let settings = ContextSettings::new(reopened);
    assert_eq!(settings.get("p1").expect("get").mode, ContextMode::Off);
    settings
        .set("p1", ContextMode::Shadow, None)
        .expect("set after upgrade");
}
