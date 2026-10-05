//! Capture episodes remain descriptive and project scoped.
mod support;
use application::capture_episode::{artifact_fact, CaptureEpisodeStore};
use application::captures::CaptureArtifactRecord;

#[test]
fn real_wire_diff_hunks_count_content_prefixes_and_leave_incomplete_snippets_unknown() {
    let snippets = [
        (
            "--- a/src/a.rs\n+++ b/src/a.rs\n@@ -1 +1 @@\n---counter\n+++counter",
            Some((1, 1, 1)),
        ),
        ("@@ -1,2 +1,2 @@\n context\n-old\n+new", Some((1, 1, 1))),
        ("-old\n+new", None),
        ("@@ -1,2 +1,2 @@\n-old\n+new", None),
        ("@@ malformed @@\n-old\n+new", None),
        ("--- a/file\n+++ b/file", None),
    ];
    for (index, (snippet, expected)) in snippets.iter().enumerate() {
        let test = ingest_diff(&format!("wire-{index}"), snippet);
        let rows = test.store.capture_episodes("p", None, 0).unwrap();
        assert_eq!(rows[0].artifacts[0].kind, "diff_hunk");
        assert_eq!(
            rows[0].artifacts[0]
                .diff_counts
                .as_ref()
                .map(|c| (c.hunks, c.additions, c.removals)),
            *expected
        );
        assert!(!rows[0].facts_truncated);
    }
}

#[test]
fn nul_and_multibyte_content_are_byte_bounded_and_never_count_truncated_hunks() {
    for (index, content) in [
        format!("@@ -1 +1 @@\n-old\n+new\0{}", "界".repeat(100_007)),
        format!("@@ -1 +1 @@\n-old\n+{}", "界".repeat(100_007)),
        "@@ -1 +1 @@\n-old\n+new\0".into(),
    ]
    .iter()
    .enumerate()
    {
        let test = ingest_diff(&format!("byte-limit-{index}"), content);
        let row = test.store.capture_episodes("p", None, 0).unwrap().remove(0);
        assert_eq!(row.facts_truncated, content.len() > 65_536);
        assert!(row.artifacts[0].diff_counts.is_none());
        let db = rusqlite::Connection::open(test.root.join("app.db")).unwrap();
        let (length, prefix): (i64, Vec<u8>) = db
            .query_row(
                "SELECT length(CAST(content AS BLOB)),substr(CAST(content AS BLOB),1,65536)
             FROM capture_artifacts",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(length as usize, content.len());
        assert_eq!(prefix.len(), content.len().min(65_536));
        if index == 1 {
            assert!(std::str::from_utf8(&prefix)
                .unwrap_err()
                .error_len()
                .is_none());
        }
    }
}

fn ingest_diff(tag: &str, content: &str) -> support::TestStore {
    use application::captures::{CaptureApi, CaptureIngest};
    use application::projects::{canonicalize_location, ProjectRecord, ProjectRepository};
    use application::{
        artifact_fingerprint, ArtifactKind, CaptureEnvelope, CaptureSource, ProjectRef,
        SourceArtifact,
    };
    let test = support::open(tag, &[]);
    let location = canonicalize_location(&test.root.to_string_lossy()).unwrap();
    test.store
        .insert(&ProjectRecord::new(
            "p".into(),
            location.clone(),
            "now".into(),
        ))
        .unwrap();
    CaptureIngest::new(test.store.clone())
        .ingest(
            &CaptureEnvelope {
                schema_version: 1,
                capture_id: "diff-capture".into(),
                idempotency_key: "diff-key".into(),
                source: CaptureSource {
                    adapter: "opencode".into(),
                    adapter_version: "1".into(),
                    session_id: "session".into(),
                    message_id: "message".into(),
                },
                project: ProjectRef {
                    canonical_path: location,
                },
                observed_at: "2026-01-01T00:00:00Z".into(),
                artifacts: vec![SourceArtifact {
                    artifact_id: "diff-artifact".into(),
                    kind: ArtifactKind::DiffHunk,
                    content: content.into(),
                    metadata: serde_json::Map::new(),
                    fingerprint: artifact_fingerprint(content),
                }],
            },
            "diff-key",
        )
        .unwrap();
    test
}

#[test]
fn upgrade_33_to_34_preserves_legacy_artifacts_without_reconstructing_identity() {
    let test = support::open("episode-upgrade", &["legacy"]);
    let db = rusqlite::Connection::open(test.root.join("app.db")).unwrap();
    // Remove subsequent markers from this disposable fixture to obtain v33.
    // Keep existing assessment columns: migration 39 must handle them idempotently.
    db.execute_batch(
        "DROP TABLE capture_episode_sources;
        DELETE FROM schema_migrations WHERE version IN (38, 39);",
    )
    .unwrap();
    db.execute(
        "INSERT INTO capture_artifacts VALUES
        ('capture-legacy','old-artifact','user_text','old redacted content',
         '{\"role\":\"user\",\"author\":\"not-authenticated\"}','old-hash')",
        [],
    )
    .unwrap();
    let before: (String, String, String) = db.query_row(
        "SELECT content,metadata,fingerprint FROM capture_artifacts WHERE artifact_id='old-artifact'",
        [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    ).unwrap();
    assert_eq!(
        db.query_row("SELECT MAX(version) FROM schema_migrations", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        37
    );
    let upgraded = storage_sqlite::SqliteStore::open(test.root.join("app.db")).unwrap();
    let rows = upgraded.capture_episodes("legacy", None, 0).unwrap();
    assert_eq!(rows.len(), 1);
    assert!(rows[0].provenance.is_none());
    let fact = rows[0]
        .artifacts
        .iter()
        .find(|a| a.artifact_id == "old-artifact")
        .unwrap();
    assert_eq!(fact.role_metadata.as_deref(), Some("user"));
    assert_eq!(fact.redacted_hash, "old-hash");
    let after = db.query_row(
        "SELECT content,metadata,fingerprint FROM capture_artifacts WHERE artifact_id='old-artifact'",
        [], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?)),
    ).unwrap();
    assert_eq!(before, after);
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM capture_episode_sources", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        db.query_row("SELECT MAX(version) FROM schema_migrations", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        39
    );
}

#[test]
fn purge_cascades_episode_sources_and_preserves_other_project_through_clone() {
    use application::projects::ProjectRepository;
    let test = support::open("episode-purge", &["removed", "clone"]);
    let db = rusqlite::Connection::open(test.root.join("app.db")).unwrap();
    for project in ["removed", "clone"] {
        db.execute("INSERT INTO capture_episode_sources VALUES (?1,?2)",
            rusqlite::params![format!("capture-{project}"),
                r#"{"adapter":"opencode","adapter_version":"1","session_id":"same-session","message_id":"same-message","observed_at":null}"#]).unwrap();
    }
    let clone = test.store.clone();
    let preserved = clone.capture_episodes("clone", None, 0).unwrap();
    assert!(test.store.purge("removed").unwrap());
    assert!(clone
        .capture_episodes("removed", None, 0)
        .unwrap()
        .is_empty());
    assert_eq!(clone.capture_episodes("clone", None, 0).unwrap(), preserved);
    for table in [
        "capture_episode_sources",
        "capture_receipts",
        "capture_artifacts",
    ] {
        let count: i64 = db
            .query_row(
                &format!("SELECT COUNT(*) FROM {table} WHERE capture_id='capture-removed'"),
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 0, "{table}");
    }
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM capture_episode_sources", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
}

#[test]
fn twenty_row_pages_are_stable_bounded_and_project_scoped() {
    let test = support::open("episode-pages", &["p1", "p2"]);
    let db = rusqlite::Connection::open(test.root.join("app.db")).unwrap();
    for index in 0..45 {
        for project in ["p1", "p2"] {
            let id = format!("page-{project}-{index:03}");
            db.execute(
                "INSERT INTO capture_receipts VALUES (?1,?1,?2,'same-clock',0)",
                rusqlite::params![id, format!("C:/synthetic/{project}")],
            )
            .unwrap();
        }
    }
    let first = test.store.capture_episodes("p1", None, 0).unwrap();
    assert_eq!(first.len(), 20);
    assert_eq!(first, test.store.capture_episodes("p1", None, 0).unwrap());
    let second = test.store.capture_episodes("p1", None, 20).unwrap();
    let third = test.store.capture_episodes("p1", None, 40).unwrap();
    assert_eq!((second.len(), third.len()), (20, 6));
    let ids: Vec<_> = first
        .iter()
        .chain(&second)
        .chain(&third)
        .map(|r| r.capture_id.as_str())
        .collect();
    assert_eq!(
        ids.iter()
            .copied()
            .collect::<std::collections::HashSet<_>>()
            .len(),
        46
    );
    assert!(ids.iter().all(|id| !id.contains("p2")));
    assert!(ids.windows(2).all(|pair| pair[0] > pair[1]));
    for offset in [46, 47, usize::MAX] {
        assert!(test
            .store
            .capture_episodes("p1", None, offset)
            .unwrap()
            .is_empty());
    }
}

#[test]
fn recognized_secrets_are_absent_from_persisted_source_and_safe_fact_output() {
    use application::captures::{CaptureApi, CaptureIngest};
    use application::projects::{canonicalize_location, ProjectRecord, ProjectRepository};
    use application::{
        artifact_fingerprint, ArtifactKind, CaptureEnvelope, CaptureSource, ProjectRef,
        SourceArtifact,
    };
    let test = support::open("episode-secrets", &[]);
    let location = canonicalize_location(&test.root.to_string_lossy()).unwrap();
    test.store
        .insert(&ProjectRecord::new(
            "p".into(),
            location.clone(),
            "now".into(),
        ))
        .unwrap();
    let ingest = CaptureIngest::new(test.store.clone());
    let mut secrets = vec![
        "github_pat_abcdefghijklmnopqrst",
        "ghp_abcdefghijklmnopqrst",
        "sk-abcdefgh",
        "xoxb-abcdefghij",
        "xoxa-abcdefghij",
        "xoxp-abcdefghij",
        "xoxr-abcdefghij",
        "xoxs-abcdefghij",
        "-----BEGIN PRIVATE KEY-----\nprivate-body\n-----END PRIVATE KEY-----",
    ];
    let assignments: Vec<_> = [
        "token",
        "api_key",
        "secret",
        "password",
        "access_key",
        "authorization",
    ]
    .iter()
    .map(|name| format!("{name}=sensitive-value"))
    .collect();
    secrets.extend(assignments.iter().map(String::as_str));
    for (index, secret) in secrets.iter().enumerate() {
        let content = format!("recorded\n{secret}");
        let metadata = serde_json::json!({"path": secret, "role": "assistant",
            "author": secret, "status": "success", "nested": {"value": secret}})
        .as_object()
        .unwrap()
        .clone();
        let envelope = CaptureEnvelope {
            schema_version: 1,
            capture_id: format!("secret-{index}"),
            idempotency_key: format!("key-{index}"),
            source: CaptureSource {
                adapter: (*secret).into(),
                adapter_version: (*secret).into(),
                session_id: (*secret).into(),
                message_id: (*secret).into(),
            },
            project: ProjectRef {
                canonical_path: location.clone(),
            },
            observed_at: String::new(),
            artifacts: vec![SourceArtifact {
                artifact_id: format!("artifact-{index}"),
                kind: ArtifactKind::UserText,
                fingerprint: artifact_fingerprint(&content),
                content,
                metadata,
            }],
        };
        ingest.ingest(&envelope, &envelope.idempotency_key).unwrap();
        let row = test
            .store
            .capture_episodes("p", Some(&envelope.capture_id), 0)
            .unwrap()
            .remove(0);
        let source = row.provenance.as_ref().unwrap();
        assert!(
            source.adapter.is_none()
                && source.adapter_version.is_none()
                && source.session_id.is_none()
                && source.message_id.is_none()
        );
        assert!(source.observed_at.is_none());
        assert_eq!(row.artifacts[0].role_metadata.as_deref(), Some("assistant"));
        let output = serde_json::to_string(&row).unwrap();
        assert!(!output.contains("\"author\":") && !output.contains("\"status\":"));
        assert!(!output.contains(secret), "output secret {index}");
        let db = rusqlite::Connection::open(test.root.join("app.db")).unwrap();
        let stored: String = db
            .query_row(
                "SELECT provenance FROM capture_episode_sources WHERE capture_id=?1",
                [&envelope.capture_id],
                |r| r.get(0),
            )
            .unwrap();
        assert!(!stored.contains(secret), "source secret {index}");
        let (content, metadata): (String, String) = db
            .query_row(
                "SELECT content,metadata FROM capture_artifacts WHERE capture_id=?1",
                [&envelope.capture_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert!(!content.contains(secret));
        let metadata: serde_json::Value = serde_json::from_str(&metadata).unwrap();
        assert_ne!(metadata["author"].as_str(), Some(*secret));
        assert_ne!(metadata["nested"]["value"].as_str(), Some(*secret));
    }
}

#[test]
fn ingest_keeps_first_source_while_checkpoint_advances_and_replay_is_stable() {
    use application::captures::{CaptureApi, CaptureIngest};
    use application::projects::{canonicalize_location, ProjectRecord, ProjectRepository};
    use application::{CaptureEnvelope, CaptureSource, ProjectRef};
    let test = support::open("episode-source", &[]);
    let location = canonicalize_location(&test.root.to_string_lossy()).unwrap();
    test.store
        .insert(&ProjectRecord::new(
            "p".into(),
            location.clone(),
            "now".into(),
        ))
        .unwrap();
    let ingest = CaptureIngest::new(test.store.clone());
    let mut envelope = CaptureEnvelope {
        schema_version: 1,
        capture_id: "first".into(),
        idempotency_key: "first-key".into(),
        source: CaptureSource {
            adapter: "opencode".into(),
            adapter_version: "1".into(),
            session_id: "session".into(),
            message_id: "message-one".into(),
        },
        project: ProjectRef {
            canonical_path: location,
        },
        observed_at: "2026-01-01T00:00:00Z".into(),
        artifacts: vec![],
    };
    ingest.ingest(&envelope, &envelope.idempotency_key).unwrap();
    envelope.capture_id = "second".into();
    envelope.idempotency_key = "second-key".into();
    envelope.source.message_id = "message-two".into();
    ingest.ingest(&envelope, &envelope.idempotency_key).unwrap();
    envelope.source.message_id = "edited-replay".into();
    assert!(
        ingest
            .ingest(&envelope, &envelope.idempotency_key)
            .unwrap()
            .replayed
    );
    let rows = test.store.capture_episodes("p", None, 0).unwrap();
    assert_eq!(rows.len(), 2);
    let first = rows.iter().find(|r| r.capture_id == "first").unwrap();
    assert_eq!(
        first.provenance.as_ref().unwrap().message_id.as_deref(),
        Some("message-one")
    );
    let second = rows.iter().find(|r| r.capture_id == "second").unwrap();
    assert_eq!(
        second.provenance.as_ref().unwrap().message_id.as_deref(),
        Some("message-two")
    );
    let db = rusqlite::Connection::open(test.root.join("app.db")).unwrap();
    let checkpoint: String = db
        .query_row("SELECT message_id FROM adapter_checkpoints", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(checkpoint, "message-two");
}

#[test]
fn legacy_unknown_wrong_project_and_pagination() {
    let test = support::open("capture-episode", &["p1", "p2"]);
    support::decision(&test.store, "p1", "one", "Question", "Choice");
    let rows = test
        .store
        .capture_episodes("p1", Some("capture-p1"), 0)
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert!(rows[0].provenance.is_none());
    assert!(test
        .store
        .capture_episodes("p2", Some("capture-p1"), 0)
        .unwrap()
        .is_empty());
    assert!(test
        .store
        .capture_episodes("p1", None, usize::MAX)
        .unwrap()
        .is_empty());
}

#[test]
fn snippet_counts_exclude_headers_and_reject_unsafe_metadata() {
    let mut artifact = CaptureArtifactRecord {
        capture_id: "c".into(),
        artifact_id: "a".into(),
        kind: "diff_hunk".into(),
        content: "--- a/src/a.rs\n+++ b/src/a.rs\n@@ -1 +1 @@\n-old\n+new".into(),
        metadata: r#"{"path":"src/ação.rs","role":"assistant","status":"success"}"#.into(),
        fingerprint: "redacted-hash".into(),
    };
    let fact = artifact_fact(&artifact);
    assert_eq!(fact.file_mention.as_deref(), Some("src/ação.rs"));
    let counts = fact.diff_counts.unwrap();
    assert_eq!((counts.hunks, counts.additions, counts.removals), (1, 1, 1));
    artifact.metadata = r#"{"path":"../.env","role":"authenticated-owner"}"#.into();
    let fact = artifact_fact(&artifact);
    assert!(fact.file_mention.is_none());
    assert!(fact.role_metadata.is_none());
    artifact.metadata = "malformed".into();
    artifact.content = "é".repeat(65_537);
    let fact = artifact_fact(&artifact);
    assert!(fact.diff_counts.is_none());
    assert!(fact.file_mention.is_none());
}
