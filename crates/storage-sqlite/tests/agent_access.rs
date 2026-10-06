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
fn claim_scope_is_exposed_and_malformed_scope_fails_closed() {
    let test = support::open("agent-inherited-scope", &["p1"]);
    let path = register_directory(&test);
    let id = support::decision(&test.store, "p1", "scoped", "Question", "Choice");
    application::decisions::Decisions::new(test.store.clone())
        .revise(
            &id,
            application::decisions::DecisionEdits {
                scope: Some(vec!["only offline".into()]),
                ..application::decisions::DecisionEdits::default()
            },
        )
        .unwrap();
    let claim = Claims::new(test.store.clone())
        .create(NewClaim {
            project_id: "p1".into(),
            kind: ClaimKind::Constraint,
            statement: "Rule".into(),
            valid_from: None,
            valid_until: None,
            source_decision_id: Some(id.clone()),
            source_version: Some(2),
            qualifiers: Vec::new(),
        })
        .unwrap();
    let access = AgentAccess::new(test.store.clone());
    assert!(access
        .decision(&path, &id)
        .unwrap()
        .contains("only offline"));
    Connection::open(test.root.join("app.db"))
        .unwrap()
        .execute(
            "UPDATE context_claims SET inherited_scope='invalid' WHERE claim_id=?1",
            [claim.claim_id],
        )
        .unwrap();
    assert!(access.decision(&path, &id).is_err());
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
            source_version: None,
            qualifiers: Vec::new(),
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
            .search(&path, "  ", None, None)
            .map_err(|error| error.code()),
        Err("invalid_request")
    );
    assert_eq!(
        access
            .search(&path, "banco", Some(10), None)
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
        .search(&path, "banco", None, None)
        .expect("search")
        .expect("block");
    assert!(first.contains("Qual banco usar? → SQLite"));
    let again = access
        .search(&path, "banco", None, None)
        .expect("search")
        .expect("block");
    assert_eq!(first, again, "explicit searches are not deduplicated");
    assert_eq!(
        access
            .search(&path, "renderização", None, None)
            .expect("search"),
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

#[test]
fn file_context_follows_the_project_map() {
    use application::graph::{KnowledgeGraph, LinkRequest, NewEntity};
    use domain::entities::{EdgeKind, EntityKind, NodeKind};

    let test = support::open("agent-file", &["p1"]);
    let path = register_directory(&test);
    let decision = support::decision(&test.store, "p1", "db", "Qual banco usar?", "SQLite");
    support::decision(&test.store, "p1", "ui", "Qual toolkit?", "GPUI");
    let graph = KnowledgeGraph::new(test.store.clone());
    let storage = graph
        .create_entity(NewEntity {
            project_id: "p1".into(),
            kind: Some(EntityKind::Component),
            name: "storage".into(),
            patterns: vec!["crates/storage-sqlite/**".into()],
            ..NewEntity::default()
        })
        .expect("component")
        .entity_id;
    graph
        .link(LinkRequest {
            kind: EdgeKind::Affects,
            source_kind: NodeKind::Decision,
            source_id: decision,
            entity_id: storage,
        })
        .expect("link");
    let access = AgentAccess::new(test.store.clone());

    // The file names no word of the decision; only the map ties them.
    let block = access
        .file_context(&path, "crates/storage-sqlite/src/store.rs", None)
        .expect("file context")
        .expect("block");
    assert!(block.contains("Qual banco usar? → SQLite"));
    assert!(
        !block.contains("GPUI"),
        "only what the map ties to the file"
    );
    let absolute = format!("{path}/crates/storage-sqlite/src/store.rs");
    assert!(access
        .file_context(&path, &absolute, None)
        .expect("absolute path")
        .is_some_and(|text| text.contains("SQLite")));
    assert_eq!(
        access
            .file_context(&path, "apps/desktop/src/main.rs", None)
            .expect("unmapped"),
        None
    );
    assert_eq!(
        access
            .file_context(&path, "  ", None)
            .map_err(|error| error.code()),
        Err("invalid_request")
    );
}

#[test]
fn every_query_is_logged_without_its_text() {
    let test = support::open("agent-queries", &["p1"]);
    let path = register_directory(&test);
    let id = support::decision(&test.store, "p1", "a", "Qual banco usar?", "SQLite");
    let access = AgentAccess::new(test.store.clone());
    let elsewhere = test.root.join("elsewhere");
    std::fs::create_dir_all(&elsewhere).expect("dir");

    access.decision(&path, &id).expect("decision");
    assert!(access.decision(&path, "D:zzzzzzzz").is_err());
    access.search(&path, "banco", None, None).expect("search");
    access
        .search(&path, "renderização", None, None)
        .expect("empty search");
    access
        .file_context(&path, "src/segredo.rs", None)
        .expect("file");
    // Unknown project and malformed requests leave no trace.
    assert!(access.decision(&elsewhere.to_string_lossy(), &id).is_err());
    assert!(access.decision(&path, "D:ab").is_err());
    assert!(access.search(&path, " ", None, None).is_err());

    let connection = Connection::open(test.root.join("app.db")).expect("raw");
    let mut statement = connection
        .prepare(
            "SELECT project_id, tool, outcome, chars, created_at \
             FROM agent_queries ORDER BY query_id",
        )
        .expect("prepare");
    let rows: Vec<(String, String, String, i64, String)> = statement
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })
        .expect("query")
        .collect::<Result<_, _>>()
        .expect("rows");
    let summary: Vec<(&str, &str)> = rows
        .iter()
        .map(|row| (row.1.as_str(), row.2.as_str()))
        .collect();
    assert_eq!(
        summary,
        [
            ("decision", "answered"),
            ("decision", "not_found"),
            ("search", "answered"),
            ("search", "empty"),
            ("file", "empty"),
        ]
    );
    for row in &rows {
        assert_eq!(row.0, "p1");
        assert_eq!(row.3 > 0, row.2 == "answered", "chars only when answered");
        assert!(row.4.ends_with('Z'), "RFC3339 UTC: {}", row.4);
        for secret in ["banco", "zzzzzzzz", "segredo", "renderização", id.as_str()] {
            assert!(!row.0.contains(secret) && !row.4.contains(secret));
        }
    }
}
