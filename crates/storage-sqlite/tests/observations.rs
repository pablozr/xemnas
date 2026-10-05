use application::observations::reader::LocalObservationReader;
use application::observations::refresh::refresh_project;
use application::observations::refresh::refresh_project_with_metrics;
use application::observations::*;
use application::projects::{ProjectRecord, ProjectRepository};
use storage_sqlite::SqliteStore;

mod rewind;

fn fixture() -> (std::path::PathBuf, SqliteStore) {
    let root = std::env::temp_dir().join(format!(
        "observations-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let store = SqliteStore::open(root.join("db.sqlite")).unwrap();
    ProjectRepository::insert(
        &store,
        &ProjectRecord::new(
            "p".into(),
            root.to_string_lossy().into(),
            "2026-10-04T00:00:00Z".into(),
        ),
    )
    .unwrap();
    (root, store)
}

fn refresh(store: &SqliteStore, root: &std::path::Path) {
    let generation = store
        .request_refresh(&RefreshRequest {
            project_id: "p".into(),
            capture_trigger: "test".into(),
            requested_at: "2026-10-04T01:00:00Z".into(),
        })
        .unwrap()
        .unwrap();
    assert_eq!(
        refresh_project(
            store,
            &LocalObservationReader,
            "p",
            &root.to_string_lossy(),
            generation.generation,
            "test"
        )
        .unwrap(),
        ApplyRefreshResult::Applied
    );
}

#[test]
fn snapshot_cache_versions_removal_and_invalid_parse() {
    let (root, store) = fixture();
    std::fs::write(
        root.join("package.json"),
        r#"{"name":"demo","version":"1","dependencies":{"x":"^2"}}"#,
    )
    .unwrap();
    refresh(&store, &root);
    let first = store.snapshot("p").unwrap();
    assert_eq!(first.observations.len(), 2);
    refresh(&store, &root);
    assert_eq!(
        store.snapshot("p").unwrap().observations,
        first.observations
    );
    assert!(store.snapshot("other").unwrap().observations.is_empty());
    std::fs::write(
        root.join("package.json"),
        r#"{"name":"demo","version":"2"}"#,
    )
    .unwrap();
    refresh(&store, &root);
    let changed = store.snapshot("p").unwrap();
    assert_eq!(changed.observations.len(), 1);
    assert_eq!(changed.observations[0].version, 2);
    std::fs::write(root.join("package.json"), "{invalid").unwrap();
    refresh(&store, &root);
    assert_eq!(store.snapshot("p").unwrap().observations.len(), 1);
    assert!(store.snapshot("p").unwrap().coverage.unknown);
    std::fs::remove_file(root.join("package.json")).unwrap();
    refresh(&store, &root);
    assert_eq!(store.snapshot("p").unwrap().coverage.missing_sources, 2);
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn stale_generation_never_overwrites_snapshot() {
    let (root, store) = fixture();
    let request = RefreshRequest {
        project_id: "p".into(),
        capture_trigger: "test".into(),
        requested_at: "2026-10-04T01:00:00Z".into(),
    };
    let old = store.request_refresh(&request).unwrap().unwrap();
    store.request_refresh(&request).unwrap();
    assert_eq!(
        store
            .apply_refresh(&RefreshBatch {
                project_id: "p".into(),
                expected_generation: old.generation,
                sources: vec![],
                observations: vec![],
                coverage: ObservationCoverage::default()
            })
            .unwrap(),
        ApplyRefreshResult::Superseded
    );
    assert!(store.snapshot("p").unwrap().coverage.unknown);
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn reader_rejects_escape_and_bounds_bytes() {
    let (root, store) = fixture();
    std::fs::write(root.join("package.json"), "0123456789").unwrap();
    let mut request = SourceReadRequest {
        project_id: "p".into(),
        project_root: root.to_string_lossy().into(),
        project_relative_path: "../package.json".into(),
        max_bytes: 4,
    };
    assert_eq!(
        LocalObservationReader.read_source(&request).unwrap().status,
        CheckStatus::Unreadable
    );
    request.project_relative_path = "package.json".into();
    assert_eq!(
        LocalObservationReader.read_source(&request).unwrap().status,
        CheckStatus::QuotaExceeded
    );
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn workspace_inheritance_removal_and_reopened_cache() {
    let (root, store) = fixture();
    std::fs::create_dir(root.join("member")).unwrap();
    std::fs::write(root.join("Cargo.toml"),
        "[workspace]\nmembers=['member']\n[workspace.package]\nversion='3.2'\n[workspace.dependencies]\nx='^4'\n").unwrap();
    std::fs::write(
        root.join("member/Cargo.toml"),
        "[package]\nname='child'\nversion.workspace=true\n[dependencies]\nx.workspace=true\n",
    )
    .unwrap();
    refresh(&store, &root);
    let snapshot = store.snapshot("p").unwrap();
    assert_eq!(snapshot.observations.len(), 2);
    assert!(snapshot
        .observations
        .iter()
        .all(|r| r.provenance.field_pointer.contains(";Cargo.toml#")));
    assert!(snapshot
        .observations
        .iter()
        .any(|r| r.value.declared_version.as_deref() == Some("3.2")));
    drop(store);
    let store = SqliteStore::open(root.join("db.sqlite")).unwrap();
    let generation = store
        .request_refresh(&RefreshRequest {
            project_id: "p".into(),
            capture_trigger: "test".into(),
            requested_at: "2026-10-04T02:00:00Z".into(),
        })
        .unwrap()
        .unwrap();
    let (_, metrics) = refresh_project_with_metrics(
        &store,
        &LocalObservationReader,
        "p",
        &root.to_string_lossy(),
        generation.generation,
        "test",
    )
    .unwrap();
    assert_eq!(metrics.parser_calls, 0);
    assert_eq!(metrics.cache_hits, 2);
    assert_eq!(
        store.snapshot("p").unwrap().observations,
        snapshot.observations
    );
    std::fs::write(root.join("Cargo.toml"), "[workspace]\nmembers=[]\n").unwrap();
    refresh(&store, &root);
    assert!(store.snapshot("p").unwrap().observations.is_empty());
    refresh(&store, &root);
    assert!(store.snapshot("p").unwrap().observations.is_empty());
    drop(store);
    let store = SqliteStore::open(root.join("db.sqlite")).unwrap();
    refresh(&store, &root);
    assert!(store.snapshot("p").unwrap().observations.is_empty());
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn cached_workspace_resolves_new_declaration_and_missing_support() {
    let (root, store) = fixture();
    std::fs::create_dir(root.join("member")).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers=['member']\n[workspace.dependencies]\nx='^7'\n",
    )
    .unwrap();
    std::fs::write(
        root.join("member/Cargo.toml"),
        "[package]\nname='child'\nversion='1'\n",
    )
    .unwrap();
    refresh(&store, &root);
    drop(store);
    let store = SqliteStore::open(root.join("db.sqlite")).unwrap();
    std::fs::write(
        root.join("member/Cargo.toml"),
        "[package]\nname='child'\nversion='1'\n[dependencies]\nx.workspace=true\n",
    )
    .unwrap();
    let generation = store
        .request_refresh(&RefreshRequest {
            project_id: "p".into(),
            capture_trigger: "test".into(),
            requested_at: "new".into(),
        })
        .unwrap()
        .unwrap();
    let (_, metrics) = refresh_project_with_metrics(
        &store,
        &LocalObservationReader,
        "p",
        &root.to_string_lossy(),
        generation.generation,
        "test",
    )
    .unwrap();
    assert_eq!(metrics.parser_calls, 1); // changed member only; root never parsed
    let snapshot = store.snapshot("p").unwrap();
    let dependency = snapshot
        .observations
        .iter()
        .find(|r| r.value.version_requirement.is_some())
        .unwrap();
    assert_eq!(dependency.value.version_requirement.as_deref(), Some("^7"));
    assert_eq!(dependency.provenance.supporting_sources.len(), 1);
    assert_eq!(
        dependency.provenance.supporting_sources[0].source_sha256,
        snapshot
            .sources
            .iter()
            .find(|s| s.project_relative_path == "Cargo.toml")
            .unwrap()
            .sha256
            .clone()
            .unwrap()
    );
    let version = dependency.version;
    std::fs::write(
        root.join("Cargo.toml"),
        "# comment changed\n[workspace]\nmembers=['member']\n[workspace.dependencies]\nx='^7'\n",
    )
    .unwrap();
    refresh(&store, &root);
    let updated = store.snapshot("p").unwrap();
    let dependency = updated
        .observations
        .iter()
        .find(|r| r.value.version_requirement.is_some())
        .unwrap();
    assert_eq!(dependency.version, version + 1);
    refresh(&store, &root);
    assert_eq!(
        store.snapshot("p").unwrap().observations,
        updated.observations
    );
    drop(store);
    let store = SqliteStore::open(root.join("db.sqlite")).unwrap();
    refresh(&store, &root);
    assert_eq!(
        store.snapshot("p").unwrap().observations,
        updated.observations
    );
    std::fs::remove_file(root.join("Cargo.toml")).unwrap();
    refresh(&store, &root);
    assert!(store.snapshot("p").unwrap().observations.is_empty());
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn retired_member_does_not_make_surviving_member_suspect() {
    let (root, store) = fixture();
    for member in ["a", "b"] {
        std::fs::create_dir(root.join(member)).unwrap();
        std::fs::write(
            root.join(member).join("Cargo.toml"),
            format!("[package]\nname='{member}'\nversion='1'\n"),
        )
        .unwrap();
    }
    std::fs::write(root.join("Cargo.toml"), "[workspace]\nmembers=['a','b']\n").unwrap();
    refresh(&store, &root);
    std::fs::write(root.join("Cargo.toml"), "[workspace]\nmembers=['b']\n").unwrap();
    refresh(&store, &root);
    drop(store);
    let store = SqliteStore::open(root.join("db.sqlite")).unwrap();
    for _ in 0..2 {
        refresh(&store, &root);
        let snapshot = store.snapshot("p").unwrap();
        assert_eq!(snapshot.observations.len(), 1);
        assert_eq!(
            snapshot
                .sources
                .iter()
                .find(|s| s.project_relative_path == "a/Cargo.toml")
                .unwrap()
                .last_check_status,
            CheckStatus::OutOfScope
        );
        assert!(snapshot.sources.iter().all(|s| !matches!(
            s.last_check_status,
            CheckStatus::Unreadable | CheckStatus::Unsupported | CheckStatus::QuotaExceeded
        )));
    }
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn unknown_root_and_quota_recover_without_revision_churn() {
    struct FailedRoot;
    impl ObservationReader for FailedRoot {
        fn read_source(
            &self,
            request: &SourceReadRequest,
        ) -> Result<SourceReadResult, ObservationError> {
            if request.project_relative_path == "Cargo.toml" {
                Ok(SourceReadResult {
                    status: CheckStatus::QuotaExceeded,
                    bytes: vec![],
                })
            } else {
                LocalObservationReader.read_source(request)
            }
        }
    }
    let (root, store) = fixture();
    std::fs::create_dir(root.join("member")).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers=['member']\n[workspace.package]\nversion='1'\n",
    )
    .unwrap();
    std::fs::write(
        root.join("member/Cargo.toml"),
        "[package]\nname='child'\nversion.workspace=true\n",
    )
    .unwrap();
    refresh(&store, &root);
    let original = store.snapshot("p").unwrap().observations;
    let generation = store
        .request_refresh(&RefreshRequest {
            project_id: "p".into(),
            capture_trigger: "test".into(),
            requested_at: "quota".into(),
        })
        .unwrap()
        .unwrap();
    refresh_project(
        &store,
        &FailedRoot,
        "p",
        &root.to_string_lossy(),
        generation.generation,
        "test",
    )
    .unwrap();
    assert!(store.snapshot("p").unwrap().coverage.unknown);
    assert!(store.snapshot("p").unwrap().observations.is_empty());
    refresh(&store, &root);
    let recovered = store.snapshot("p").unwrap().observations;
    assert_eq!(recovered.len(), original.len());
    assert_eq!(recovered[0].version, original[0].version);
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(windows)]
#[test]
fn registered_root_replaced_by_junction_is_rejected() {
    let (root, store) = fixture();
    let project = root.join("project");
    let external = root.join("external");
    std::fs::create_dir(&project).unwrap();
    std::fs::create_dir(&external).unwrap();
    std::fs::write(external.join("package.json"), "EXTERNAL-MUST-NOT-READ").unwrap();
    std::fs::remove_dir(&project).unwrap();
    let output = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&project)
        .arg(&external)
        .output()
        .unwrap();
    if !output.status.success() {
        eprintln!("SKIP junction creation unavailable: {}", output.status);
    } else {
        let read = LocalObservationReader
            .read_source(&SourceReadRequest {
                project_id: "p".into(),
                project_root: project.to_string_lossy().into(),
                project_relative_path: "package.json".into(),
                max_bytes: 100,
            })
            .unwrap();
        assert_eq!(read.status, CheckStatus::Unreadable);
        assert!(read.bytes.is_empty());
        std::fs::remove_dir(&project).unwrap();
    }
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn upgrade_28_to_current_preserves_project() {
    let (root, store) = fixture();
    drop(store);
    let connection = rusqlite::Connection::open(root.join("db.sqlite")).unwrap();
    // The schema just before descriptive observations (migration 29).
    rewind::rewind(&connection, 28);
    drop(connection);
    let store = SqliteStore::open(root.join("db.sqlite")).unwrap();
    assert!(ProjectRepository::get(&store, "p").unwrap().is_some());
    assert!(store.snapshot("p").unwrap().coverage.unknown);
    drop(store);
    let connection = rusqlite::Connection::open(root.join("db.sqlite")).unwrap();
    let version: i64 = connection
        .query_row("SELECT MAX(version) FROM schema_migrations", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(version, 40);
    drop(connection);
    std::fs::remove_dir_all(root).unwrap();
}
