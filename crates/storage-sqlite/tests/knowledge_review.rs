//! Synthetic temporary database coverage for the read-only review port.
mod support;
use application::{
    claim_suggestions::{ClaimSuggestionRecord, ClaimSuggestionStore},
    claims::{Claims, NewClaim},
    knowledge_review::ReviewStore,
    relation_suggestions::{RelationSuggestionRecord, RelationSuggestionStore},
    relations::DecisionRelations,
};
use domain::{claims::ClaimKind, relations::RelationKind};

#[test]
fn snapshot_keeps_raw_pending_and_full_state_is_unchanged() {
    let t = support::open("review-readonly", &["p", "other"]);
    let old = support::decision(&t.store, "p", "old", "Arquitetura?", "Monólito");
    let new = support::decision(&t.store, "p", "new", "Distribuição?", "ZIP Windows");
    let dependent = support::decision(&t.store, "p", "dependent", "Persistência?", "SQLite");
    let foreign = support::decision(&t.store, "other", "foreign", "Arquitetura?", "Outra");
    let now = "2026-01-01T00:00:00Z";
    t.store
        .insert_claim_suggestion(&ClaimSuggestionRecord {
            source_version: Some(1),
            inherited_scope: "[]".into(),
            qualifiers: "[]".into(),
            suggestion_id: "pending-claim".into(),
            project_id: "p".into(),
            decision_id: old.clone(),
            kind: ClaimKind::Constraint,
            statement: "Preservar histórico".into(),
            quote: "Monólito".into(),
            created_at: now.into(),
        })
        .unwrap();
    t.store
        .insert_relation_suggestion(&RelationSuggestionRecord {
            suggestion_id: "pending-relation".into(),
            project_id: "p".into(),
            from_id: dependent.clone(),
            to_id: old.clone(),
            kind: RelationKind::DependsOn,
            quote: "SQLite".into(),
            reason: "Proposta".into(),
            created_at: now.into(),
        })
        .unwrap();
    Claims::new(t.store.clone())
        .create(NewClaim {
            source_version: None,
            qualifiers: Vec::new(),
            project_id: "p".into(),
            kind: ClaimKind::Constraint,
            statement: "Preservar histórico".into(),
            valid_from: Some(now.into()),
            valid_until: None,
            source_decision_id: Some(old.clone()),
        })
        .unwrap();
    DecisionRelations::new(t.store.clone())
        .relate(&dependent, &old, RelationKind::DependsOn)
        .unwrap();
    DecisionRelations::new(t.store.clone())
        .supersede(&new, &old)
        .unwrap();
    let before = t.store.review_snapshot("p").unwrap().unwrap();
    assert_eq!(before.decisions.len(), 3);
    assert_eq!(before.claim_suggestions.len(), 1);
    assert_eq!(before.relation_suggestions.len(), 1);
    assert_eq!(before.claims.len(), 1);
    assert!(!before.decisions.iter().any(|d| d.decision_id == foreign));
    let connection = rusqlite::Connection::open(t.root.join("app.db")).unwrap();
    let dump = || {
        let mut sql = connection
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap();
        let tables = sql
            .query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let mut result = Vec::new();
        for table in tables {
            let mut stmt = connection
                .prepare(&format!("SELECT * FROM \"{table}\" ORDER BY 1"))
                .unwrap();
            let n = stmt.column_count();
            let mut rows = stmt.query([]).unwrap();
            while let Some(row) = rows.next().unwrap() {
                result.push((
                    table.clone(),
                    (0..n)
                        .map(|i| row.get::<_, rusqlite::types::Value>(i).unwrap())
                        .collect::<Vec<_>>(),
                ));
            }
        }
        result
    };
    let state = dump();
    assert_eq!(t.store.review_snapshot("p").unwrap().unwrap(), before);
    assert!(t.store.review_snapshot("missing").unwrap().is_none());
    let ids = t
        .store
        .search_review_decisions("p", "Arquitetura", 10)
        .unwrap();
    assert!(!ids.contains(&foreign));
    assert!(!ids.contains(&old));
    assert_eq!(dump(), state);
}
