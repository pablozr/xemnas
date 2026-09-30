//! Context Claims over SQLite: creation rules, validity by date, retirement
//! and migration 0011.

mod support;

use application::claims::{Claims, ClaimsError, NewClaim};
use domain::claims::ClaimKind;
use rusqlite::Connection;

fn new_claim(project: &str, statement: &str, from: &str, until: Option<&str>) -> NewClaim {
    NewClaim {
        project_id: project.to_string(),
        kind: ClaimKind::Constraint,
        statement: statement.to_string(),
        valid_from: Some(from.to_string()),
        valid_until: until.map(str::to_string),
        source_decision_id: None,
    }
}

#[test]
fn create_normalizes_and_lists_by_date() {
    let test = support::open("claims-list", &["p1"]);
    let claims = Claims::new(test.store.clone());
    let windows = claims
        .create(new_claim("p1", "  Só Windows  ", "2026-01-01", None))
        .expect("create");
    assert_eq!(windows.statement, "Só Windows");
    assert_eq!(windows.valid_from, "2026-01-01T00:00:00Z");
    claims
        .create(new_claim(
            "p1",
            "Sem rede externa",
            "2026-03-01",
            Some("2026-06-01"),
        ))
        .expect("create");

    let statements = |as_of: Option<&str>| -> Vec<String> {
        claims
            .list("p1", as_of)
            .expect("list")
            .into_iter()
            .map(|claim| claim.statement)
            .collect()
    };
    assert_eq!(statements(None), vec!["Só Windows", "Sem rede externa"]);
    assert_eq!(statements(Some("2026-02-01")), vec!["Só Windows"]);
    assert_eq!(
        statements(Some("2026-04-15")),
        vec!["Só Windows", "Sem rede externa"]
    );
    assert_eq!(statements(Some("2026-06-01")), vec!["Só Windows"]);
    assert_eq!(
        claims
            .list("p1", Some("ontem"))
            .map_err(|error| error.code()),
        Err("invalid_date")
    );
}

#[test]
fn source_decision_must_exist_in_the_same_project() {
    let test = support::open("claims-source", &["p1", "p2"]);
    let own = support::decision(&test.store, "p1", "a", "Qual banco?", "SQLite");
    let foreign = support::decision(&test.store, "p2", "b", "Qual banco?", "Postgres");
    let claims = Claims::new(test.store.clone());
    let with_source = |source: &str| NewClaim {
        source_decision_id: Some(source.to_string()),
        ..new_claim("p1", "Banco local", "2026-01-01", None)
    };

    let created = claims.create(with_source(&own)).expect("same project");
    assert_eq!(created.source_decision_id.as_deref(), Some(own.as_str()));
    assert_eq!(
        claims.create(with_source(&foreign)),
        Err(ClaimsError::InvalidSource)
    );
    assert_eq!(
        claims.create(with_source("missing")),
        Err(ClaimsError::InvalidSource)
    );
    assert_eq!(
        claims.create(new_claim("nope", "x", "2026-01-01", None)),
        Err(ClaimsError::ProjectNotFound)
    );
    assert_eq!(
        claims
            .create(new_claim("p1", " ", "2026-01-01", None))
            .map_err(|error| error.code()),
        Err("empty_statement")
    );
}

#[test]
fn retire_closes_the_validity_and_keeps_the_claim() {
    let test = support::open("claims-retire", &["p1"]);
    let claims = Claims::new(test.store.clone());
    let claim = claims
        .create(new_claim("p1", "Só Windows", "2026-01-01", None))
        .expect("create");

    let retired = claims
        .retire(&claim.claim_id, Some("2026-05-01"))
        .expect("retire");
    assert_eq!(retired.valid_until.as_deref(), Some("2026-05-01T00:00:00Z"));
    assert_eq!(claims.list("p1", None).expect("list").len(), 1);
    assert!(claims
        .list("p1", Some("2026-05-02"))
        .expect("list")
        .is_empty());
    assert_eq!(
        claims
            .retire(&claim.claim_id, Some("2026-06-01"))
            .map_err(|error| error.code()),
        Err("already_ended")
    );
    assert_eq!(claims.retire("missing", None), Err(ClaimsError::NotFound));
}

#[test]
fn statements_are_indexed_for_search() {
    let test = support::open("claims-fts", &["p1"]);
    let claims = Claims::new(test.store.clone());
    let claim = claims
        .create(new_claim(
            "p1",
            "Toda API responde em JSON",
            "2026-01-01",
            None,
        ))
        .expect("create");
    let connection = Connection::open(test.root.join("app.db")).expect("raw");
    let found: String = connection
        .query_row(
            "SELECT claim_id FROM claims_fts WHERE claims_fts MATCH 'json'",
            [],
            |row| row.get(0),
        )
        .expect("indexed");
    assert_eq!(found, claim.claim_id);
}

#[test]
fn migration_0011_upgrades_a_version_10_database() {
    let test = support::open("claims-upgrade", &["p1"]);
    let database = test.root.join("app.db");
    {
        let connection = Connection::open(&database).expect("raw");
        connection
            .execute_batch(
                "DROP TABLE claims_fts; DROP TABLE context_claims; \
                 DELETE FROM schema_migrations WHERE version = 11;",
            )
            .expect("simulate version 10");
    }
    let reopened = storage_sqlite::SqliteStore::open(&database).expect("reopen and migrate");
    let claims = Claims::new(reopened);
    assert!(claims.list("p1", None).expect("list").is_empty());
    claims
        .create(new_claim("p1", "Depois do upgrade", "2026-01-01", None))
        .expect("create after upgrade");
}
