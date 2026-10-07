//! Scale of rule selection in `build_pack`: a project with about 2,000 rules
//! over 60 components, 20 of them big (100 tied rules each). The ranking of
//! tied rules is an in-memory pass over the claims already loaded:
//! O(candidate rules x task concepts) per pack.

mod support;

use std::time::Instant;

use application::claims::{Claims, NewClaim};
use application::context::{ContextPacks, ContextProvider, ContextRequest};
use application::graph::{KnowledgeGraph, LinkRequest, NewEntity};
use domain::claims::ClaimKind;
use domain::entities::{EdgeKind, EntityKind, NodeKind};

const TOPICS: &[&str] = &[
    "retry",
    "logging",
    "pagination",
    "encryption",
    "timeout",
    "migration",
    "metrics",
    "translation",
    "documentation",
    "deployment",
    "rollback",
    "queue",
    "backup",
    "accessibility",
    "throttling",
    "calendar",
    "validation",
    "licensing",
    "latency",
    "serialization",
    "scheduling",
    "indexing",
    "sharding",
    "caching",
    "webhook",
    "sandbox",
    "telemetry",
    "checkpoint",
    "quota",
    "failover",
];

/// Ceiling of one `build_pack` (p95) at this scale, in microseconds.
const SCALE_P95_CEILING_US: u128 = 10_000;

#[test]
fn rule_ranking_scales_to_a_large_project() {
    let test = support::open("context-scale", &["p1"]);
    let claims = Claims::new(test.store.clone());
    let graph = KnowledgeGraph::new(test.store.clone());
    let mut rule = 0;
    for component in 0..60 {
        let id = graph
            .create_entity(NewEntity {
                project_id: "p1".into(),
                kind: Some(EntityKind::Component),
                name: format!("modulo{component:02}"),
                patterns: vec![format!("crates/modulo{component:02}/**")],
                ..NewEntity::default()
            })
            .expect("component")
            .entity_id;
        let rules = if component < 20 { 100 } else { 3 };
        for n in 0..rules {
            let (a, b) = (
                TOPICS[(component + n) % TOPICS.len()],
                TOPICS[(n * 7 + 3) % TOPICS.len()],
            );
            rule += 1;
            let claim = claims
                .create(NewClaim {
                    source_version: None,
                    qualifiers: Vec::new(),
                    project_id: "p1".into(),
                    kind: ClaimKind::Constraint,
                    statement: format!("Rule {rule}: every {a} must respect {b} limits"),
                    valid_from: Some("2020-01-01".into()),
                    valid_until: None,
                    source_decision_id: None,
                })
                .expect("claim")
                .claim_id;
            graph
                .link(LinkRequest {
                    kind: EdgeKind::AppliesTo,
                    source_kind: NodeKind::Claim,
                    source_id: claim,
                    entity_id: id.clone(),
                })
                .expect("tie");
        }
    }
    assert!(rule >= 2_000);

    let packs = ContextPacks::new(test.store.clone());
    let mut latency = Vec::new();
    let mut ranked = 0;
    for n in 0..300 {
        let component = n % 20;
        let topic = TOPICS[(n * 11) % TOPICS.len()];
        let request = ContextRequest {
            project_id: "p1".into(),
            task: format!("Adjust the {topic} handling of this module"),
            as_of: None,
            budget_chars: Some(12_000),
            files: vec![format!("crates/modulo{component:02}/src/lib.rs")],
        };
        let start = Instant::now();
        let pack = packs.build_pack(request).expect("pack");
        latency.push(start.elapsed().as_micros());
        ranked += pack.claims.len();
    }
    latency.sort_unstable();
    let (p50, p95) = (
        latency[latency.len() / 2],
        latency[latency.len() * 95 / 100],
    );
    println!(
        "latency scale rules={rule} packs={} rules_in_packs={ranked} p50_us={p50} p95_us={p95}",
        latency.len()
    );
    assert!(
        p95 <= SCALE_P95_CEILING_US,
        "build_pack p95 {p95}us above {SCALE_P95_CEILING_US}us"
    );
}
