use std::collections::BTreeSet;

use application::context::ContextPack;
use application::injection::{estimate_tokens, render_compact};
use application::observations::*;

fn pack() -> ContextPack {
    serde_json::from_str(r#"{"project_id":"p","task":"serde","as_of":"2026-10-04T00:00:00Z","budget_chars":8000,"used_chars":0,"decisions":[],"claims":[],"omitted":0}"#).unwrap()
}

fn fact() -> PackObservation {
    PackObservation {
        authority: ObservationAuthority::Descriptive,
        record: ObservationRecord {
            observation_id: "observation12345678".into(),
            project_id: "p".into(),
            version: 2,
            subject: ObservationSubject::Dependency {
                name: "serde".into(),
            },
            value: ObservationValue {
                declared_version: None,
                dependency_category: Some(DependencyCategory::Runtime),
                version_requirement: Some("1".into()),
                target: None,
            },
            path_scope: vec!["crates/backend".into()],
            provenance: ObservationProvenance {
                source_id: "source".into(),
                source_sha256: "hash".into(),
                supporting_sources: vec![],
                parser_policy_version: "1".into(),
                field_pointer: "dependencies.serde".into(),
                capture_trigger: "test".into(),
                commit: None,
            },
            observed_at: "2026-10-04T00:00:00Z".into(),
            status: RecordStatus::Current,
            invalidated_at: None,
            invalidation_reason: None,
        },
    }
}

#[test]
fn descriptive_citation_is_indivisible_and_counts_coverage_frame() {
    let mut pack = pack();
    pack.observations.push(fact());
    let block = render_compact(&pack, 2000, &BTreeSet::new()).unwrap();
    assert!(block.items.is_empty());
    assert_eq!(block.observations.len(), 1);
    assert!(block
        .text
        .contains("O:12345678 v2 observado localmente; não é regra"));
    assert!(block.text.contains("dependencies.serde"));
    assert!(block.text.contains("crates/backend"));
    assert!(block.text.contains("source_sha256"));
    assert!(block.text.contains("Cobertura descritiva"));
    assert_eq!(block.tokens, estimate_tokens(&block.text));
    for budget in 0..block.tokens {
        if let Some(smaller) = render_compact(&pack, budget, &BTreeSet::new()) {
            assert!(smaller.tokens <= budget);
            assert!(smaller.observations.is_empty());
            assert!(!smaller.text.contains("dependencies.serde"));
        }
    }
}

#[test]
fn legacy_unknown_empty_snapshot_does_not_invent_facts() {
    let pack = pack();
    assert!(pack.observation_coverage.unknown);
    assert!(render_compact(&pack, 2000, &BTreeSet::new()).is_none());
}
