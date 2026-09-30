//! Injection audit over SQLite: per-session, per-mode delivery and migration 0012.

mod support;

use std::collections::BTreeSet;

use application::injection::{
    DeliveredItem, InjectionMode, InjectionRecord, InjectionStore, ItemKind,
};
use rusqlite::Connection;

fn item(kind: ItemKind, id: &str, version: i64) -> DeliveredItem {
    DeliveredItem {
        kind,
        id: id.to_string(),
        version,
    }
}

fn record(
    id: &str,
    session: &str,
    mode: InjectionMode,
    items: Vec<DeliveredItem>,
) -> InjectionRecord {
    InjectionRecord {
        injection_id: id.to_string(),
        session_id: session.to_string(),
        project_id: "p1".to_string(),
        mode,
        tokens: 42,
        omitted: 1,
        created_at: "2026-09-30T00:00:00Z".to_string(),
        items,
    }
}

#[test]
fn delivered_items_are_scoped_by_session_and_mode() {
    let test = support::open("injections", &["p1"]);
    let store = &test.store;
    store
        .record_injection(&record(
            "i1",
            "s1",
            InjectionMode::Inject,
            vec![
                item(ItemKind::Decision, "d1", 1),
                item(ItemKind::Claim, "c1", 1),
            ],
        ))
        .expect("record");
    store
        .record_injection(&record(
            "i2",
            "s1",
            InjectionMode::Inject,
            vec![item(ItemKind::Decision, "d1", 2)],
        ))
        .expect("record");
    store
        .record_injection(&record(
            "i3",
            "s1",
            InjectionMode::Shadow,
            vec![item(ItemKind::Decision, "d9", 1)],
        ))
        .expect("record");

    let expected: BTreeSet<DeliveredItem> = [
        item(ItemKind::Decision, "d1", 1),
        item(ItemKind::Decision, "d1", 2),
        item(ItemKind::Claim, "c1", 1),
    ]
    .into_iter()
    .collect();
    assert_eq!(
        store
            .delivered("s1", InjectionMode::Inject)
            .expect("delivered"),
        expected
    );
    assert_eq!(
        store
            .delivered("s1", InjectionMode::Shadow)
            .expect("delivered")
            .len(),
        1
    );
    assert!(store
        .delivered("s2", InjectionMode::Inject)
        .expect("delivered")
        .is_empty());
}

#[test]
fn the_audit_never_stores_prompt_text() {
    let test = support::open("injections-columns", &["p1"]);
    let connection = Connection::open(test.root.join("app.db")).expect("raw");
    let columns: Vec<String> = ["context_injections", "context_injection_items"]
        .iter()
        .flat_map(|table| {
            let mut statement = connection
                .prepare(&format!("SELECT name FROM pragma_table_info('{table}')"))
                .expect("prepare");
            statement
                .query_map([], |row| row.get::<_, String>(0))
                .expect("query")
                .collect::<Result<Vec<_>, _>>()
                .expect("collect")
        })
        .collect();
    assert!(
        columns
            .iter()
            .all(|column| !column.contains("prompt") && !column.contains("text")),
        "{columns:?}"
    );
}

#[test]
fn migration_0012_upgrades_a_version_11_database() {
    let test = support::open("injections-upgrade", &["p1"]);
    let database = test.root.join("app.db");
    Connection::open(&database)
        .expect("raw")
        .execute_batch(
            "DROP TABLE context_injection_items; DROP TABLE context_injections; \
             DELETE FROM schema_migrations WHERE version = 12;",
        )
        .expect("simulate version 11");
    let reopened = storage_sqlite::SqliteStore::open(&database).expect("reopen and migrate");
    reopened
        .record_injection(&record(
            "i1",
            "s1",
            InjectionMode::Shadow,
            vec![item(ItemKind::Claim, "c1", 1)],
        ))
        .expect("record after upgrade");
}
