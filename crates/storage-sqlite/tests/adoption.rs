//! Adoption over SQLite: what the candidate's own evidence ties it to, links
//! confirmed together with the decision, declined ties never suggested again,
//! and the rest of the map derived at once.

mod support;

use application::adoption::{Adoption, ProposedLink};
use application::captures::{
    CaptureArtifactRecord, CaptureCheckpointRecord, CaptureReceiptRecord, CaptureRepository,
    CaptureWrite,
};
use application::extract::{DecisionCandidateRecord, ExtractionStore};
use application::graph::{KnowledgeGraph, NewEntity};
use application::jobs::{JobRecord, JobState, ANALYZE_CAPTURE_KIND};
use domain::entities::{EdgeKind, EntityKind};
use storage_sqlite::SqliteStore;

const MANIFEST: &str = "diff --git a/crates/storage-sqlite/Cargo.toml \
                        b/crates/storage-sqlite/Cargo.toml\n@@ -1 +1,2 @@\n \
                        [dependencies]\n+rusqlite = \"0.31\"\n";

/// A pending candidate whose evidence is one manifest hunk; it cites a file
/// of `storage-sqlite`, one of the desktop and one nobody covers.
fn candidate(store: &SqliteStore, kind: &str) {
    let at = "2026-01-02T00:00:00Z".to_string();
    store
        .insert_capture(&CaptureWrite {
            receipt: CaptureReceiptRecord {
                capture_id: "capture-adopt".into(),
                idempotency_key: "key-adopt".into(),
                canonical_path: "C:/synthetic/p1".into(),
                received_at: at.clone(),
                artifact_count: 1,
            },
            artifacts: vec![CaptureArtifactRecord {
                capture_id: "capture-adopt".into(),
                artifact_id: "art-manifest".into(),
                kind: "diff_hunk".into(),
                content: MANIFEST.into(),
                metadata: "{}".into(),
                fingerprint: format!("{:0>64}", 3),
            }],
            job: JobRecord {
                id: "job-adopt".into(),
                kind: ANALYZE_CAPTURE_KIND.into(),
                payload: "capture-adopt".into(),
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
                capture_id: "capture-adopt".into(),
                observed_at: at.clone(),
                updated_at: at.clone(),
            },
        })
        .expect("capture");
    store
        .insert_candidates(&[DecisionCandidateRecord {
            id: "cand-adopt".into(),
            project_id: "p1".into(),
            capture_id: "capture-adopt".into(),
            status: "pending".into(),
            question: "Onde guardar as decisões?".into(),
            choice: "Em SQLite, via rusqlite.".into(),
            rationale: "Arquivo local.".into(),
            signals: "[\"public_contract\"]".into(),
            confidence: 0.8,
            confidence_reason: "sintético".into(),
            evidence_refs: "[\"art-manifest\"]".into(),
            diff_summary: "{\"files\":[\"crates/storage-sqlite/Cargo.toml\",\
                           \"apps/desktop/src/main.rs\",\"tools/run.ps1\"],\"artifacts\":1}"
                .into(),
            dedup_hash: "dedup-adopt".into(),
            created_at: at.clone(),
            updated_at: at,
            kind: kind.into(),
            significance: 0.8,
            criteria: "[]".into(),
        }])
        .expect("candidate");
}

fn entities(store: &SqliteStore) -> (String, String, String) {
    let graph = KnowledgeGraph::new(store.clone());
    let create = |kind, name: &str, patterns: Vec<String>, aliases: Vec<String>| {
        graph
            .create_entity(NewEntity {
                project_id: "p1".into(),
                kind: Some(kind),
                name: name.into(),
                patterns,
                aliases,
                ..NewEntity::default()
            })
            .expect("entity")
            .entity_id
    };
    (
        create(
            EntityKind::Component,
            "storage-sqlite",
            vec!["crates/storage-sqlite/**".into()],
            Vec::new(),
        ),
        create(
            EntityKind::Component,
            "desktop",
            vec!["apps/desktop/**".into()],
            Vec::new(),
        ),
        create(
            EntityKind::Technology,
            "SQLite",
            Vec::new(),
            vec!["rusqlite".into()],
        ),
    )
}

#[test]
fn adopting_links_what_was_kept_and_rejects_what_was_declined() {
    let test = support::open("adoption", &["p1"]);
    let (storage, desktop, sqlite) = entities(&test.store);
    candidate(&test.store, "decision");
    let adoption = Adoption::new(test.store.clone());

    let preview = adoption.preview("cand-adopt").expect("preview");
    let ties: Vec<(EdgeKind, String)> = preview
        .links
        .iter()
        .map(|link| (link.kind, link.entity_id.clone()))
        .collect();
    assert_eq!(
        ties,
        vec![
            (EdgeKind::Affects, storage.clone()),
            (EdgeKind::Affects, desktop.clone()),
            (EdgeKind::Uses, sqlite.clone()),
        ]
    );
    assert_eq!(preview.uncovered, vec!["tools/run.ps1".to_string()]);

    // Keep storage and SQLite, drop the desktop.
    let kept: Vec<ProposedLink> = preview
        .links
        .iter()
        .filter(|link| link.entity_id != desktop)
        .cloned()
        .collect();
    let declined: Vec<ProposedLink> = preview
        .links
        .iter()
        .filter(|link| link.entity_id == desktop)
        .cloned()
        .collect();
    let outcome = adoption
        .adopt("cand-adopt", None, &kept, &declined)
        .expect("adopt");
    assert_eq!(outcome.linked, 2);
    assert!(!outcome.rule);
    let queued: i64 = rusqlite::Connection::open(test.root.join("app.db"))
        .expect("raw")
        .query_row(
            "SELECT COUNT(*) FROM jobs WHERE kind = ?1 AND payload = ?2",
            [
                application::relation_suggestions::RELATION_JOB_KIND,
                outcome.id.as_str(),
            ],
            |row| row.get(0),
        )
        .expect("jobs");
    assert_eq!(queued, 1, "relations are looked for in the background");

    let graph = KnowledgeGraph::new(test.store.clone());
    let detail = graph.entity_detail(&storage, None).expect("storage");
    assert_eq!(detail.decisions.len(), 1, "confirmed with the adoption");
    assert!(
        graph
            .suggestions("p1")
            .expect("suggestions")
            .iter()
            .all(|suggestion| suggestion.entity.node.id != desktop),
        "a declined tie is not suggested again"
    );
    graph.refresh_suggestions("p1").expect("refresh");
    assert!(graph
        .suggestions("p1")
        .expect("again")
        .iter()
        .all(|suggestion| suggestion.entity.node.id != desktop));
}

#[test]
fn an_adopted_rule_applies_to_the_components_it_touched() {
    let test = support::open("adoption-rule", &["p1"]);
    let (storage, _desktop, _sqlite) = entities(&test.store);
    candidate(&test.store, "rule");
    let adoption = Adoption::new(test.store.clone());
    let preview = adoption.preview("cand-adopt").expect("preview");
    assert!(preview
        .links
        .iter()
        .all(|link| link.kind == EdgeKind::AppliesTo));
    let kept: Vec<ProposedLink> = preview
        .links
        .iter()
        .filter(|link| link.entity_id == storage)
        .cloned()
        .collect();
    let outcome = adoption
        .adopt("cand-adopt", None, &kept, &[])
        .expect("adopt");
    assert!(outcome.rule);
    assert_eq!(outcome.linked, 1);
    let detail = KnowledgeGraph::new(test.store.clone())
        .entity_detail(&storage, None)
        .expect("storage");
    assert_eq!(detail.claims.len(), 1, "the rule holds on storage now");
}
