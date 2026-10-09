//! Knowledge graph over SQLite (ADR-0005): suggestions from captured work,
//! confirmation, queries by date and project removal.

mod support;

use application::claims::{Claims, NewClaim};
use application::graph::{
    mention_quote, EntityEdit, GraphError, KnowledgeGraph, LinkRequest, NewEntity, NodeRef,
    TimelineKind,
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
fn a_decision_from_a_document_is_tied_to_what_its_text_names_not_to_docs() {
    let test = support::open("graph-mentions", &["p1"]);
    let graph = KnowledgeGraph::new(test.store.clone());
    let docs = component(&graph, "docs", "docs/**");
    let core = component(&graph, "core", "crates/core/**");
    let storage = component(&graph, "storage-sqlite", "crates/storage-sqlite/**");
    let api = component(&graph, "api", "apps/api/**");
    let sqlite = graph
        .create_entity(NewEntity {
            project_id: "p1".into(),
            kind: Some(EntityKind::Technology),
            name: "SQLite".into(),
            ..NewEntity::default()
        })
        .expect("create technology")
        .entity_id;
    // Taken from an ADR: the only file is the document itself.
    let decision = support::decision_with_diff(
        &test.store,
        "p1",
        "adr",
        "Como o core grava os eventos vindos da api no SQLite, sem perder nenhum?",
        &["docs/adr/0003-outbox.md"],
        "",
    );

    let report = graph.refresh_suggestions("p1").expect("refresh");
    assert_eq!(report.new_edges, 2, "core + SQLite, by mention");
    assert!(
        report.components.is_empty(),
        "docs are not proposed as a part"
    );
    let pending = graph.suggestions("p1").expect("suggestions");
    let tied = |entity: &str| pending.iter().find(|row| row.entity.node.id == entity);
    let affects = tied(&core).expect("core is named");
    assert_eq!(affects.kind, EdgeKind::Affects);
    assert_eq!(affects.source.node.id, decision);
    let quote = mention_quote(&affects.reason).expect("a mention");
    assert!(quote.contains("o core grava"), "{quote}");
    assert_eq!(tied(&sqlite).expect("SQLite is named").kind, EdgeKind::Uses);
    assert!(
        tied(&docs).is_none(),
        "the document is evidence, not the part"
    );
    assert!(tied(&storage).is_none());
    assert!(
        tied(&api).is_none(),
        "a short name in plain prose is not a mention"
    );

    // A decision that touched the part keeps the file as the reason.
    support::decision_with_diff(
        &test.store,
        "p1",
        "code",
        "Como o core valida a entrada?",
        &["crates/core/src/lib.rs"],
        "",
    );
    graph.refresh_suggestions("p1").expect("refresh again");
    let reasons: Vec<String> = graph
        .suggestions("p1")
        .expect("suggestions")
        .into_iter()
        .filter(|row| row.entity.node.id == core && row.source.node.id != decision)
        .map(|row| row.reason)
        .collect();
    assert_eq!(reasons, vec!["crates/core/src/lib.rs".to_string()]);
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
            source_version: None,
            qualifiers: Vec::new(),
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
        qualifiers: "[]".into(),
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

#[test]
fn folders_with_the_same_name_are_named_by_path_and_a_part_has_one_parent() {
    let test = support::open("graph-identity", &["p1"]);
    support::decision_with_diff(
        &test.store,
        "p1",
        "a",
        "Como expor a API?",
        &["apps/api/src/main.rs"],
        "diff --git a/apps/api/src/main.rs b/apps/api/src/main.rs\n+fn main() {}\n",
    );
    support::decision_with_diff(
        &test.store,
        "p1",
        "b",
        "Como servir a API interna?",
        &["services/api/src/lib.rs"],
        "diff --git a/services/api/src/lib.rs b/services/api/src/lib.rs\n+pub fn x() {}\n",
    );
    let graph = KnowledgeGraph::new(test.store.clone());
    let report = graph.refresh_suggestions("p1").expect("refresh");
    let mut names: Vec<(String, String)> = report
        .components
        .iter()
        .map(|proposal| (proposal.name.clone(), proposal.pattern.clone()))
        .collect();
    names.sort();
    assert_eq!(
        names,
        vec![
            ("apps/api".to_string(), "apps/api/**".to_string()),
            ("services/api".to_string(), "services/api/**".to_string()),
        ]
    );

    // C, C++ and C# are three technologies.
    for name in ["C", "C++", "C#"] {
        graph
            .create_entity(NewEntity {
                project_id: "p1".into(),
                kind: Some(EntityKind::Technology),
                name: name.into(),
                ..NewEntity::default()
            })
            .unwrap_or_else(|error| panic!("{name}: {error:?}"));
    }

    let api = component(&graph, "apps/api", "apps/api/**");
    let backend = component(&graph, "backend", "backend/**");
    let platform = component(&graph, "platform", "platform/**");
    let part = |parent: &str| LinkRequest {
        kind: EdgeKind::PartOf,
        source_kind: NodeKind::Entity,
        source_id: api.clone(),
        entity_id: parent.to_string(),
    };
    graph.link(part(&backend)).expect("first parent");
    assert_eq!(
        graph
            .link(part(&platform))
            .map(|_| ())
            .map_err(|error| error.code()),
        Err("second_parent")
    );
}

#[test]
fn an_adopted_rule_is_suggested_for_the_components_its_evidence_touched() {
    use application::extract::{DecisionCandidateRecord, ExtractionStore};
    use application::inbox::{CandidateEdits, Inbox};

    let test = support::open("graph-rule", &["p1"]);
    let graph = KnowledgeGraph::new(test.store.clone());
    let storage = component(&graph, "storage-sqlite", "crates/storage-sqlite/**");
    component(&graph, "desktop", "apps/desktop/**");
    test.store
        .insert_candidates(&[DecisionCandidateRecord {
            qualifiers: "[]".into(),
            id: "cand-rule".into(),
            project_id: "p1".into(),
            capture_id: "capture-p1".into(),
            status: "pending".into(),
            question: "Como gravar decisões?".into(),
            choice: "Toda escrita de decisão acontece numa transação.".into(),
            rationale: "Evita duplicatas.".into(),
            signals: "[\"public_contract\"]".into(),
            confidence: 0.8,
            confidence_reason: "sintético".into(),
            evidence_refs: "[\"art-p1\"]".into(),
            diff_summary: "{\"files\":[\"crates/storage-sqlite/src/inbox.rs\"],\"artifacts\":1}"
                .into(),
            dedup_hash: "dedup-rule".into(),
            created_at: "2026-01-02T00:00:00Z".into(),
            updated_at: "2026-01-02T00:00:00Z".into(),
            kind: "rule".into(),
            significance: 0.8,
            criteria: "[]".into(),
        }])
        .expect("candidate");
    let edits: Option<CandidateEdits> = None;
    let rule = Inbox::new(test.store.clone())
        .confirm("cand-rule", edits)
        .expect("adopt rule")
        .decision_id;

    graph.refresh_suggestions("p1").expect("refresh");
    let applies: Vec<(String, String)> = graph
        .suggestions("p1")
        .expect("suggestions")
        .into_iter()
        .filter(|suggestion| suggestion.kind == EdgeKind::AppliesTo)
        .map(|suggestion| (suggestion.source.node.id, suggestion.entity.node.id))
        .collect();
    assert_eq!(
        applies,
        vec![(rule, storage)],
        "only the component it touched"
    );
}

#[test]
fn an_empty_map_assembles_itself_from_the_declared_workspace() {
    use application::graph::WorkspaceKind;
    use application::projects::{ProjectRecord, ProjectRepository};

    let test = support::open("graph-assemble", &[]);
    let repo = test.root.join("repo");
    for dir in ["crates/core", "crates/api"] {
        std::fs::create_dir_all(repo.join(dir)).expect("dirs");
    }
    std::fs::write(
        repo.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/*\"]\n",
    )
    .expect("root");
    std::fs::write(
        repo.join("crates/core/Cargo.toml"),
        "[package]\nname = \"core\"\ndescription = \"Regras do domínio.\"\n",
    )
    .expect("core");
    std::fs::write(
        repo.join("crates/api/Cargo.toml"),
        "[package]\nname = \"api\"\n",
    )
    .expect("api");
    test.store
        .insert(&ProjectRecord::new(
            "ws".into(),
            repo.to_string_lossy().replace('\\', "/"),
            "2026-01-01T00:00:00Z".into(),
        ))
        .expect("project");

    let graph = KnowledgeGraph::new(test.store.clone());
    // Two members and the files at the root of the workspace.
    assert_eq!(graph.assemble("ws").expect("assemble"), 3);
    assert_eq!(graph.assemble("ws").expect("again"), 0, "only an empty map");
    let entities = graph.entities("ws").expect("entities");
    let root_files = entities
        .iter()
        .find(|entity| entity.name == "workspace")
        .expect("workspace root");
    assert_eq!(root_files.patterns, vec!["*"]);
    assert!(root_files.aliases.contains(&"workspace root".to_string()));
    let core = entities
        .iter()
        .find(|entity| entity.name == "core")
        .expect("core");
    assert_eq!(core.patterns, vec!["crates/core/**"]);
    assert_eq!(core.description, "Regras do domínio.");

    // A member added later is proposed, not created.
    std::fs::create_dir_all(repo.join("crates/web")).expect("web");
    std::fs::write(
        repo.join("crates/web/Cargo.toml"),
        "[package]\nname = \"web\"\n",
    )
    .expect("web manifest");
    let report = graph.refresh_suggestions("ws").expect("refresh");
    assert_eq!(report.components.len(), 1);
    assert_eq!(report.components[0].name, "web");
    assert_eq!(report.components[0].declared, Some(WorkspaceKind::Cargo));
    assert_eq!(graph.assemble("ws").expect("still"), 0);
}

#[test]
fn package_aliases_reach_existing_components_and_link_decisions_once() {
    use application::projects::{ProjectRecord, ProjectRepository};

    let test = support::open("graph-package-aliases", &[]);
    let repo = test.root.join("repo");
    let packages = [
        ("core", "@acme/core"),
        ("opencode-adapter", "@acme/opencode-adapter"),
        ("plugin", "@acme/app"),
    ];
    for (dir, name) in packages {
        std::fs::create_dir_all(repo.join("packages").join(dir)).expect("dirs");
        std::fs::write(
            repo.join("packages").join(dir).join("package.json"),
            format!("{{\"name\": \"{name}\"}}"),
        )
        .expect("package");
    }
    std::fs::write(
        repo.join("package.json"),
        "{\"workspaces\": [\"packages/*\"]}",
    )
    .expect("root");
    test.store
        .insert(&ProjectRecord::new(
            "ws".into(),
            repo.to_string_lossy().replace('\\', "/"),
            "2026-01-01T00:00:00Z".into(),
        ))
        .expect("project");

    // A map discovered before aliases existed: names and patterns only.
    let graph = KnowledgeGraph::new(test.store.clone());
    let mut ids = Vec::new();
    for (dir, name) in packages {
        ids.push(
            graph
                .create_entity(NewEntity {
                    project_id: "ws".into(),
                    kind: Some(EntityKind::Component),
                    name: name.into(),
                    patterns: vec![format!("packages/{dir}/**")],
                    aliases: if dir == "core" {
                        vec!["engine".into()]
                    } else {
                        vec![]
                    },
                    ..NewEntity::default()
                })
                .expect("component")
                .entity_id,
        );
    }
    let decision = support::decision_at(
        &test.store,
        "ws",
        &repo
            .to_string_lossy()
            .replace(std::path::MAIN_SEPARATOR, "/"),
        "adr",
        "Como o core grava o que o opencode adapter captura?",
        &["docs/adr/0001.md"],
        "",
    );

    let first = graph.refresh_suggestions("ws").expect("refresh");
    assert_eq!(first.new_edges, 2, "core and adapter, by their aliases");
    let entities = graph.entities("ws").expect("entities");
    let aliases = |id: &str| {
        entities
            .iter()
            .find(|entity| entity.entity_id == id)
            .expect("entity")
            .aliases
            .clone()
    };
    assert_eq!(aliases(&ids[0]), vec!["engine", "core"], "user alias kept");
    assert_eq!(
        aliases(&ids[1]),
        vec!["opencode-adapter", "opencode adapter"]
    );
    assert_eq!(aliases(&ids[2]), vec!["app", "plugin"]);

    // Idempotent, and a rejected link is not suggested again.
    let pending = graph.suggestions("ws").expect("suggestions");
    let core_edge = pending
        .iter()
        .find(|row| row.entity.node.id == ids[0] && row.source.node.id == decision)
        .expect("core suggested");
    graph.invalidate(&core_edge.edge_id).expect("reject");
    let again = graph.refresh_suggestions("ws").expect("refresh again");
    assert_eq!(again.new_edges, 0);
    assert_eq!(graph.suggestions("ws").expect("pending").len(), 1);
    assert_eq!(aliases(&ids[0]), vec!["engine", "core"]);
}

#[test]
fn recent_files_come_from_decisions_without_repeats() {
    let test = support::open("graph-recent-files", &["p1"]);
    support::decision_with_diff(
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
    support::decision_with_diff(
        &test.store,
        "p1",
        "again",
        "Como migrar o banco?",
        &["crates/storage-sqlite/src/store.rs", ""],
        STORE_DIFF,
    );
    let graph = KnowledgeGraph::new(test.store.clone());

    let files = graph.recent_files("p1", 6).expect("files");
    assert_eq!(files.len(), 2, "deduplicated, blanks dropped");
    assert!(files.contains(&"crates/storage-sqlite/src/store.rs".to_string()));
    assert_eq!(graph.recent_files("p1", 1).expect("one").len(), 1);
    assert!(graph.recent_files("p2", 6).expect("none").is_empty());
}

/// Raw rows of `entity_edges`: `(origin, reason, confirmed_by, invalidated_by)`.
fn edge_labels(
    test: &support::TestStore,
    edge_id: &str,
) -> (String, String, Option<String>, Option<String>) {
    rusqlite::Connection::open(test.root.join("app.db"))
        .expect("raw")
        .query_row(
            "SELECT origin, reason, confirmed_by, invalidated_by FROM entity_edges \
             WHERE edge_id = ?1",
            [edge_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("edge")
}

#[test]
fn the_migration_tells_who_confirmed_and_who_invalidated_each_edge() {
    let test = support::open("graph-actors-migration", &["p1"]);
    let path = test.root.join("app.db");
    {
        // Rebuild the schema as it was before the actors (version 46).
        let raw = rusqlite::Connection::open(&path).expect("raw");
        raw.execute_batch(
            "ALTER TABLE entity_edges DROP COLUMN confirmed_by; \
             ALTER TABLE entity_edges DROP COLUMN invalidated_by; \
             DELETE FROM schema_migrations WHERE version = 47;",
        )
        .expect("downgrade");
        for entity in ["e1", "e2", "e3", "e4", "e5", "e6", "e7", "e8"] {
            raw.execute(
                "INSERT INTO entities (entity_id, project_id, kind, name, key, created_at) \
                 VALUES (?1, 'p1', 'component', ?1, ?1, '2026-01-01T00:00:00Z')",
                [entity],
            )
            .expect("entity");
        }
        // edge, entity, kind, source kind, source, origin, reason, created, confirmed, invalidated
        let edges = [
            (
                "x1",
                "e1",
                "affects",
                "decision",
                "d1",
                "human",
                "",
                "2026-02-01T10:00:00Z",
                Some("2026-02-01T10:00:00Z"),
                None,
            ),
            (
                "x2",
                "e2",
                "affects",
                "decision",
                "d1",
                "derived",
                "citado no texto: \"a\"",
                "2026-02-01T10:00:00Z",
                None,
                Some("2026-02-01T11:00:00Z"),
            ),
            (
                "x3",
                "e3",
                "applies_to",
                "claim",
                "c1",
                "human",
                "",
                "2026-02-01T10:00:00Z",
                Some("2026-02-01T10:00:00Z"),
                None,
            ),
            (
                "x4",
                "e4",
                "affects",
                "decision",
                "d2",
                "human",
                "",
                "2026-02-02T10:02:00Z",
                Some("2026-02-02T10:02:00Z"),
                None,
            ),
            (
                "x5",
                "e5",
                "affects",
                "decision",
                "d2",
                "human",
                "",
                "2026-02-02T16:00:00Z",
                Some("2026-02-02T16:00:00Z"),
                None,
            ),
            (
                "x6",
                "e6",
                "applies_to",
                "claim",
                "c2",
                "derived",
                "herdado da decisão de origem",
                "2026-02-03T10:00:00Z",
                Some("2026-02-03T10:00:00Z"),
                None,
            ),
            (
                "x7",
                "e7",
                "affects",
                "decision",
                "d3",
                "human",
                "",
                "2026-02-04T10:00:00Z",
                Some("2026-02-04T10:00:00Z"),
                None,
            ),
            (
                "x8",
                "e8",
                "affects",
                "decision",
                "d3",
                "derived",
                "crates/a/src/lib.rs",
                "2026-02-04T10:00:00Z",
                None,
                Some("2026-02-04T12:00:00Z"),
            ),
        ];
        for edge in edges {
            raw.execute(
                "INSERT INTO entity_edges (edge_id, project_id, kind, source_kind, source_id, \
                 entity_id, origin, reason, created_at, confirmed_at, invalidated_at) \
                 VALUES (?1, 'p1', ?3, ?4, ?5, ?2, ?6, ?7, ?8, ?9, ?10)",
                rusqlite::params![
                    edge.0, edge.1, edge.2, edge.3, edge.4, edge.5, edge.6, edge.7, edge.8, edge.9
                ],
            )
            .expect("edge");
        }
        // The review's ledger: it accepted the link x1 (AI), discarded x2 (rules),
        // adopted the rule c1 (rules) and the candidate that made d2 (AI).
        let ledger = [
            ("link", "x1", "accepted", "ai", None, "2026-02-01T10:00:00Z"),
            (
                "link",
                "x2",
                "discarded",
                "rules",
                None,
                "2026-02-01T11:00:00Z",
            ),
            (
                "claim",
                "s1",
                "accepted",
                "rules",
                Some("c1"),
                "2026-02-01T10:00:00Z",
            ),
            (
                "candidate",
                "k2",
                "accepted",
                "ai",
                Some("d2"),
                "2026-02-02T10:00:00Z",
            ),
        ];
        for (kind, item, verdict, by, result, at) in ledger {
            raw.execute(
                "INSERT INTO auto_reviews (item_kind, item_id, project_id, verdict, decided_by, \
                 reason, title, result_id, created_at) VALUES (?1, ?2, 'p1', ?3, ?4, 'r', 't', ?5, ?6)",
                rusqlite::params![kind, item, verdict, by, result, at],
            )
            .expect("ledger");
        }
    }

    // Opening the store migrates it.
    let upgraded = storage_sqlite::SqliteStore::open(&path).expect("migrate");
    drop(upgraded);

    let label = |id: &str| edge_labels(&test, id);
    // (a) the ledger names the actor of a settled link.
    assert_eq!(
        label("x1"),
        ("human".into(), "".into(), Some("ai".into()), None)
    );
    assert_eq!(
        label("x2"),
        (
            "derived".into(),
            "citado no texto: \"a\"".into(),
            None,
            Some("rules".into())
        )
    );
    // (b) the rule the review adopted inherited its tie.
    assert_eq!(
        label("x3"),
        (
            "derived".into(),
            "herdado da decisão de origem".into(),
            Some("inherited".into()),
            None
        )
    );
    // (c) a tie made within five minutes of the review's adoption was the review's;
    // one made hours later was a person's.
    assert_eq!(
        label("x4"),
        ("derived".into(), "".into(), Some("ai".into()), None)
    );
    assert_eq!(
        label("x5"),
        ("human".into(), "".into(), Some("person".into()), None)
    );
    // (d) an inherited tie with no ledger entry.
    assert_eq!(
        label("x6"),
        (
            "derived".into(),
            "herdado da decisão de origem".into(),
            Some("inherited".into()),
            None
        )
    );
    // (e) the rest was done by hand.
    assert_eq!(
        label("x7"),
        ("human".into(), "".into(), Some("person".into()), None)
    );
    assert_eq!(
        label("x8"),
        (
            "derived".into(),
            "crates/a/src/lib.rs".into(),
            None,
            Some("person".into())
        )
    );
}

#[test]
fn a_link_discarded_by_the_machine_comes_back_on_a_structural_reason_and_one_a_person_removed_does_not(
) {
    use application::graph::EntityEdit;
    use domain::entities::EdgeActor;
    for (by, returns) in [
        (EdgeActor::Ai, true),
        (EdgeActor::Rules, true),
        (EdgeActor::Person, false),
    ] {
        let test = support::open("graph-relink", &["p1"]);
        let graph = KnowledgeGraph::new(test.store.clone());
        let core = component(&graph, "core", "libs/core/**");
        let decision = support::decision_with_diff(
            &test.store,
            "p1",
            "relink",
            "Como o core valida a entrada?",
            &["crates/core/src/lib.rs"],
            "",
        );
        // The name in the question is a mention; no pattern covers the file yet.
        graph.refresh_suggestions("p1").expect("mention");
        let pending = graph.suggestions("p1").expect("suggestions");
        assert_eq!(pending.len(), 1);
        assert!(mention_quote(&pending[0].reason).is_some());
        graph
            .invalidate_as(&pending[0].edge_id, by)
            .expect("discard");

        // The component now covers the file: a structural reason.
        graph
            .update_entity(
                &core,
                EntityEdit {
                    name: "core".into(),
                    description: String::new(),
                    patterns: vec!["crates/core/**".into()],
                    aliases: Vec::new(),
                },
            )
            .expect("update");
        let report = graph.refresh_suggestions("p1").expect("structural");
        assert_eq!(report.new_edges, usize::from(returns), "{by:?}");
        let live: Vec<_> = graph
            .suggestions("p1")
            .expect("suggestions")
            .into_iter()
            .filter(|suggestion| suggestion.source.node.id == decision)
            .collect();
        assert_eq!(live.len(), usize::from(returns), "{by:?}");
        if returns {
            assert_eq!(live[0].reason, "crates/core/src/lib.rs");
        }
        // Another refresh changes nothing: a returned link is not discarded again.
        assert_eq!(graph.refresh_suggestions("p1").expect("again").new_edges, 0);
    }
}

fn raw_edge(
    id: &str,
    kind: EdgeKind,
    source: (NodeKind, &str),
    entity: &str,
    reason: &str,
    confirmed_by: Option<domain::entities::EdgeActor>,
) -> application::graph::EdgeRecord {
    application::graph::EdgeRecord {
        edge_id: id.into(),
        project_id: "p1".into(),
        kind,
        source_kind: source.0,
        source_id: source.1.into(),
        entity_id: entity.into(),
        origin: domain::entities::EdgeOrigin::Derived,
        reason: reason.into(),
        created_at: "2026-01-03T00:00:00Z".into(),
        confirmed_at: confirmed_by.map(|_| "2026-01-03T00:00:00Z".into()),
        invalidated_at: None,
        confirmed_by,
        invalidated_by: None,
    }
}

#[test]
fn a_refresh_drops_the_machines_links_that_the_rules_no_longer_support_and_spares_a_persons() {
    use application::graph::{ai_link_reason, mention_reason, GraphStore};
    use domain::entities::EdgeActor;
    let test = support::open("graph-revalidate", &["p1"]);
    let graph = KnowledgeGraph::new(test.store.clone());
    let core = component(&graph, "core", "crates/core/**");
    let negated = |key: &str| {
        support::decision(
            &test.store,
            "p1",
            key,
            "O servico fica independente de core.",
            "Sem acoplar",
        )
    };
    let [d1, d2, d3, d4] = ["d1", "d2", "d3", "d4"].map(negated);
    let fine = support::decision(
        &test.store,
        "p1",
        "d5",
        "Como o core valida?",
        "Pelo schema",
    );
    let quote = mention_reason("O servico fica independente de core.");
    let store = &test.store;
    for edge in [
        raw_edge(
            "pending",
            EdgeKind::Affects,
            (NodeKind::Decision, &d1),
            &core,
            &quote,
            None,
        ),
        raw_edge(
            "by-rules",
            EdgeKind::Affects,
            (NodeKind::Decision, &d2),
            &core,
            &quote,
            Some(EdgeActor::Rules),
        ),
        raw_edge(
            "by-person",
            EdgeKind::Affects,
            (NodeKind::Decision, &d3),
            &core,
            &quote,
            Some(EdgeActor::Person),
        ),
        raw_edge(
            "by-ai-guess",
            EdgeKind::Affects,
            (NodeKind::Decision, &d4),
            &core,
            &ai_link_reason("O servico fica independente de core.", "Rege o core."),
            None,
        ),
        raw_edge(
            "affirmative",
            EdgeKind::Affects,
            (NodeKind::Decision, &fine),
            &core,
            &mention_reason("Como o core valida?"),
            None,
        ),
    ] {
        store.insert_edge(&edge).expect("edge");
    }
    // A rule that inherited a tie its decision does not hold, and one a person applied.
    let orphan = support::decision(store, "p1", "d6", "Como gravar?", "Com cuidado");
    let derived_claim = |statement: &str| {
        application::claims::Claims::new(store.clone())
            .create(application::claims::NewClaim {
                source_version: None,
                qualifiers: Vec::new(),
                project_id: "p1".into(),
                kind: domain::claims::ClaimKind::Constraint,
                statement: statement.into(),
                valid_from: Some("2020-01-01".into()),
                valid_until: None,
                source_decision_id: Some(orphan.clone()),
            })
            .expect("claim")
            .claim_id
    };
    let orphan_claim = derived_claim("Regra herdada");
    let person_claim = derived_claim("Regra aplicada por uma pessoa");
    store
        .insert_edge(&raw_edge(
            "inherited",
            EdgeKind::AppliesTo,
            (NodeKind::Claim, &orphan_claim),
            &core,
            "herdado da decisão de origem",
            Some(EdgeActor::Inherited),
        ))
        .expect("inherited");
    store
        .insert_edge(&raw_edge(
            "applied",
            EdgeKind::AppliesTo,
            (NodeKind::Claim, &person_claim),
            &core,
            "",
            Some(EdgeActor::Person),
        ))
        .expect("applied");

    let report = graph.refresh_suggestions("p1").expect("refresh");
    assert_eq!(report.revalidated, 3);
    let state = |id: &str| {
        let edge = store.get_edge(id).expect("get").expect("edge");
        (edge.invalidated_at.is_some(), edge.invalidated_by)
    };
    assert_eq!(state("pending"), (true, Some(EdgeActor::Rules)));
    assert_eq!(state("by-rules"), (true, Some(EdgeActor::Rules)));
    assert_eq!(state("by-person"), (false, None), "a person's word stays");
    assert_eq!(
        state("by-ai-guess"),
        (false, None),
        "the AI's own quote is not re-judged"
    );
    assert_eq!(state("affirmative"), (false, None));
    assert_eq!(state("inherited"), (true, Some(EdgeActor::Inherited)));
    assert_eq!(state("applied"), (false, None));
    // Idempotent: nothing more to drop, and nothing comes back.
    let again = graph.refresh_suggestions("p1").expect("again");
    assert_eq!(again.revalidated, 0);
    assert_eq!(state("pending"), (true, Some(EdgeActor::Rules)));
}

#[test]
fn a_component_whose_folder_is_gone_is_reported_with_where_its_files_belong() {
    use application::projects::{ProjectRecord, ProjectRepository};
    let test = support::open("graph-phantom", &[]);
    let repo = test.root.join("repo");
    std::fs::create_dir_all(repo.join("crates/sc-session/examples")).expect("dirs");
    std::fs::write(
        repo.join("crates/sc-session/examples/jam.rs"),
        "fn main() {}",
    )
    .expect("file");
    std::fs::write(
        repo.join("crates/sc-session/Cargo.toml"),
        "[package]\nname = \"sc-session\"\n",
    )
    .expect("manifest");
    std::fs::write(
        repo.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/*\"]\n",
    )
    .expect("root");
    test.store
        .insert(&ProjectRecord::new(
            "ws".into(),
            repo.to_string_lossy().replace('\\', "/"),
            "2026-01-01T00:00:00Z".into(),
        ))
        .expect("project");
    let graph = KnowledgeGraph::new(test.store.clone());
    graph.assemble("ws").expect("assemble");
    // The ghost somebody created from a proposal: `examples/**` exists nowhere.
    let ghost = graph
        .create_entity(NewEntity {
            project_id: "ws".into(),
            kind: Some(EntityKind::Component),
            name: "examples".into(),
            patterns: vec!["examples/**".into()],
            ..NewEntity::default()
        })
        .expect("ghost")
        .entity_id;
    let location = repo.to_string_lossy().replace('\\', "/");
    let decision = support::decision_at(
        &test.store,
        "ws",
        &location,
        "jam",
        "Como o exemplo toca?",
        &["examples/jam.rs"],
        "",
    );
    use application::graph::GraphStore;
    test.store
        .insert_edge(&raw_edge_in(
            "ws",
            "ghost-edge",
            &decision,
            &ghost,
            "examples/jam.rs",
        ))
        .expect("edge");

    let report = graph.refresh_suggestions("ws").expect("refresh");
    assert_eq!(
        report.stale_components.len(),
        1,
        "{:?}",
        report.stale_components
    );
    let stale = &report.stale_components[0];
    assert_eq!(stale.name, "examples");
    assert_eq!(stale.patterns, vec!["examples/**"]);
    assert_eq!(stale.owner.as_deref(), Some("sc-session"));
    // Nothing was retired by itself.
    assert!(graph
        .entities("ws")
        .expect("entities")
        .iter()
        .all(|entity| entity.retired_at.is_none()));
}

fn raw_edge_in(
    project: &str,
    id: &str,
    decision: &str,
    entity: &str,
    reason: &str,
) -> application::graph::EdgeRecord {
    application::graph::EdgeRecord {
        project_id: project.into(),
        ..raw_edge(
            id,
            EdgeKind::Affects,
            (NodeKind::Decision, decision),
            entity,
            reason,
            Some(domain::entities::EdgeActor::Person),
        )
    }
}
