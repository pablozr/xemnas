//! Agent access over SQLite: open a short reference and search, read-only.

mod support;

use application::agent_access::{AgentAccess, AgentAccessError};
use application::claims::{Claims, NewClaim};
use application::injection::short_ref;
use application::projects::canonicalize_location;
use application::relations::DecisionRelations;
use domain::claims::ClaimKind;
use domain::relations::RelationKind;
use rusqlite::Connection;

fn register_directory(test: &support::TestStore) -> String {
    let directory = test.root.join("workspace");
    std::fs::create_dir_all(&directory).expect("workspace");
    let location = canonicalize_location(&directory.to_string_lossy()).expect("canonical");
    Connection::open(test.root.join("app.db"))
        .expect("raw")
        .execute(
            "UPDATE projects SET location = ?1 WHERE id = 'p1'",
            [&location],
        )
        .expect("relocate project");
    directory.to_string_lossy().into_owned()
}

#[test]
fn a_short_reference_opens_the_full_decision_with_its_links() {
    let test = support::open("agent-decision", &["p1"]);
    let path = register_directory(&test);
    let old = support::decision(&test.store, "p1", "old", "Qual banco?", "Postgres");
    let new = support::decision(&test.store, "p1", "new", "Qual banco?", "SQLite");
    let base = support::decision(&test.store, "p1", "base", "Onde roda?", "Desktop local");
    let relations = DecisionRelations::new(test.store.clone());
    relations.supersede(&new, &old).expect("supersede");
    relations
        .relate(&new, &base, RelationKind::DependsOn)
        .expect("depends");
    Claims::new(test.store.clone())
        .create(NewClaim {
            project_id: "p1".to_string(),
            kind: ClaimKind::Constraint,
            statement: "Sem servidor".to_string(),
            valid_from: Some("2020-01-01".to_string()),
            valid_until: None,
            source_decision_id: Some(new.clone()),
        })
        .expect("claim");
    let access = AgentAccess::new(test.store.clone());

    let text = access
        .decision(&path, &format!("D:{}", short_ref(&new)))
        .expect("decision");
    assert!(text.starts_with("<xemnas-context"));
    assert!(text.contains(&format!("D:{} v1 (vigente)", short_ref(&new))));
    assert!(text.contains("pergunta: Qual banco?"));
    assert!(text.contains("escolha: SQLite"));
    assert!(text.contains("motivo: motivo de new"));
    assert!(text.contains(&format!("substitui: D:{} Qual banco?", short_ref(&old))));
    assert!(text.contains(&format!("depende de: D:{} Onde roda?", short_ref(&base))));
    assert!(text.contains("claim constraint:"));
    assert!(text.contains("evidências: 1 artefato(s) capturado(s)"));

    let old_text = access
        .decision(&path, &short_ref(&old).to_uppercase())
        .expect("case-insensitive, without prefix");
    assert!(old_text.contains("(substituída)"));
    assert!(old_text.contains(&format!("substituída por: D:{}", short_ref(&new))));
    assert!(
        access.decision(&path, &new).is_ok(),
        "the full id also works"
    );
}

#[test]
fn errors_are_explicit() {
    let test = support::open("agent-errors", &["p1"]);
    let path = register_directory(&test);
    support::decision(&test.store, "p1", "a", "Qual banco?", "SQLite");
    let access = AgentAccess::new(test.store.clone());
    let elsewhere = test.root.join("elsewhere");
    std::fs::create_dir_all(&elsewhere).expect("dir");

    assert_eq!(
        access.decision(&path, "D:zzzzzzzz"),
        Err(AgentAccessError::NotFound)
    );
    assert_eq!(
        access.decision(&path, "D:ab").map_err(|error| error.code()),
        Err("invalid_request")
    );
    assert_eq!(
        access.decision(&elsewhere.to_string_lossy(), "D:zzzzzzzz"),
        Err(AgentAccessError::ProjectNotFound)
    );
    assert_eq!(
        access
            .search(&path, "  ", None)
            .map_err(|error| error.code()),
        Err("invalid_request")
    );
    assert_eq!(
        access
            .search(&path, "banco", Some(10))
            .map_err(|error| error.code()),
        Err("invalid_request")
    );
}

#[test]
fn search_returns_the_compact_block_every_time() {
    let test = support::open("agent-search", &["p1"]);
    let path = register_directory(&test);
    support::decision(&test.store, "p1", "a", "Qual banco usar?", "SQLite");
    let access = AgentAccess::new(test.store.clone());

    let first = access
        .search(&path, "banco", None)
        .expect("search")
        .expect("block");
    assert!(first.contains("Qual banco usar? → SQLite"));
    let again = access
        .search(&path, "banco", None)
        .expect("search")
        .expect("block");
    assert_eq!(first, again, "explicit searches are not deduplicated");
    assert_eq!(
        access.search(&path, "renderização", None).expect("search"),
        None
    );
    let recorded: i64 = Connection::open(test.root.join("app.db"))
        .expect("raw")
        .query_row("SELECT COUNT(*) FROM context_injections", [], |row| {
            row.get(0)
        })
        .expect("count");
    assert_eq!(recorded, 0, "agent reads are not injections");
}
