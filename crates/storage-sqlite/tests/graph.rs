//! Knowledge graph over SQLite (ADR-0005): suggestions from captured work,
//! confirmation, queries by date and project removal.

mod support;

use application::claims::{Claims, NewClaim};
use application::graph::{
    EntityEdit, GraphError, KnowledgeGraph, LinkRequest, NewEntity, NodeRef, TimelineKind,
};
use application::projects::Projects;
use application::relations::DecisionRelations;
use domain::claims::ClaimKind;
use domain::entities::{EdgeKind, EntityKind, NodeKind};
use domain::relations::RelationKind;

const STORE_DIFF: &str =
    "diff --git a/crates/storage-sqlite/src/store.rs b/crates/storage-sqlite/src/store.rs\n\
@@ -1 +1 @@\n\
-old\n\
+new\n\
diff --git a/crates/storage-sqlite/Cargo.toml b/crates/storage-sqlite/Cargo.toml\n\
@@ -1 +1,2 @@\n\
 [dependencies]\n\
+rusqlite = { version = \"0.40\" }\n";

fn component(
    graph: &KnowledgeGraph<storage_sqlite::SqliteStore>,
    name: &str,
    pattern: &str,
) -> String {
    graph
        .create_entity(NewEntity {
            project_id: "p1".into(),
            kind: Some(EntityKind::Component),
            name: name.into(),
            patterns: vec![pattern.into()],
            ..NewEntity::default()
        })
        .expect("create component")
        .entity_id
}

#[test]
fn suggestions_come_from_captured_files_and_dependencies() {
    let test = support::open("graph-suggest", &["p1"]);
    let decision = support::decision_with_diff(
        &test.store,
        "p1",
        "store",
        "Onde guardar as decisões?",
        &[
            "crates/storage-sqlite/src/store.rs",
            "crates/storage-sqlite/Cargo.toml",
        ],
        STORE_DIFF,
    );
    let graph = KnowledgeGraph::new(test.store.clone());

    let first = graph.refresh_suggestions("p1").expect("refresh");
    assert_eq!(first.new_edges, 0, "no entity yet");
    assert_eq!(first.components.len(), 1);
    assert_eq!(first.components[0].name, "storage-sqlite");
    assert_eq!(first.components[0].pattern, "crates/storage-sqlite/**");
    assert_eq!(first.technologies.len(), 1);
    assert_eq!(first.technologies[0].name, "rusqlite");

    let storage = component(&graph, "storage-sqlite", "crates/storage-sqlite/**");
    let sqlite = graph
        .create_entity(NewEntity {
            project_id: "p1".into(),
            kind: Some(EntityKind::Technology),
            name: "SQLite".into(),
            aliases: vec!["rusqlite".into()],
            ..NewEntity::default()
        })
        .expect("create technology")
        .entity_id;

    let second = graph.refresh_suggestions("p1").expect("refresh again");
    assert_eq!(second.new_edges, 2, "affects + uses");
    assert!(second.components.is_empty() && second.technologies.is_empty());
    let pending = graph.suggestions("p1").expect("suggestions");
    assert_eq!(pending.len(), 2);
    assert!(pending
        .iter()
        .all(|suggestion| suggestion.source.node.id == decision));

    // The graph view sees the pending pair as suggestions, not edges.
    let whole = graph.project_graph("p1", None).expect("graph");
    assert_eq!(whole.nodes.len(), 3, "two entities and the decision");
    assert!(whole.edges.is_empty());
    assert_eq!(whole.suggested.len(), 2);

    // Suggestions do not count until confirmed.
    assert!(graph
        .file_lens("p1", "crates/storage-sqlite/src/x.rs", None)
        .expect("lens")
        .decisions
        .is_empty());

    let affects = pending
        .iter()
        .find(|suggestion| suggestion.kind == EdgeKind::Affects)
        .expect("affects");
    let uses = pending
        .iter()
        .find(|suggestion| suggestion.kind == EdgeKind::Uses)
        .expect("uses");
    assert_eq!(affects.entity.node.id, storage);
    assert_eq!(uses.entity.node.id, sqlite);
    assert_eq!(uses.reason, "rusqlite");
    graph.confirm(&affects.edge_id).expect("confirm");
    graph.invalidate(&uses.edge_id).expect("reject");

    assert_eq!(
        graph.refresh_suggestions("p1").expect("third").new_edges,
        0,
        "a rejected suggestion never returns"
    );
    assert!(graph.suggestions("p1").expect("none left").is_empty());

    let lens = graph
        .file_lens("p1", "crates\\storage-sqlite\\src\\store.rs", None)
        .expect("lens");
    assert_eq!(lens.components.len(), 1);
    assert_eq!(lens.decisions.len(), 1);
    assert_eq!(lens.decisions[0].node.id, decision);

    let whole = graph.project_graph("p1", None).expect("graph");
    assert!(whole.suggested.is_empty());
    assert_eq!(whole.edges.len(), 1);
    assert_eq!(whole.edges[0].from, NodeRef::decision(decision.clone()));
    assert_eq!(whole.edges[0].to, NodeRef::entity(storage.clone()));

    let map = graph.project_map("p1", None).expect("map");
    let row = map
        .entities
        .iter()
        .find(|row| row.entity.entity_id == storage)
        .expect("row");
    assert_eq!(row.decisions, 1);
    assert_eq!(map.pending, 0);
}

#[test]
fn claims_parts_impact_and_history() {
    let test = support::open("graph-query", &["p1"]);
    let base = support::decision_with_diff(
        &test.store,
        "p1",
        "base",
        "Qual banco?",
        &["crates/storage-sqlite/src/a.rs"],
        "",
    );
    let later = support::decision(
        &test.store,
        "p1",
        "later",
        "Como migrar?",
        "Migrations embutidas",
    );
    let graph = KnowledgeGraph::new(test.store.clone());
    let storage = component(&graph, "storage", "crates/storage-sqlite/**");
    let backend = component(&graph, "backend", "crates/application/**");

    graph
        .link(LinkRequest {
            kind: EdgeKind::Affects,
            source_kind: NodeKind::Decision,
            source_id: base.clone(),
            entity_id: storage.clone(),
        })
        .expect("link decision");
    graph
        .link(LinkRequest {
            kind: EdgeKind::PartOf,
            source_kind: NodeKind::Entity,
            source_id: storage.clone(),
            entity_id: backend.clone(),
        })
        .expect("part of");
    assert_eq!(
        graph
            .link(LinkRequest {
                kind: EdgeKind::PartOf,
                source_kind: NodeKind::Entity,
                source_id: backend.clone(),
                entity_id: storage.clone(),
            })
            .map_err(|error| error.code()),
        Err("cycle")
    );
    assert_eq!(
        graph
            .link(LinkRequest {
                kind: EdgeKind::Uses,
                source_kind: NodeKind::Decision,
                source_id: base.clone(),
                entity_id: storage.clone(),
            })
            .map_err(|error| error.code()),
        Err("edge_not_allowed")
    );

    let claim = Claims::new(test.store.clone())
        .create(NewClaim {
            project_id: "p1".into(),
            kind: ClaimKind::Constraint,
            statement: "Toda escrita passa por uma transação.".into(),
            valid_from: Some("2026-01-01".into()),
            valid_until: None,
            source_decision_id: None,
        })
        .expect("claim");
    graph
        .link(LinkRequest {
            kind: EdgeKind::AppliesTo,
            source_kind: NodeKind::Claim,
            source_id: claim.claim_id.clone(),
            entity_id: backend.clone(),
        })
        .expect("claim applies");

    // The file matches the child; the parent's rule applies too.
    let lens = graph
        .file_lens("p1", "crates/storage-sqlite/src/a.rs", None)
        .expect("lens");
    assert_eq!(lens.components.len(), 2);
    assert_eq!(lens.claims.len(), 1);

    DecisionRelations::new(test.store.clone())
        .relate(&later, &base, RelationKind::DependsOn)
        .expect("depends on");
    let impact = graph
        .impact("p1", &NodeRef::entity(storage.clone()), None)
        .expect("impact");
    let ids: Vec<&str> = impact.iter().map(|row| row.node.id.as_str()).collect();
    assert!(ids.contains(&base.as_str()) && ids.contains(&later.as_str()));

    let detail = graph.entity_detail(&backend, None).expect("detail");
    assert_eq!(detail.parts.len(), 1);
    assert_eq!(detail.claims.len(), 1);

    let around = graph
        .neighborhood("p1", &NodeRef::entity(storage.clone()), 2, 40, None)
        .expect("neighborhood");
    assert_eq!(
        around.nodes[0].summary.node,
        NodeRef::entity(storage.clone())
    );
    assert!(
        around
            .nodes
            .iter()
            .any(|node| node.summary.node.id == later),
        "two hops reach the dependent decision"
    );
    let small = graph
        .neighborhood("p1", &NodeRef::entity(storage.clone()), 2, 2, None)
        .expect("truncated");
    assert!(small.truncated && small.nodes.len() == 2);

    // Before anything existed, nothing held.
    assert!(graph
        .project_map("p1", Some("2020-01-01"))
        .expect("old map")
        .entities
        .is_empty());

    let events = graph
        .timeline("p1", Some(&storage), None, None)
        .expect("timeline");
    assert!(events
        .iter()
        .any(|event| event.kind == TimelineKind::EntityCreated));
    assert!(events
        .iter()
        .any(|event| event.kind == TimelineKind::EdgeConfirmed));

    graph.retire_entity(&storage).expect("retire");
    assert!(graph
        .file_lens("p1", "crates/storage-sqlite/src/a.rs", None)
        .expect("after")
        .components
        .is_empty());
    assert_eq!(
        graph.retire_entity(&storage).map_err(|error| error.code()),
        Err("conflict")
    );
}

#[test]
fn names_are_unique_per_kind_and_patterns_validated() {
    let test = support::open("graph-names", &["p1"]);
    let graph = KnowledgeGraph::new(test.store.clone());
    let first = component(&graph, "Storage SQLite", "crates/storage-sqlite/**");
    let duplicate = graph.create_entity(NewEntity {
        project_id: "p1".into(),
        kind: Some(EntityKind::Component),
        name: "storage-sqlite".into(),
        ..NewEntity::default()
    });
    assert_eq!(
        duplicate.map_err(|error| error.code()),
        Err("duplicate_name")
    );
    let bad = graph.create_entity(NewEntity {
        project_id: "p1".into(),
        kind: Some(EntityKind::Component),
        name: "fora".into(),
        patterns: vec!["../fora/**".into()],
        ..NewEntity::default()
    });
    assert_eq!(bad.map_err(|error| error.code()), Err("invalid_pattern"));
    let technology_pattern = graph.create_entity(NewEntity {
        project_id: "p1".into(),
        kind: Some(EntityKind::Technology),
        name: "Rust".into(),
        patterns: vec!["**/*.rs".into()],
        ..NewEntity::default()
    });
    assert_eq!(
        technology_pattern.map_err(|error| error.code()),
        Err("pattern_on_technology")
    );

    let edited = graph
        .update_entity(
            &first,
            EntityEdit {
                name: "Armazenamento".into(),
                description: "Banco local".into(),
                patterns: vec![
                    "crates/storage-sqlite/**".into(),
                    "crates/storage-sqlite/**".into(),
                ],
                aliases: vec!["storage".into()],
            },
        )
        .expect("edit");
    assert_eq!(edited.patterns.len(), 1, "duplicates collapse");
    let stored = graph.entities("p1").expect("entities");
    assert_eq!(stored[0].name, "Armazenamento");
    assert_eq!(stored[0].aliases, vec!["storage"]);
    assert!(matches!(
        graph.entity_detail("missing", None),
        Err(GraphError::NotFound)
    ));
}

#[test]
fn removing_the_project_removes_its_graph() {
    let test = support::open("graph-removal", &["p1"]);
    let graph = KnowledgeGraph::new(test.store.clone());
    component(&graph, "storage", "crates/storage-sqlite/**");
    let projects = Projects::new(test.store.clone());
    assert_eq!(projects.removal_impact("p1").expect("impact").entities, 1);
    projects.remove_with_data("p1", "p1").expect("remove");
    assert!(graph.entities("p1").expect("gone").is_empty());
}

#[test]
fn two_decisions_of_one_capture_keep_their_own_dependencies() {
    use application::captures::{
        CaptureArtifactRecord, CaptureCheckpointRecord, CaptureReceiptRecord, CaptureRepository,
        CaptureWrite,
    };
    use application::extract::{DecisionCandidateRecord, ExtractionStore};
    use application::inbox::{CandidateEdits, Inbox};
    use application::jobs::{JobRecord, JobState, ANALYZE_CAPTURE_KIND};

    let test = support::open("graph-scope", &["p1"]);
    let at = "2026-01-02T00:00:00Z".to_string();
    let hunk = |id: &str, content: &str, seed: usize| CaptureArtifactRecord {
        capture_id: "capture-two".into(),
        artifact_id: id.into(),
        kind: "diff_hunk".into(),
        content: content.into(),
        metadata: "{}".into(),
        fingerprint: format!("{seed:0>64}"),
    };
    test.store
        .insert_capture(&CaptureWrite {
            receipt: CaptureReceiptRecord {
                capture_id: "capture-two".into(),
                idempotency_key: "key-capture-two".into(),
                canonical_path: "C:/synthetic/p1".into(),
                received_at: at.clone(),
                artifact_count: 2,
            },
            artifacts: vec![
                hunk(
                    "art-ui",
                    "diff --git a/web/form.ts b/web/form.ts\n+export const x = 1;\n",
                    1,
                ),
                hunk(
                    "art-db",
                    "diff --git a/api/Cargo.toml b/api/Cargo.toml\n@@ -1 +1,2 @@\n \
                     [dependencies]\n+redis = \"0.25\"\n",
                    2,
                ),
            ],
            job: JobRecord {
                id: "job-two".into(),
                kind: ANALYZE_CAPTURE_KIND.into(),
                payload: "capture-two".into(),
                state: JobState::Queued,
                idempotent: true,
                attempts: 0,
                last_error: None,
                created_at: at.clone(),
                updated_at: at.clone(),
            },
            checkpoint: CaptureCheckpointRecord {
                adapter: "opencode".into(),
                adapter_version: "0.1.0".into(),
                session_id: "s".into(),
                message_id: "m".into(),
                capture_id: "capture-two".into(),
                observed_at: at.clone(),
                updated_at: at.clone(),
            },
        })
        .expect("capture");
    let candidate = |key: &str, artifact: &str, file: &str| DecisionCandidateRecord {
        id: format!("cand-{key}"),
        project_id: "p1".into(),
        capture_id: "capture-two".into(),
        status: "pending".into(),
        question: format!("pergunta {key}"),
        choice: format!("escolha {key}"),
        rationale: "motivo".into(),
        signals: "[\"public_contract\"]".into(),
        confidence: 0.7,
        confidence_reason: "sintético".into(),
        evidence_refs: format!("[\"{artifact}\"]"),
        diff_summary: format!("{{\"files\":[\"{file}\"],\"artifacts\":1}}"),
        dedup_hash: format!("dedup-{key}"),
        created_at: at.clone(),
        updated_at: at.clone(),
        kind: "decision".into(),
        significance: 1.0,
        criteria: "[]".into(),
    };
    test.store
        .insert_candidates(&[
            candidate("ui", "art-ui", "web/form.ts"),
            candidate("db", "art-db", "api/Cargo.toml"),
        ])
        .expect("candidates");
    let inbox = Inbox::new(test.store.clone());
    let edits: Option<CandidateEdits> = None;
    let ui = inbox
        .confirm("cand-ui", edits.clone())
        .expect("ui")
        .decision_id;
    let db = inbox.confirm("cand-db", edits).expect("db").decision_id;

    let graph = KnowledgeGraph::new(test.store.clone());
    let redis = graph
        .create_entity(NewEntity {
            project_id: "p1".into(),
            kind: Some(EntityKind::Technology),
            name: "Redis".into(),
            ..NewEntity::default()
        })
        .expect("technology")
        .entity_id;
    graph.refresh_suggestions("p1").expect("refresh");
    let uses: Vec<String> = graph
        .suggestions("p1")
        .expect("suggestions")
        .into_iter()
        .filter(|suggestion| {
            suggestion.kind == EdgeKind::Uses && suggestion.entity.node.id == redis
        })
        .map(|suggestion| suggestion.source.node.id)
        .collect();
    assert_eq!(
        uses,
        vec![db],
        "only the decision that cites the manifest uses Redis"
    );
    assert!(!uses.contains(&ui));
}
