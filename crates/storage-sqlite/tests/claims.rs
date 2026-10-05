//! Context Claims over SQLite: creation rules, validity by date, retirement
//! and migration 0011.

mod support;

use application::claims::{Claims, ClaimsError, NewClaim};
use domain::claims::ClaimKind;
use rusqlite::Connection;

fn new_claim(project: &str, statement: &str, from: &str, until: Option<&str>) -> NewClaim {
    NewClaim {
        source_version: None,
        qualifiers: Vec::new(),
        project_id: project.to_string(),
        kind: ClaimKind::Constraint,
        statement: statement.to_string(),
        valid_from: Some(from.to_string()),
        valid_until: until.map(str::to_string),
        source_decision_id: None,
    }
}

#[test]
fn qualifiers_survive_revision_and_derived_claim_cannot_override_them() {
    use application::decisions::{DecisionEdits, Decisions};
    use application::qualifiers::{KnowledgeQualifier, QualifierKind};
    let test = support::open("qualifier-inheritance", &["p1"]);
    let id = support::decision(&test.store, "p1", "qualified", "Pergunta?", "Escolha");
    let decisions = Decisions::new(test.store.clone());
    let qualifier = KnowledgeQualifier {
        kind: QualifierKind::Validation,
        text: "não executado — declaração do revisor".into(),
        artifact_id: None,
    };
    let revised = decisions
        .revise(
            &id,
            DecisionEdits {
                qualifiers: Some(vec![qualifier.clone()]),
                ..DecisionEdits::default()
            },
        )
        .unwrap();
    assert_eq!(revised.qualifiers, vec![qualifier.clone()]);
    assert_eq!(revised.revisions[0].qualifiers, vec![qualifier.clone()]);
    let mut input = new_claim("p1", "Regra derivada", "2026-01-01", None);
    input.source_decision_id = Some(id.clone());
    // Caller/model cannot remove the original qualification with an empty list.
    let claim = Claims::new(test.store.clone()).create(input).unwrap();
    assert_eq!(claim.source_version, Some(2));
    assert_eq!(
        application::qualifiers::decode(&claim.qualifiers).unwrap(),
        vec![qualifier]
    );
    let legacy = revised
        .revisions
        .iter()
        .find(|revision| revision.version == 1)
        .unwrap();
    assert!(legacy.qualifiers.is_empty());
    assert!(decisions
        .revise_version(
            &id,
            1,
            DecisionEdits {
                qualifiers: Some(Vec::new()),
                ..DecisionEdits::default()
            }
        )
        .is_err());
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
                  ALTER TABLE decision_candidates DROP COLUMN qualifiers; \
                  ALTER TABLE engineering_decisions DROP COLUMN qualifiers; \
                  ALTER TABLE decision_revisions DROP COLUMN qualifiers; \
                  ALTER TABLE claim_suggestions DROP COLUMN qualifiers; \
                  ALTER TABLE claim_suggestions DROP COLUMN inherited_scope; \
                  ALTER TABLE claim_suggestions DROP COLUMN source_version; \
                  DELETE FROM schema_migrations WHERE version IN (11, 25, 27, 28);",
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

#[test]
fn migration_21_preserves_real_legacy_rows_and_defaults_qualifiers() {
    let test = support::open("qualifier-legacy-upgrade", &["p1"]);
    let id = support::decision(&test.store, "p1", "legacy", "Pergunta", "Escolha");
    let claim = Claims::new(test.store.clone())
        .create(new_claim("p1", "Regra", "2026-01-01", None))
        .unwrap();
    let database = test.root.join("app.db");
    let connection = Connection::open(&database).unwrap();
    connection
        .execute_batch(
            "ALTER TABLE decision_candidates DROP COLUMN qualifiers;
        ALTER TABLE engineering_decisions DROP COLUMN qualifiers;
        ALTER TABLE decision_revisions DROP COLUMN qualifiers;
        ALTER TABLE context_claims DROP COLUMN qualifiers;
        ALTER TABLE claim_suggestions DROP COLUMN qualifiers;
        DELETE FROM schema_migrations WHERE version = 25;",
        )
        .unwrap();
    drop(connection);
    let upgraded = storage_sqlite::SqliteStore::open(&database).unwrap();
    use application::claims::ClaimStore;
    use application::decisions::DecisionStore;
    assert_eq!(upgraded.get(&id).unwrap().unwrap().qualifiers, "[]");
    assert_eq!(upgraded.revisions(&id).unwrap()[0].qualifiers, "[]");
    assert_eq!(
        upgraded
            .get_claim(&claim.claim_id)
            .unwrap()
            .unwrap()
            .qualifiers,
        "[]"
    );
}

#[test]
fn source_change_before_insert_is_rejected_without_claim() {
    use application::claims::ClaimStore;
    use application::decisions::{DecisionEdits, Decisions};
    let test = support::open("qualifier-source-cas", &["p1"]);
    let id = support::decision(&test.store, "p1", "cas", "Pergunta", "Escolha");
    let claims = Claims::new(test.store.clone());
    let mut input = new_claim("p1", "Regra original", "2026-01-01", None);
    input.source_decision_id = Some(id.clone());
    let mut stale = claims.create(input).unwrap();
    stale.claim_id = "stale-not-inserted".into();
    Decisions::new(test.store.clone())
        .revise(
            &id,
            DecisionEdits {
                scope: Some(vec!["somente teste".into()]),
                ..DecisionEdits::default()
            },
        )
        .unwrap();
    assert!(matches!(
        test.store.insert_claim_versioned(&stale, Some(1)),
        Err(ClaimsError::InvalidSource)
    ));
    assert!(test.store.get_claim(&stale.claim_id).unwrap().is_none());
}

#[test]
fn full_qualifiers_and_fifty_scope_entries_are_inherited_independently() {
    use application::decisions::{DecisionEdits, Decisions};
    use application::qualifiers::{KnowledgeQualifier, QualifierKind};
    let test = support::open("full-source-snapshot", &["p1"]);
    let id = support::decision(&test.store, "p1", "full", "Question", "Choice");
    let qualifiers = (0..16)
        .map(|n| KnowledgeQualifier {
            kind: QualifierKind::Validation,
            text: format!("declaração {n}"),
            artifact_id: None,
        })
        .collect::<Vec<_>>();
    let scope = (0..50).map(|n| format!("escopo {n}")).collect::<Vec<_>>();
    Decisions::new(test.store.clone())
        .revise(
            &id,
            DecisionEdits {
                qualifiers: Some(qualifiers.clone()),
                scope: Some(scope.clone()),
                ..DecisionEdits::default()
            },
        )
        .unwrap();
    let mut input = new_claim("p1", "Regra", "2026-01-01", None);
    input.source_decision_id = Some(id);
    input.source_version = Some(2);
    let claim = Claims::new(test.store.clone()).create(input).unwrap();
    assert_eq!(
        application::qualifiers::decode(&claim.qualifiers).unwrap(),
        qualifiers
    );
    assert_eq!(
        claim.inherited_scope,
        format!(
            "[{}]",
            scope
                .iter()
                .map(|s| format!("\"{s}\""))
                .collect::<Vec<_>>()
                .join(",")
        )
    );
}

#[test]
fn unchanged_citation_survives_edit_but_changed_text_is_a_declaration() {
    use application::decisions::{DecisionEdits, Decisions};
    use application::qualifiers::{KnowledgeQualifier, QualifierKind};
    let test = support::open("preserve-citation", &["p1"]);
    let id = support::decision(&test.store, "p1", "citation", "Question", "Choice");
    let db = Connection::open(test.root.join("app.db")).unwrap();
    db.execute(
        "UPDATE engineering_decisions SET qualifiers=?2 WHERE decision_id=?1",
        [
            &id,
            "[{\"kind\":\"validation\",\"text\":\"original excerpt\",\"artifact_id\":\"art-p1\"}]",
        ],
    )
    .unwrap();
    let decisions = Decisions::new(test.store.clone());
    let qualifier = KnowledgeQualifier {
        kind: QualifierKind::Validation,
        text: "original excerpt".into(),
        artifact_id: Some("spoof".into()),
    };
    let edited = decisions
        .revise(
            &id,
            DecisionEdits {
                qualifiers: Some(vec![qualifier.clone()]),
                ..DecisionEdits::default()
            },
        )
        .unwrap();
    assert_eq!(edited.qualifiers[0].artifact_id.as_deref(), Some("art-p1"));
    let mut changed = qualifier;
    changed.text = "new declaration".into();
    let edited = decisions
        .revise(
            &id,
            DecisionEdits {
                qualifiers: Some(vec![changed]),
                ..DecisionEdits::default()
            },
        )
        .unwrap();
    assert!(edited.qualifiers[0].artifact_id.is_none());
}

#[test]
fn migration_23_to_24_keeps_legacy_claim_and_suggestion_without_inferred_version() {
    use application::claim_suggestions::{ClaimSuggestionRecord, ClaimSuggestionStore};
    use application::claims::ClaimStore;
    let test = support::open("claim-version-24-upgrade", &["p1"]);
    let id = support::decision(&test.store, "p1", "version24", "Question", "Choice");
    let mut input = new_claim("p1", "Legacy rule", "2026-01-01", None);
    input.source_decision_id = Some(id.clone());
    input.source_version = Some(1);
    let claim = Claims::new(test.store.clone()).create(input).unwrap();
    assert_eq!(
        test.store
            .get_claim(&claim.claim_id)
            .unwrap()
            .unwrap()
            .source_version,
        Some(1)
    );
    let suggestion = ClaimSuggestionRecord {
        suggestion_id: "legacy-suggestion".into(),
        project_id: "p1".into(),
        decision_id: id,
        kind: ClaimKind::Constraint,
        statement: "Suggested rule".into(),
        quote: "Choice".into(),
        created_at: "2026-01-01T00:00:00Z".into(),
        qualifiers: "[]".into(),
        inherited_scope: "[]".into(),
        source_version: Some(1),
    };
    assert!(test.store.insert_claim_suggestion(&suggestion).unwrap());
    let database = test.root.join("app.db");
    let db = Connection::open(&database).unwrap();
    db.execute_batch(
        "ALTER TABLE context_claims DROP COLUMN source_version;
        DELETE FROM schema_migrations WHERE version=28;
        UPDATE claim_suggestions SET source_version=NULL WHERE suggestion_id='legacy-suggestion';",
    )
    .unwrap();
    drop(db);
    let upgraded = storage_sqlite::SqliteStore::open(&database).unwrap();
    let legacy = upgraded.get_claim(&claim.claim_id).unwrap().unwrap();
    assert_eq!(legacy.statement, "Legacy rule");
    assert_eq!(legacy.source_version, None);
    assert_eq!(legacy.source_decision_id, claim.source_decision_id);
    let pending = upgraded
        .pending_claim_suggestion("legacy-suggestion")
        .unwrap()
        .unwrap();
    assert_eq!(pending.source_version, None);
    assert_eq!(pending.statement, "Suggested rule");
    let db = Connection::open(&database).unwrap();
    let count: i64 = db
        .query_row("SELECT COUNT(*) FROM schema_migrations", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 39); // Version 7 intentionally has no registered migration.
}
