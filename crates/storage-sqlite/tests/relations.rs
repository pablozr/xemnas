//! Decision relations over SQLite: supersession, rules and migration 0010.

mod support;

use application::decisions::{DecisionStatus, Decisions, DecisionsError};
use application::relations::{DecisionRelations, RelationDirection};
use domain::relations::RelationKind;
use rusqlite::Connection;

#[test]
fn supersede_marks_the_older_decision_and_keeps_both() {
    let test = support::open("supersede", &["p1"]);
    let older = support::decision(&test.store, "p1", "old", "Qual banco?", "Postgres");
    let newer = support::decision(&test.store, "p1", "new", "Qual banco?", "SQLite");
    let relations = DecisionRelations::new(test.store.clone());

    let view = relations.supersede(&newer, &older).expect("supersede");
    assert_eq!(view.kind, RelationKind::Supersedes);
    assert_eq!(view.other_id, older);

    let decisions = Decisions::new(test.store.clone());
    let old_detail = decisions.detail(&older).expect("older still readable");
    assert_eq!(old_detail.summary.status, DecisionStatus::Superseded);
    assert_eq!(
        decisions.detail(&newer).expect("newer").summary.status,
        DecisionStatus::Accepted
    );

    let of_older = relations.of(&older).expect("relations of older");
    assert_eq!(of_older.len(), 1);
    assert_eq!(of_older[0].direction, RelationDirection::Incoming);
    assert_eq!(of_older[0].other_id, newer);
    let of_newer = relations.of(&newer).expect("relations of newer");
    assert_eq!(of_newer[0].direction, RelationDirection::Outgoing);
}

#[test]
fn a_superseded_decision_cannot_be_superseded_again_or_revived() {
    let test = support::open("twice", &["p1"]);
    let a = support::decision(&test.store, "p1", "a", "q", "a");
    let b = support::decision(&test.store, "p1", "b", "q", "b");
    let c = support::decision(&test.store, "p1", "c", "q", "c");
    let relations = DecisionRelations::new(test.store.clone());
    relations.supersede(&b, &a).expect("b supersedes a");

    assert_eq!(relations.supersede(&c, &a), Err(DecisionsError::Conflict));
    assert_eq!(
        relations.relate(&a, &c, RelationKind::DependsOn),
        Err(DecisionsError::Conflict),
        "a superseded decision takes part in no new relation"
    );
}

#[test]
fn domain_rules_and_project_boundaries_are_enforced() {
    let test = support::open("rules", &["p1", "p2"]);
    let a = support::decision(&test.store, "p1", "a", "q", "a");
    let b = support::decision(&test.store, "p1", "b", "q", "b");
    let other = support::decision(&test.store, "p2", "x", "q", "x");
    let relations = DecisionRelations::new(test.store.clone());

    relations
        .relate(&a, &b, RelationKind::DependsOn)
        .expect("a depends on b");
    let code = |result: Result<_, DecisionsError>| result.map_err(|error| error.code()).map(|_| ());
    assert_eq!(
        code(relations.relate(&b, &a, RelationKind::DependsOn)),
        Err("invalid_relation"),
        "cycle"
    );
    assert_eq!(
        code(relations.relate(&a, &b, RelationKind::DependsOn)),
        Err("invalid_relation"),
        "duplicate"
    );
    assert_eq!(
        code(relations.relate(&a, &a, RelationKind::ConflictsWith)),
        Err("invalid_relation"),
        "self reference"
    );
    assert_eq!(
        code(relations.relate(&a, &other, RelationKind::ConflictsWith)),
        Err("invalid_relation"),
        "different projects"
    );
    assert_eq!(
        code(relations.relate(&a, "missing", RelationKind::ConflictsWith)),
        Err("not_found")
    );

    relations
        .relate(&b, &a, RelationKind::ConflictsWith)
        .expect("conflict is symmetric");
    assert_eq!(
        code(relations.relate(&a, &b, RelationKind::ConflictsWith)),
        Err("invalid_relation"),
        "the reversed symmetric relation is a duplicate"
    );
}

#[test]
fn the_database_rejects_a_second_successor_and_self_relations() {
    let test = support::open("constraints", &["p1"]);
    let a = support::decision(&test.store, "p1", "a", "q", "a");
    let b = support::decision(&test.store, "p1", "b", "q", "b");
    let c = support::decision(&test.store, "p1", "c", "q", "c");
    let connection = Connection::open(test.root.join("app.db")).expect("raw");
    let insert = |from: &str, to: &str| {
        connection.execute(
            "INSERT INTO decision_relations VALUES (?1, ?2, 'supersedes', '2026-01-01T00:00:00Z')",
            [from, to],
        )
    };
    insert(&b, &a).expect("first successor");
    assert!(insert(&c, &a).is_err(), "only one successor per decision");
    assert!(insert(&a, &a).is_err(), "no self relation");
}

#[test]
fn migration_0010_upgrades_a_version_9_database() {
    let test = support::open("upgrade", &["p1"]);
    let database = test.root.join("app.db");
    let decision = support::decision(&test.store, "p1", "a", "q", "a");
    {
        let connection = Connection::open(&database).expect("raw");
        connection
            .execute_batch(
                "DROP TABLE decision_relations; DELETE FROM schema_migrations WHERE version = 10;",
            )
            .expect("simulate version 9");
    }
    let reopened = storage_sqlite::SqliteStore::open(&database).expect("reopen and migrate");
    let relations = DecisionRelations::new(reopened.clone());
    assert!(relations.of(&decision).expect("relations").is_empty());
    let version: i64 = Connection::open(&database)
        .expect("raw")
        .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .expect("version");
    assert_eq!(version, 41);
    assert_eq!(
        Decisions::new(reopened)
            .detail(&decision)
            .expect("existing decision survives")
            .summary
            .status,
        DecisionStatus::Accepted
    );
}
