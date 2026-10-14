//! Gate for the periodic observation reconciler: a parked project costs no
//! refresh job, a changed manifest costs exactly one, and the check itself is
//! cheap for many projects.

mod support;

use application::observations::reader::LocalObservationReader;
use application::observations::reconcile::ReconcileGate;
use application::observations::refresh::{identity, refresh_project};
use application::observations::*;
use application::projects::canonicalize_location;
use rusqlite::Connection;
use std::time::Instant;

const PACKAGE_JSON: &str =
    r#"{"name":"parked","version":"0.0.0","dependencies":{"left":"^1.0.0"}}"#;

/// Idle ticks simulated, one per 60 s of the real reconciler.
const IDLE_TICKS: usize = 60;
/// Projects and ticks of the cost gate.
const PROJECTS: usize = 64;
const COST_TICKS: usize = 50;
/// Ceiling for one pass over 64 projects with 3 manifests each, measured on
/// 14/10/2026; about five times the measured p95, like the other latency gates.
const RECONCILE_TICK_P95_CEILING_MS: f64 = 100.0;

fn raw(test: &support::TestStore) -> Connection {
    Connection::open(test.root.join("app.db")).expect("raw")
}

/// Points the project at a real directory and returns its canonical path.
fn relocate(test: &support::TestStore, project: &str) -> String {
    let directory = test.root.join(format!("ws-{project}"));
    std::fs::create_dir_all(&directory).expect("workspace");
    let location = canonicalize_location(&directory.to_string_lossy()).expect("canonical");
    raw(test)
        .execute(
            "UPDATE projects SET location = ?1 WHERE id = ?2",
            [&location, project],
        )
        .expect("relocate project");
    location
}

fn source(project: &str, path: &str) -> ObservationSource {
    ObservationSource {
        source_id: identity(project, path),
        project_id: project.into(),
        project_relative_path: path.into(),
        manifest_kind: if path.ends_with("Cargo.toml") {
            ManifestKind::Cargo
        } else {
            ManifestKind::PackageJson
        },
        sha256: None,
        parser_policy_version: "1".into(),
        last_checked_at: None,
        last_check_status: CheckStatus::Missing,
        semantic_cache: None,
    }
}

fn request(project: &str, trigger: &str) -> RefreshRequest {
    RefreshRequest {
        project_id: project.into(),
        capture_trigger: trigger.into(),
        requested_at: "now".into(),
    }
}

/// Registers the manifests as known sources, as a first refresh would.
fn register_sources(test: &support::TestStore, project: &str, paths: &[&str]) {
    let generation = test
        .store
        .request_refresh(&request(project, "seed"))
        .unwrap()
        .unwrap();
    test.store
        .apply_refresh(&RefreshBatch {
            project_id: project.into(),
            expected_generation: generation.generation,
            sources: paths.iter().map(|path| source(project, path)).collect(),
            observations: vec![],
            coverage: Default::default(),
        })
        .unwrap();
}

fn generation(test: &support::TestStore, project: &str) -> i64 {
    raw(test)
        .query_row(
            "SELECT generation FROM observation_refresh WHERE project_id = ?1",
            [project],
            |row| row.get(0),
        )
        .expect("generation")
}

fn queued_jobs(test: &support::TestStore) -> i64 {
    raw(test)
        .query_row(
            "SELECT COUNT(*) FROM jobs WHERE kind = 'refresh_observations' AND state = 'queued'",
            [],
            |row| row.get(0),
        )
        .expect("jobs")
}

/// One reconciler pass over the project, as `main.rs` does it. Returns whether
/// a refresh was scheduled.
fn tick(test: &support::TestStore, gate: &mut ReconcileGate, project: &str) -> bool {
    if !gate.changed(&test.store, project) {
        return false;
    }
    test.store
        .request_refresh(&request(project, "reconciler"))
        .unwrap();
    true
}

#[test]
fn reconcile_idle_gate() {
    let test = support::open("reconcile-idle", &["p1"]);
    let location = relocate(&test, "p1");
    let manifest = std::path::Path::new(&location).join("package.json");
    std::fs::write(&manifest, PACKAGE_JSON).expect("manifest");
    register_sources(&test, "p1", &["package.json"]);
    let current = test
        .store
        .request_refresh(&request("p1", "test"))
        .unwrap()
        .unwrap()
        .generation;
    refresh_project(
        &test.store,
        &LocalObservationReader,
        "p1",
        &location,
        current,
        "test",
    )
    .unwrap();
    raw(&test)
        .execute("UPDATE jobs SET state = 'completed'", [])
        .unwrap();

    let mut gate = ReconcileGate::default();
    let start = generation(&test, "p1");
    let scheduled: Vec<bool> = (0..IDLE_TICKS)
        .map(|_| tick(&test, &mut gate, "p1"))
        .collect();
    println!(
        "gate reconcile idle_ticks={IDLE_TICKS} jobs={} generation_delta={}",
        queued_jobs(&test),
        generation(&test, "p1") - start
    );
    assert!(
        scheduled[0],
        "the first look after opening always refreshes"
    );
    assert_eq!(
        scheduled.iter().filter(|scheduled| **scheduled).count(),
        1,
        "a parked project schedules nothing after the first look"
    );
    assert_eq!(queued_jobs(&test), 1);

    raw(&test)
        .execute("UPDATE jobs SET state = 'completed'", [])
        .unwrap();
    std::fs::write(&manifest, format!("{PACKAGE_JSON}\n\n")).expect("rewrite manifest");
    assert!(tick(&test, &mut gate, "p1"), "a changed manifest refreshes");
    assert!(!tick(&test, &mut gate, "p1"), "once");

    std::fs::write(
        std::path::Path::new(&location).join("Cargo.toml"),
        "[package]\nname = \"parked\"\n",
    )
    .expect("new manifest");
    assert!(
        tick(&test, &mut gate, "p1"),
        "a new root manifest refreshes"
    );
    assert!(!tick(&test, &mut gate, "p1"), "once");
}

#[test]
fn reconcile_tick_latency_gate() {
    let ids: Vec<String> = (0..PROJECTS).map(|index| format!("q{index}")).collect();
    let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
    let test = support::open("reconcile-cost", &refs);
    for id in &ids {
        let location = relocate(&test, id);
        let root = std::path::Path::new(&location);
        std::fs::create_dir_all(root.join("packages/a")).unwrap();
        std::fs::write(root.join("package.json"), PACKAGE_JSON).unwrap();
        std::fs::write(root.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
        std::fs::write(root.join("packages/a/package.json"), PACKAGE_JSON).unwrap();
        register_sources(
            &test,
            id,
            &["package.json", "Cargo.toml", "packages/a/package.json"],
        );
    }
    let mut gate = ReconcileGate::default();
    for id in &ids {
        assert!(gate.changed(&test.store, id), "first look refreshes");
    }
    let mut samples = Vec::new();
    for _ in 0..COST_TICKS {
        let started = Instant::now();
        for id in &ids {
            assert!(!gate.changed(&test.store, id), "{id} did not change");
        }
        samples.push(started.elapsed().as_micros());
    }
    samples.sort_unstable();
    let p50 = samples[samples.len() / 2] as f64 / 1000.0;
    let p95 = samples[samples.len() * 95 / 100] as f64 / 1000.0;
    println!("latency reconcile_tick p50={p50:.2}ms p95={p95:.2}ms projects={PROJECTS}");
    assert!(
        p95 <= RECONCILE_TICK_P95_CEILING_MS,
        "p95 {p95}ms above {RECONCILE_TICK_P95_CEILING_MS}ms"
    );
}
