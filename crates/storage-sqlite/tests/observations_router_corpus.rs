#[allow(dead_code)]
#[path = "fixtures/observations_corpus.rs"]
mod corpus;
mod support;

use application::context::{ContextPacks, ContextProvider, ContextRequest};
use application::injection::InjectionMode;
use application::observations::refresh::{identity, refresh_project};
use application::observations::*;
use std::collections::BTreeMap;

struct Reader(BTreeMap<String, Vec<u8>>);
impl ObservationReader for Reader {
    fn read_source(
        &self,
        request: &SourceReadRequest,
    ) -> Result<SourceReadResult, ObservationError> {
        Ok(match self.0.get(&request.project_relative_path) {
            Some(bytes) => SourceReadResult {
                status: CheckStatus::Verified,
                bytes: bytes.clone(),
            },
            None => SourceReadResult {
                status: CheckStatus::Missing,
                bytes: vec![],
            },
        })
    }
}

fn refresh(test: &support::TestStore, reader: &Reader) {
    let generation = test
        .store
        .request_refresh(&RefreshRequest {
            project_id: "p1".into(),
            capture_trigger: "test".into(),
            requested_at: "now".into(),
        })
        .unwrap()
        .unwrap();
    assert_eq!(
        refresh_project(
            &test.store,
            reader,
            "p1",
            "root",
            generation.generation,
            "test"
        )
        .unwrap(),
        ApplyRefreshResult::Applied
    );
}

fn reader(test: &support::TestStore, seeds: &[corpus::ManifestSeed]) -> Reader {
    let sources = seeds
        .iter()
        .map(|seed| ObservationSource {
            source_id: identity("p1", seed.path),
            project_id: "p1".into(),
            project_relative_path: seed.path.into(),
            manifest_kind: seed.kind,
            sha256: None,
            parser_policy_version: "1".into(),
            last_checked_at: None,
            last_check_status: CheckStatus::Missing,
            semantic_cache: None,
        })
        .collect();
    let generation = test
        .store
        .request_refresh(&RefreshRequest {
            project_id: "p1".into(),
            capture_trigger: "seed".into(),
            requested_at: "now".into(),
        })
        .unwrap()
        .unwrap();
    test.store
        .apply_refresh(&RefreshBatch {
            project_id: "p1".into(),
            expected_generation: generation.generation,
            sources,
            observations: vec![],
            coverage: Default::default(),
        })
        .unwrap();
    let mut reader = Reader(
        seeds
            .iter()
            .map(|seed| (seed.path.into(), seed.content.as_bytes().to_vec()))
            .collect(),
    );
    let members: Vec<_> = seeds
        .iter()
        .filter_map(|seed| seed.path.rsplit_once('/').map(|(parent, _)| parent))
        .collect();
    reader.0.insert(
        "Cargo.toml".into(),
        format!(
            "[workspace]\nmembers = [{}]\n",
            members
                .iter()
                .map(|member| format!("\"{member}\""))
                .collect::<Vec<_>>()
                .join(",")
        )
        .into_bytes(),
    );
    reader.0.insert(
        "package.json".into(),
        format!(
            "{{\"workspaces\":[{}]}}",
            members
                .iter()
                .map(|member| format!("\"{member}\""))
                .collect::<Vec<_>>()
                .join(",")
        )
        .into_bytes(),
    );
    reader
}

#[test]
fn reserved_selection_oracles_use_real_refresh_and_sqlite_without_graph() {
    let mut checked = 0;
    for scenario in corpus::SCENARIOS.iter().filter(|scenario| {
        matches!(
            scenario.oracle,
            corpus::Oracle::SelectFacts | corpus::Oracle::NoSelection
        )
    }) {
        let test = support::open(scenario.id, &["p1", "p2"]);
        let reader = reader(&test, scenario.manifests);
        refresh(&test, &reader);
        let pack = ContextPacks::new(test.store.clone())
            .build_pack(ContextRequest {
                project_id: "p1".into(),
                task: scenario.query.into(),
                as_of: None,
                budget_chars: None,
                files: scenario.files.iter().map(|path| (*path).into()).collect(),
            })
            .unwrap();
        if matches!(scenario.oracle, corpus::Oracle::NoSelection) {
            assert!(pack.observations.is_empty(), "{}", scenario.id);
        }
        for expected in scenario.expected_facts {
            assert!(
                pack.observations.iter().any(|fact| {
                    let name = match &fact.record.subject {
                        ObservationSubject::Package { name }
                        | ObservationSubject::Dependency { name } => name,
                    };
                    name == expected.name
                        && fact.authority == expected.authority
                        && fact.record.value.dependency_category == expected.category
                        && fact.record.value.version_requirement.as_deref() == expected.requirement
                        && fact.record.value.declared_version.as_deref()
                            == expected.declared_version
                        && fact.record.value.target.as_deref() == expected.target
                }),
                "{}: {}",
                scenario.id,
                expected.name
            );
        }
        assert!(pack.claims.is_empty());
        assert!(pack.decisions.is_empty());
        checked += 1;
    }
    assert_eq!(checked, 8);
}

#[test]
fn delivery_replay_dirty_generation_versions_removal_and_scope_isolation() {
    let test = support::open("observation-delivery-cas", &["p1", "p2"]);
    let mut reader = reader(&test, corpus::SCENARIOS[0].manifests);
    refresh(&test, &reader);
    let fact = test.store.snapshot("p1").unwrap().observations.into_iter().find(|record|
        matches!(&record.subject, ObservationSubject::Dependency { name } if name == "serde")).unwrap();
    let delivered = DeliveredObservation {
        project_id: "p1".into(),
        observation_id: fact.observation_id,
        version: fact.version,
        source_id: fact.provenance.source_id,
    };
    for (session, mode) in [
        ("s", InjectionMode::Inject),
        ("s", InjectionMode::Shadow),
        ("other", InjectionMode::Inject),
    ] {
        assert!(test
            .store
            .record_observation_delivery(session, "p1", mode, std::slice::from_ref(&delivered), &[])
            .unwrap());
    }
    assert!(!test
        .store
        .record_observation_delivery(
            "s",
            "p1",
            InjectionMode::Inject,
            std::slice::from_ref(&delivered),
            &[]
        )
        .unwrap());
    assert!(!test
        .store
        .record_observation_delivery(
            "s",
            "p2",
            InjectionMode::Inject,
            std::slice::from_ref(&delivered),
            &[]
        )
        .unwrap());
    test.store
        .request_refresh(&RefreshRequest {
            project_id: "p1".into(),
            capture_trigger: "edit".into(),
            requested_at: "now".into(),
        })
        .unwrap();
    assert!(!test
        .store
        .record_observation_delivery(
            "fresh",
            "p1",
            InjectionMode::Inject,
            std::slice::from_ref(&delivered),
            &[]
        )
        .unwrap());
    reader
        .0
        .get_mut("servico/Cargo.toml")
        .unwrap()
        .extend_from_slice(b"\n# changed\n");
    refresh(&test, &reader);
    assert!(!test
        .store
        .record_observation_delivery(
            "fresh",
            "p1",
            InjectionMode::Inject,
            std::slice::from_ref(&delivered),
            &[]
        )
        .unwrap());
    let corrections = test
        .store
        .observation_corrections("s", "p1", InjectionMode::Inject)
        .unwrap();
    assert_eq!(corrections.len(), 1);
    assert_eq!(corrections[0].original, delivered);
    assert!(corrections[0].replacement.as_ref().unwrap().version > delivered.version);
    assert!(test
        .store
        .record_observation_delivery("s", "p1", InjectionMode::Inject, &[], &corrections)
        .unwrap());
    assert!(!test
        .store
        .record_observation_delivery("s", "p1", InjectionMode::Inject, &[], &corrections)
        .unwrap());
    assert!(test
        .store
        .observation_corrections("s", "p1", InjectionMode::Inject)
        .unwrap()
        .is_empty());
    assert_eq!(
        test.store
            .observation_corrections("s", "p1", InjectionMode::Shadow)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        test.store
            .observation_corrections("other", "p1", InjectionMode::Inject)
            .unwrap()
            .len(),
        1
    );
    assert!(test
        .store
        .observation_corrections("new", "p1", InjectionMode::Inject)
        .unwrap()
        .is_empty());
    assert!(test
        .store
        .delivered_observations("s", "p2", InjectionMode::Inject)
        .unwrap()
        .is_empty());
    reader.0.remove("servico/Cargo.toml");
    refresh(&test, &reader);
    let removed = test
        .store
        .observation_corrections("other", "p1", InjectionMode::Inject)
        .unwrap();
    assert_eq!(removed.len(), 1);
    assert!(removed[0].replacement.is_none());
}

#[test]
fn historical_normative_budget_and_dirty_edit_abstention() {
    use application::claims::{Claims, NewClaim};
    use domain::claims::ClaimKind;
    let test = support::open("observation-norm-priority", &["p1"]);
    let reader = reader(&test, corpus::SCENARIOS[0].manifests);
    Claims::new(test.store.clone())
        .create(NewClaim {
            project_id: "p1".into(),
            kind: ClaimKind::Constraint,
            statement: "Não enviar dados pessoais a serviços externos.".into(),
            valid_from: Some("1999-01-01".into()),
            valid_until: None,
            source_decision_id: None,
            source_version: None,
            qualifiers: vec![],
        })
        .unwrap();
    let request = ContextRequest {
        project_id: "p1".into(),
        task: "serde".into(),
        as_of: None,
        budget_chars: Some(500),
        files: vec![],
    };
    let before = ContextPacks::new(test.store.clone())
        .build_pack(request.clone())
        .unwrap();
    refresh(&test, &reader);
    let current = ContextPacks::new(test.store.clone())
        .build_pack(request.clone())
        .unwrap();
    assert_eq!(before.claims, current.claims);
    assert!(!current.claims.is_empty());
    assert!(current.used_chars <= 500);
    let historical = ContextPacks::new(test.store.clone())
        .build_pack(ContextRequest {
            as_of: Some("2000-01-01".into()),
            ..request.clone()
        })
        .unwrap();
    assert!(historical.observations.is_empty());
    assert!(historical.observation_coverage.unknown);
    test.store
        .request_refresh(&RefreshRequest {
            project_id: "p1".into(),
            capture_trigger: "agent_edit".into(),
            requested_at: "now".into(),
        })
        .unwrap();
    let dirty = ContextPacks::new(test.store.clone())
        .build_pack(ContextRequest {
            budget_chars: Some(8000),
            ..request
        })
        .unwrap();
    assert!(dirty.observations.is_empty());
    assert!(dirty.observation_coverage.unknown);
    assert_eq!(dirty.claims, current.claims);
}

#[test]
#[ignore = "local deterministic latency sample, no provider"]
fn observation_query_latency_300_samples() {
    let test = support::open("observation-latency", &["p1"]);
    let reader = reader(&test, corpus::SCENARIOS[0].manifests);
    refresh(&test, &reader);
    let request = ContextRequest {
        project_id: "p1".into(),
        task: "serde".into(),
        files: vec![],
        as_of: None,
        budget_chars: None,
    };
    let packs = ContextPacks::new(test.store.clone());
    for _ in 0..30 {
        packs.build_pack(request.clone()).unwrap();
    }
    let mut samples = Vec::new();
    for _ in 0..300 {
        let start = std::time::Instant::now();
        let pack = packs.build_pack(request.clone()).unwrap();
        samples.push(start.elapsed().as_micros());
        assert_eq!(pack.observations.len(), 1);
        assert_eq!(
            pack.observations[0].authority,
            ObservationAuthority::Descriptive
        );
    }
    samples.sort_unstable();
    println!(
        "observation_query samples=300 warmup=30 p50_us={} p95_us={} p99_us={}",
        samples[149], samples[284], samples[296]
    );
    // The use case has only store ports: no provider or filesystem reader is passed.
}

#[test]
fn raw_status_hash_and_stale_correction_integrity_reject_entire_audit() {
    let test = support::open("observation-integrity", &["p1"]);
    let mut reader = reader(&test, corpus::SCENARIOS[0].manifests);
    refresh(&test, &reader);
    let record = test.store.snapshot("p1").unwrap().observations.remove(0);
    let delivery = DeliveredObservation {
        project_id: "p1".into(),
        observation_id: record.observation_id.clone(),
        version: record.version,
        source_id: record.provenance.source_id.clone(),
    };
    assert!(test
        .store
        .record_observation_delivery(
            "s",
            "p1",
            InjectionMode::Inject,
            std::slice::from_ref(&delivery),
            &[]
        )
        .unwrap());
    let connection = rusqlite::Connection::open(test.root.join("app.db")).unwrap();
    connection
        .execute(
            "UPDATE observation_sources SET sha256='wrong' WHERE source_id=?1",
            [&delivery.source_id],
        )
        .unwrap();
    assert!(!test
        .store
        .record_observation_delivery(
            "new",
            "p1",
            InjectionMode::Inject,
            std::slice::from_ref(&delivery),
            &[]
        )
        .unwrap());
    reader
        .0
        .get_mut("servico/Cargo.toml")
        .unwrap()
        .extend_from_slice(b"\n# replacement\n");
    refresh(&test, &reader);
    let stale = test
        .store
        .observation_corrections("s", "p1", InjectionMode::Inject)
        .unwrap();
    assert_eq!(stale.len(), 1);
    reader.0.remove("servico/Cargo.toml");
    refresh(&test, &reader);
    assert!(!test
        .store
        .record_observation_delivery("s", "p1", InjectionMode::Inject, &[], &stale)
        .unwrap());
    assert_eq!(
        test.store
            .observation_corrections("s", "p1", InjectionMode::Inject)
            .unwrap()
            .len(),
        1
    );
    assert!(test
        .store
        .delivered_observations("new", "p1", InjectionMode::Inject)
        .unwrap()
        .is_empty());
}

#[test]
fn failed_checks_notify_uncertainty_not_removal_and_recovery_preserves_version() {
    let test = support::open("observation-uncertainty", &["p1"]);
    let reader = reader(&test, corpus::SCENARIOS[0].manifests);
    refresh(&test, &reader);
    let record = test.store.snapshot("p1").unwrap().observations.remove(0);
    let delivery = DeliveredObservation {
        project_id: "p1".into(),
        observation_id: record.observation_id.clone(),
        version: record.version,
        source_id: record.provenance.source_id.clone(),
    };
    assert!(test
        .store
        .record_observation_delivery(
            "s",
            "p1",
            InjectionMode::Inject,
            std::slice::from_ref(&delivery),
            &[]
        )
        .unwrap());
    for status in [
        CheckStatus::Unreadable,
        CheckStatus::Unsupported,
        CheckStatus::QuotaExceeded,
    ] {
        let generation = test
            .store
            .request_refresh(&RefreshRequest {
                project_id: "p1".into(),
                capture_trigger: "test".into(),
                requested_at: "now".into(),
            })
            .unwrap()
            .unwrap();
        let mut sources = test.store.current_sources("p1").unwrap();
        for source in &mut sources {
            source.last_check_status = status;
            source.last_checked_at = Some(format!("check-{status:?}"));
        }
        test.store
            .apply_refresh(&RefreshBatch {
                project_id: "p1".into(),
                expected_generation: generation.generation,
                sources,
                observations: vec![],
                coverage: Default::default(),
            })
            .unwrap();
        let snapshot = test.store.snapshot("p1").unwrap();
        let preserved = snapshot
            .observations
            .iter()
            .find(|fact| fact.observation_id == record.observation_id)
            .unwrap();
        assert_eq!(preserved.version, record.version);
        assert_eq!(preserved.status, RecordStatus::Current);
        assert!(preserved.invalidated_at.is_none());
        let notices = test
            .store
            .observation_corrections("s", "p1", InjectionMode::Inject)
            .unwrap();
        assert_eq!(notices.len(), 1);
        assert!(notices[0]
            .reason
            .contains("não foi possível revalidar; não tratar como atual"));
        assert!(!notices[0].reason.contains("remov"));
        assert!(test
            .store
            .record_observation_delivery("s", "p1", InjectionMode::Inject, &[], &notices)
            .unwrap());
        assert!(test
            .store
            .observation_corrections("s", "p1", InjectionMode::Inject)
            .unwrap()
            .is_empty());
    }
    refresh(&test, &reader);
    let recovered = test.store.snapshot("p1").unwrap();
    let recovered = recovered
        .observations
        .iter()
        .find(|fact| fact.observation_id == record.observation_id)
        .unwrap();
    assert_eq!(recovered.version, record.version);
    assert_eq!(recovered.status, RecordStatus::Current);
    let recovery = test
        .store
        .observation_corrections("s", "p1", InjectionMode::Inject)
        .unwrap();
    assert_eq!(recovery.len(), 1);
    assert!(recovery[0].reason.starts_with("revalidada"));
    test.store
        .request_refresh(&RefreshRequest {
            project_id: "p1".into(),
            capture_trigger: "edit".into(),
            requested_at: "now".into(),
        })
        .unwrap();
    assert!(!test
        .store
        .record_observation_delivery("s", "p1", InjectionMode::Inject, &[], &recovery)
        .unwrap());
    assert_eq!(
        test.store
            .observation_corrections("s", "p1", InjectionMode::Inject)
            .unwrap()
            .len(),
        1
    );
    refresh(&test, &reader);
    let recovery = test
        .store
        .observation_corrections("s", "p1", InjectionMode::Inject)
        .unwrap();
    assert!(test
        .store
        .record_observation_delivery("s", "p1", InjectionMode::Inject, &[], &recovery)
        .unwrap());
}

#[test]
fn supporting_root_changed_after_pack_rejects_delivery_and_render_keeps_provenance() {
    use application::injection::{estimate_tokens, render_compact};
    use std::collections::BTreeSet;
    let test = support::open("observation-support-cas", &["p1"]);
    let reader = reader(&test, corpus::SCENARIOS[0].manifests);
    refresh(&test, &reader);
    let mut record = test.store.snapshot("p1").unwrap().observations.remove(0);
    record
        .provenance
        .supporting_sources
        .push(ObservationSupport {
            source_id: "workspace-root".into(),
            source_sha256: "workspace-hash".into(),
            field_pointer: "workspace.dependencies.serde".into(),
        });
    let connection = rusqlite::Connection::open(test.root.join("app.db")).unwrap();
    connection.execute("DELETE FROM observation_sources WHERE project_id='p1' AND project_relative_path='Cargo.toml'", []).unwrap();
    connection.execute("INSERT INTO observation_sources \
        (source_id,project_id,project_relative_path,manifest_kind,sha256,parser_policy_version,last_checked_at,last_check_status) \
        VALUES ('workspace-root','p1','Cargo.toml','\"cargo\"','workspace-hash','1','now','\"verified\"')", []).unwrap();
    connection
        .execute(
            "UPDATE observation_records SET record_json=?1 WHERE observation_id=?2 AND version=?3",
            rusqlite::params![
                application::observations::refresh::encode(&record).unwrap(),
                record.observation_id,
                record.version
            ],
        )
        .unwrap();
    let pack = ContextPacks::new(test.store.clone())
        .build_pack(ContextRequest {
            project_id: "p1".into(),
            task: "".into(),
            files: vec!["servico/src/lib.rs".into()],
            as_of: None,
            budget_chars: None,
        })
        .unwrap();
    let block = render_compact(&pack, 2000, &BTreeSet::new()).unwrap();
    assert!(block.text.contains("workspace-root"));
    assert!(block.text.contains("workspace-hash"));
    assert!(block.text.contains("workspace.dependencies.serde"));
    assert!(block.text.contains("observado localmente; não é regra"));
    assert!(block.items.is_empty());
    assert_eq!(block.tokens, estimate_tokens(&block.text));
    connection
        .execute(
            "UPDATE observation_sources SET sha256='changed-root' WHERE source_id='workspace-root'",
            [],
        )
        .unwrap();
    assert!(!test
        .store
        .record_observation_delivery("s", "p1", InjectionMode::Inject, &block.observations, &[])
        .unwrap());
    assert!(test
        .store
        .delivered_observations("s", "p1", InjectionMode::Inject)
        .unwrap()
        .is_empty());
}
