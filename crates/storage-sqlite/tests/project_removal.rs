//! Removing a project: plain removal refuses data, removal with data needs the
//! typed name, deletes everything of that project and nothing of the others.

mod support;

use application::claims::{Claims, NewClaim};
use application::injection::{
    DeliveredItem, InjectionMode, InjectionRecord, InjectionStore, ItemKind,
};
use application::projects::{Projects, RemovalError, RemovalImpact};
use application::relations::DecisionRelations;
use domain::claims::ClaimKind;
use rusqlite::Connection;

fn populate(test: &support::TestStore, project: &str) {
    let a = support::decision(&test.store, project, &format!("{project}-a"), "q", "a");
    let b = support::decision(&test.store, project, &format!("{project}-b"), "q", "b");
    DecisionRelations::new(test.store.clone())
        .supersede(&b, &a)
        .expect("supersede");
    Claims::new(test.store.clone())
        .create(NewClaim {
            source_version: None,
            qualifiers: Vec::new(),
            project_id: project.to_string(),
            kind: ClaimKind::Constraint,
            statement: "Só Windows".to_string(),
            valid_from: Some("2026-01-01".to_string()),
            valid_until: None,
            source_decision_id: Some(b),
        })
        .expect("claim");
    test.store
        .record_injection(&InjectionRecord {
            injection_id: format!("inj-{project}"),
            session_id: "s1".to_string(),
            project_id: project.to_string(),
            mode: InjectionMode::Inject,
            tokens: 10,
            omitted: 0,
            created_at: "2026-09-30T00:00:00Z".to_string(),
            items: vec![DeliveredItem {
                kind: ItemKind::Claim,
                id: "c".to_string(),
                version: 1,
            }],
        })
        .expect("injection");
}

fn count(connection: &Connection, table: &str) -> i64 {
    connection
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .expect("count")
}

#[test]
fn removal_with_data_deletes_only_that_project_after_the_typed_name() {
    let test = support::open("removal", &["p1", "p2"]);
    populate(&test, "p1");
    populate(&test, "p2");
    let projects = Projects::new(test.store.clone());

    let impact = projects.removal_impact("p1").expect("impact");
    assert_eq!(
        impact,
        RemovalImpact {
            decisions: 2,
            candidates: 2,
            claims: 1,
            captures: 1,
            injections: 1,
            entities: 0,
        }
    );
    assert!(
        projects.remove("p1").is_err(),
        "plain removal keeps the data"
    );
    assert_eq!(
        projects.remove_with_data("p1", "p2"),
        Err(RemovalError::ConfirmationMismatch)
    );
    assert_eq!(projects.removal_impact("p1").expect("still there"), impact);

    assert_eq!(
        projects.remove_with_data("p1", " p1 ").expect("remove"),
        impact
    );
    assert!(
        projects.removal_impact("p1").is_err(),
        "the project is gone"
    );

    let connection = Connection::open(test.root.join("app.db")).expect("raw");
    for (table, remaining) in [
        ("projects", 1),
        ("engineering_decisions", 2),
        ("decision_revisions", 2),
        ("evidence_links", 2),
        ("decision_relations", 1),
        ("decisions_fts", 2),
        ("decision_candidates", 2),
        ("context_claims", 1),
        ("claims_fts", 1),
        ("context_injections", 1),
        ("context_injection_items", 1),
        ("capture_receipts", 1),
        ("capture_artifacts", 1),
        ("adapter_checkpoints", 1),
        ("jobs", 2),
    ] {
        assert_eq!(count(&connection, table), remaining, "{table}");
    }
    assert_eq!(
        projects
            .removal_impact("p2")
            .expect("other project intact")
            .decisions,
        2
    );
}

#[test]
fn empty_projects_still_use_plain_removal_and_unknown_ids_fail() {
    let test = support::open("removal-empty", &["p1"]);
    let projects = Projects::new(test.store.clone());
    assert_eq!(
        projects.removal_impact("p1").expect("impact"),
        RemovalImpact {
            captures: 1,
            ..RemovalImpact::default()
        }
    );
    assert_eq!(
        projects
            .remove_with_data("missing", "missing")
            .map_err(|error| error.code()),
        Err("not_found")
    );
}
