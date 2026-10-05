//! Reserved fixture evaluation through local files and SQLite, never seeded facts.
#[allow(dead_code)]
#[path = "fixtures/observations_corpus.rs"]
mod corpus;

use application::context::{ContextPacks, ContextProvider, ContextRequest};
use application::injection::{render_compact, InjectionMode};
use application::observations::reader::LocalObservationReader;
use application::observations::refresh::{refresh_project_with_metrics, RefreshMetrics};
use application::observations::*;
use application::projects::{ProjectRecord, ProjectRepository};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::Instant;
use storage_sqlite::SqliteStore;

struct Fixture {
    root: PathBuf,
    store: Option<SqliteStore>,
    project: String,
}

impl Fixture {
    fn new(scenario: &corpus::Scenario) -> Self {
        let project = format!(
            "eval-{}-{}-{}",
            scenario.id,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let root = std::env::temp_dir().join(&project);
        std::fs::create_dir_all(&root).unwrap();
        let store = SqliteStore::open(root.join("db.sqlite")).unwrap();
        for id in [&project, "projeto-novo"] {
            ProjectRepository::insert(
                &store,
                &ProjectRecord::new(
                    id.into(),
                    root.join(id).to_string_lossy().into(),
                    "2026-10-04".into(),
                ),
            )
            .unwrap();
        }
        let fixture = Self {
            root,
            store: Some(store),
            project,
        };
        let mut cargo = BTreeSet::new();
        let mut npm = BTreeSet::new();
        for seed in scenario.manifests {
            fixture.write(seed.path, seed.content);
            if let Some((parent, _)) = seed.path.rsplit_once('/') {
                match seed.kind {
                    ManifestKind::Cargo => {
                        cargo.insert(parent);
                    }
                    ManifestKind::PackageJson => {
                        npm.insert(parent);
                    }
                }
            }
        }
        if !cargo.is_empty() {
            fixture.write(
                "Cargo.toml",
                &format!("[workspace]\nmembers={cargo:?}\n")
                    .replace('{', "[")
                    .replace('}', "]"),
            );
        }
        if !npm.is_empty() {
            let members = npm
                .iter()
                .map(|s| format!("\"{s}\""))
                .collect::<Vec<_>>()
                .join(",");
            fixture.write("package.json", &format!("{{\"workspaces\":[{members}]}}"));
        }
        fixture
    }

    fn store(&self) -> &SqliteStore {
        self.store.as_ref().unwrap()
    }
    fn write(&self, path: &str, content: &str) {
        let path = self.root.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }
    fn reopen(&mut self) {
        drop(self.store.take());
        self.store = Some(SqliteStore::open(self.root.join("db.sqlite")).unwrap());
    }
    fn refresh(&self) -> RefreshMetrics {
        let generation = self
            .store()
            .request_refresh(&RefreshRequest {
                project_id: self.project.clone(),
                capture_trigger: "evaluation".into(),
                requested_at: "2026-10-04T00:00:00Z".into(),
            })
            .unwrap()
            .unwrap();
        let (result, metrics) = refresh_project_with_metrics(
            self.store(),
            &LocalObservationReader,
            &self.project,
            &self.root.to_string_lossy(),
            generation.generation,
            "evaluation",
        )
        .unwrap();
        assert_eq!(result, ApplyRefreshResult::Applied);
        metrics
    }
    fn request(&self, scenario: &corpus::Scenario) -> ContextRequest {
        ContextRequest {
            project_id: self.project.clone(),
            task: scenario.query.into(),
            as_of: None,
            budget_chars: None,
            files: scenario.files.iter().map(|s| (*s).into()).collect(),
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        drop(self.store.take());
        std::fs::remove_dir_all(&self.root).unwrap();
    }
}

#[test]
fn heldout_selection_eight_scenarios_three_negatives() {
    let mut scenarios = 0;
    let mut required = 0;
    let mut negatives = 0;
    let mut returned = 0;
    let mut relevant = 0;
    for scenario in corpus::SCENARIOS.iter().filter(|s| {
        matches!(
            s.oracle,
            corpus::Oracle::SelectFacts | corpus::Oracle::NoSelection
        )
    }) {
        let fixture = Fixture::new(scenario);
        fixture.refresh();
        let pack = ContextPacks::new(fixture.store().clone())
            .build_pack(fixture.request(scenario))
            .unwrap();
        if matches!(scenario.oracle, corpus::Oracle::NoSelection) {
            assert!(pack.observations.is_empty(), "{}", scenario.id);
            negatives += 1;
        }
        for expected in scenario.expected_facts {
            assert!(pack.observations.iter().any(|fact| {
                let subject = match (&fact.record.subject, &expected.subject) {
                    (ObservationSubject::Package { name }, corpus::Subject::Package)
                    | (ObservationSubject::Dependency { name }, corpus::Subject::Dependency) =>
                        name == expected.name,
                    _ => false,
                };
                subject && fact.authority == expected.authority
                    && fact.record.path_scope.contains(&expected.source_path.to_string())
                    && fact.record.value.dependency_category == expected.category
                    && fact.record.value.declared_version.as_deref() == expected.declared_version
                    && fact.record.value.version_requirement.as_deref() == expected.requirement
                    && fact.record.value.target.as_deref() == expected.target
                    && !fact.record.provenance.source_sha256.is_empty()
                    && !fact.record.provenance.field_pointer.is_empty()
            }), "{}: {}", scenario.id, expected.name);
            required += 1;
        }
        assert!(pack.claims.is_empty());
        assert!(pack.decisions.is_empty());
        if matches!(scenario.oracle, corpus::Oracle::SelectFacts) {
            returned += pack.observations.len();
            relevant += pack
                .observations
                .iter()
                .filter(|fact| {
                    scenario.expected_facts.iter().any(|expected| {
                        let name = match &fact.record.subject {
                            ObservationSubject::Package { name }
                            | ObservationSubject::Dependency { name } => name,
                        };
                        name == expected.name
                            && fact
                                .record
                                .path_scope
                                .contains(&expected.source_path.to_string())
                    })
                })
                .count();
        }
        println!(
            "CASE {} selection/provenance/descriptive-authority passed",
            scenario.id
        );
        scenarios += 1;
    }
    assert_eq!((scenarios, required, negatives), (8, 11, 3));
    println!("selection scenarios=8/8 required_fact_recall=11/11 negative_clean=3/3 positive_required_oracle_precision={relevant}/{returned}");
}

#[test]
fn fixture_lifecycle_versions_removal_failure_isolation_and_cache() {
    for index in [8, 9, 10, 13, 15] {
        let scenario = &corpus::SCENARIOS[index];
        let mut fixture = Fixture::new(scenario);
        fixture.refresh();
        let original = fixture.store().snapshot(&fixture.project).unwrap();
        let dependency = original
            .observations
            .iter()
            .find(|r| {
                matches!(
                    &r.subject, ObservationSubject::Dependency { name } if name == "serde"
                )
            })
            .unwrap()
            .clone();
        for action in scenario.actions.iter().skip(1) {
            match action {
                corpus::Action::ReplaceManifest { path, content } => fixture.write(path, content),
                corpus::Action::RemoveManifest { path } => {
                    std::fs::remove_file(fixture.root.join(path)).unwrap();
                }
                corpus::Action::MakeUnreadable { path } => {
                    std::fs::remove_file(fixture.root.join(path)).unwrap();
                    std::fs::create_dir(fixture.root.join(path)).unwrap();
                }
                corpus::Action::Refresh => {
                    let metrics = fixture.refresh();
                    if index == 15 {
                        // Stale durable policy must bypass durable semantic reuse.
                        // Process syntax cache may still reuse the current-policy tree.
                        assert_eq!(metrics.parser_calls + metrics.cache_hits, 2);
                    }
                }
                corpus::Action::QueryProject { project } => {
                    let mut request = fixture.request(scenario);
                    request.project_id = (*project).into();
                    let pack = ContextPacks::new(fixture.store().clone())
                        .build_pack(request)
                        .unwrap();
                    assert!(pack.observations.is_empty());
                    assert!(fixture
                        .store()
                        .observation_corrections("fresh", project, InjectionMode::Inject)
                        .unwrap()
                        .is_empty());
                }
                corpus::Action::ChangeParserPolicy { version } => {
                    fixture.reopen();
                    let metrics = fixture.refresh();
                    assert_eq!(metrics.parser_calls, 0);
                    assert_eq!(metrics.cache_hits, 2);
                    assert_eq!(
                        fixture
                            .store()
                            .snapshot(&fixture.project)
                            .unwrap()
                            .observations,
                        original.observations
                    );
                    // Invalidate persisted policy, not facts; production policy is compile-time.
                    let connection =
                        rusqlite::Connection::open(fixture.root.join("db.sqlite")).unwrap();
                    connection.execute("UPDATE observation_sources SET parser_policy_version=?1 WHERE project_id=?2", rusqlite::params![version, fixture.project]).unwrap();
                }
                _ => panic!("unhandled action: {}", scenario.id),
            }
        }
        let current = fixture.store().snapshot(&fixture.project).unwrap();
        match scenario.oracle {
            corpus::Oracle::ReplaceVersion => {
                let changed = current
                    .observations
                    .iter()
                    .find(|r| r.observation_id == dependency.observation_id)
                    .unwrap();
                assert!(changed.version > dependency.version);
                assert_eq!(changed.value.version_requirement.as_deref(), Some("2"));
                let connection =
                    rusqlite::Connection::open(fixture.root.join("db.sqlite")).unwrap();
                let old: String = connection.query_row("SELECT record_json FROM observation_records WHERE observation_id=?1 AND version=?2", rusqlite::params![dependency.observation_id, dependency.version], |r| r.get(0)).unwrap();
                let old: ObservationRecord =
                    application::observations::refresh::decode(&old).unwrap();
                assert!(old.invalidated_at.is_some());
            }
            corpus::Oracle::InvalidateRemoved => assert!(current.observations.is_empty()),
            corpus::Oracle::UnreadableIsNotAbsence => {
                assert!(current.coverage.unknown);
                assert!(current
                    .observations
                    .iter()
                    .any(|r| r.observation_id == dependency.observation_id
                        && r.version == dependency.version
                        && r.invalidated_at.is_none()));
                let pack = ContextPacks::new(fixture.store().clone())
                    .build_pack(fixture.request(scenario))
                    .unwrap();
                assert!(pack.observations.is_empty());
            }
            corpus::Oracle::CacheAndPolicy => {
                assert!(current
                    .sources
                    .iter()
                    .filter(|s| s.last_check_status == CheckStatus::Verified)
                    .all(|s| s.parser_policy_version
                        == application::observations::refresh::PARSER_POLICY_VERSION));
            }
            corpus::Oracle::IsolatedProject => {}
            _ => unreachable!(),
        }
        println!("CASE {} lifecycle oracle passed", scenario.id);
    }
    println!("lifecycle fixture_scenarios=5/5; policy stale persisted identity only, not release-version test");
}

#[test]
fn quotas_actual_local_reader_independent_boundaries() {
    let scenario = &corpus::SCENARIOS[11];
    for action in scenario.actions {
        let corpus::Action::QuotaProbe {
            dimension,
            at_limit,
            over_limit,
        } = action
        else {
            panic!("quota fixture action");
        };
        for (size, over) in [(*at_limit, false), (*over_limit, true)] {
            let fixture = Fixture::new(scenario);
            std::fs::remove_file(fixture.root.join("servico/Cargo.toml")).unwrap();
            let label = match dimension {
                corpus::QuotaDimension::Sources => {
                    // Both candidate roots count, including missing package.json.
                    let members = (0..size - 2)
                        .map(|i| format!("'m{i}'"))
                        .collect::<Vec<_>>()
                        .join(",");
                    fixture.write("Cargo.toml", &format!("[workspace]\nmembers=[{members}]\n"));
                    for i in 0..size - 2 {
                        fixture.write(
                            &format!("m{i}/Cargo.toml"),
                            &format!("[package]\nname='m{i}'\nversion='1'\n"),
                        );
                    }
                    "sources"
                }
                corpus::QuotaDimension::SourceBytes => {
                    let base = "[package]\nname='boundary'\nversion='1'\n#";
                    fixture.write(
                        "Cargo.toml",
                        &format!("{base}{}", "x".repeat(size - base.len())),
                    );
                    "source_bytes"
                }
                corpus::QuotaDimension::RefreshBytes => {
                    let members = (0..8)
                        .map(|i| format!("'m{i}'"))
                        .collect::<Vec<_>>()
                        .join(",");
                    let root = format!("[workspace]\nmembers=[{members}]\n");
                    fixture.write("Cargo.toml", &root);
                    let mut remaining = size - root.len();
                    for i in 0..8 {
                        let bytes = remaining.min(256 * 1024);
                        let base = format!("[package]\nname='m{i}'\nversion='1'\n#");
                        fixture.write(
                            &format!("m{i}/Cargo.toml"),
                            &format!("{base}{}", "x".repeat(bytes - base.len())),
                        );
                        remaining -= bytes;
                    }
                    assert_eq!(remaining, 0);
                    "refresh_bytes"
                }
                corpus::QuotaDimension::Observations => {
                    let deps = (0..size).map(|i| format!("d{i}='1'\n")).collect::<String>();
                    fixture.write("Cargo.toml", &format!("[dependencies]\n{deps}"));
                    "observations"
                }
            };
            fixture.refresh();
            let snapshot = fixture.store().snapshot(&fixture.project).unwrap();
            assert_eq!(snapshot.coverage.partial, over, "{label} size={size}");
            assert!(snapshot.sources.len() <= 64);
            assert!(snapshot.observations.len() <= 1024);
            if over {
                assert!(snapshot.coverage.unknown);
            } else if matches!(dimension, corpus::QuotaDimension::Observations) {
                assert_eq!(snapshot.observations.len(), 1024);
            }
            println!(
                "CASE quota_boundaries {label} size={size} partial={} verified={} quota_sources={}",
                snapshot.coverage.partial,
                snapshot.coverage.verified_sources,
                snapshot.coverage.quota_exceeded_sources
            );
        }
    }
}

#[test]
fn session_corrections_render_delivery_versions_removal_unknown_recovery() {
    let scenario = &corpus::SCENARIOS[12];
    let fixture = Fixture::new(scenario);
    fixture.refresh();
    let pack = ContextPacks::new(fixture.store().clone())
        .build_pack(fixture.request(scenario))
        .unwrap();
    let block = render_compact(&pack, 2000, &BTreeSet::new()).unwrap();
    assert_eq!(block.observations.len(), 1);
    let original = block.observations[0].clone();
    let scopes = [
        ("sessao-a", InjectionMode::Inject),
        ("sessao-a", InjectionMode::Shadow),
        ("sessao-b", InjectionMode::Inject),
    ];
    for (session, mode) in scopes {
        assert!(fixture
            .store()
            .record_observation_delivery(session, &fixture.project, mode, &block.observations, &[])
            .unwrap());
    }
    for (content, reason) in [
        (None, "unknown"),
        (Some(scenario.manifests[0].content), "recovery"),
        (
            Some("[package]\nname='servico'\nversion='0.1.0'\n[dependencies]\nserde='2'\n"),
            "change",
        ),
        (
            Some("[package]\nname='servico'\nversion='0.1.0'\n"),
            "removed",
        ),
    ] {
        let path = fixture.root.join("servico/Cargo.toml");
        if path.is_dir() {
            std::fs::remove_dir(&path).unwrap();
        }
        if let Some(content) = content {
            fixture.write("servico/Cargo.toml", content);
        } else {
            std::fs::remove_file(&path).unwrap();
            std::fs::create_dir(&path).unwrap();
        }
        fixture.refresh();
        for (session, mode) in scopes {
            let corrections = fixture
                .store()
                .observation_corrections(session, &fixture.project, mode)
                .unwrap();
            assert_eq!(corrections.len(), 1, "{reason} {session} {mode:?}");
            assert_eq!(corrections[0].original, original);
            match reason {
                "change" => {
                    assert!(corrections[0].replacement.as_ref().unwrap().version > original.version)
                }
                "unknown" => {
                    assert!(corrections[0].reason.contains("não foi possível revalidar"));
                    assert!(!corrections[0].reason.contains("remov"));
                }
                "recovery" => assert!(corrections[0].reason.starts_with("revalidada")),
                "removed" => assert!(corrections[0].replacement.is_none()),
                _ => unreachable!(),
            }
            assert!(fixture
                .store()
                .record_observation_delivery(session, &fixture.project, mode, &[], &corrections)
                .unwrap());
            assert!(!fixture
                .store()
                .record_observation_delivery(session, &fixture.project, mode, &[], &corrections)
                .unwrap());
            assert!(fixture
                .store()
                .observation_corrections(session, &fixture.project, mode)
                .unwrap()
                .is_empty());
        }
        assert!(fixture
            .store()
            .observation_corrections("sessao-nova", &fixture.project, InjectionMode::Inject)
            .unwrap()
            .is_empty());
        assert!(fixture
            .store()
            .observation_corrections("sessao-a", "projeto-novo", InjectionMode::Inject)
            .unwrap()
            .is_empty());
    }
    println!("CASE session_correction rendered delivery; original version; change/unknown/recovery/removal; once per session/project/mode passed");
}

#[test]
fn historical_unknown_and_normative_budget_retention() {
    use application::claims::{Claims, NewClaim};
    use domain::claims::ClaimKind;
    let scenario = &corpus::SCENARIOS[14];
    let fixture = Fixture::new(scenario);
    Claims::new(fixture.store().clone())
        .create(NewClaim {
            project_id: fixture.project.clone(),
            kind: ClaimKind::Constraint,
            statement: "Não enviar dados pessoais a serviços externos.".into(),
            valid_from: Some("1999-01-01".into()),
            valid_until: None,
            source_decision_id: None,
            source_version: None,
            qualifiers: vec![],
        })
        .unwrap();
    let packs = ContextPacks::new(fixture.store().clone());
    let baseline = [500, 8000].map(|budget| {
        let mut request = fixture.request(scenario);
        request.budget_chars = Some(budget);
        packs.build_pack(request).unwrap()
    });
    fixture.refresh();
    let mut historical = fixture.request(scenario);
    historical.as_of = Some("2000-01-01T00:00:00Z".into());
    let historical = packs.build_pack(historical).unwrap();
    assert!(historical.observations.is_empty());
    assert!(historical.observation_coverage.unknown);
    for (budget, before) in [500, 8000].into_iter().zip(baseline) {
        let mut request = fixture.request(scenario);
        request.budget_chars = Some(budget);
        let after = packs.build_pack(request).unwrap();
        assert!(!before.claims.is_empty());
        assert_eq!(before.claims, after.claims);
        assert!(after.used_chars >= before.used_chars);
        if after.observations.is_empty() {
            assert_eq!(before.used_chars, after.used_chars);
        }
        assert!(after.used_chars <= budget);
        assert!(after
            .observations
            .iter()
            .all(|o| o.authority == ObservationAuthority::Descriptive));
        let block = render_compact(&after, 4000, &BTreeSet::new()).unwrap();
        assert!(block.text.contains("Não enviar dados pessoais"));
        if !block.observations.is_empty() {
            assert!(block.text.contains("observado localmente; não é regra"));
        }
    }
    println!("CASE historical_and_normative_priority historical unknown; normative claims retained budgets=500,8000; total used_chars includes descriptive text; descriptive authority passed");
}

#[test]
#[ignore = "local scoped latency report; no provider calls or human measurements"]
fn scoped_latency_report() {
    let scenario = &corpus::SCENARIOS[0];
    let mut samples = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    for _ in 0..30 {
        let mut fixture = Fixture::new(scenario);
        let start = Instant::now();
        let cold = fixture.refresh();
        samples[0].push(start.elapsed().as_micros());
        assert_eq!(cold.parser_calls, 2);
        fixture.reopen();
        let start = Instant::now();
        let warm = fixture.refresh();
        samples[1].push(start.elapsed().as_micros());
        assert_eq!((warm.parser_calls, warm.cache_hits), (0, 2));
        let packs = ContextPacks::new(fixture.store().clone());
        let start = Instant::now();
        let pack = packs.build_pack(fixture.request(scenario)).unwrap();
        samples[2].push(start.elapsed().as_micros());
        let start = Instant::now();
        let block = render_compact(&pack, 2000, &BTreeSet::new()).unwrap();
        samples[3].push(start.elapsed().as_micros());
        assert!(block.items.is_empty());
        assert!(block.text.contains("observado localmente; não é regra"));
    }
    for (label, values) in [
        "refresh_cold",
        "refresh_warm_reopened",
        "build_pack",
        "render",
    ]
    .into_iter()
    .zip(samples.iter_mut())
    {
        values.sort_unstable();
        println!(
            "{label} samples=30 p50_us={} p95_us={} provider_calls=0",
            values[14], values[28]
        );
    }
}
