//! Forward-only original-v28 ownership upgrade and fresh-schema equivalence.
mod support;

use application::context_routing::RoutingStore;
use rusqlite::Connection;

#[test]
fn original_28_upgrades_preserving_cache_and_erasing_unowned_work() {
    let test = support::open("routing-upgrade-28", &["p1"]);
    let path = test.root.join("app.db");
    let db = Connection::open(&path).unwrap();
    // Reconstruct the original v28 schema, keeping every v1-v27 dependency.
    db.execute_batch("DROP TABLE context_routing_entries;")
        .unwrap();
    db.execute_batch(include_str!("../src/migrations/0028_context_routing.sql"))
        .unwrap();
    db.execute("DELETE FROM schema_migrations WHERE version=29", [])
        .unwrap();
    db.execute_batch("INSERT INTO context_routing_entries
        (key,project_id,snapshot,generation,profile_hash,request_json,result_json,state,created_at,expires_at)
        VALUES ('cache','p1','semantic',1,'consent',NULL,'[]','completed',1,9999999999),
        ('pending','p1','semantic',1,'consent','PRIVATE_PENDING',NULL,'pending',1,9999999999),
        ('running','p1','semantic',1,'consent','PRIVATE_RUNNING',NULL,'running',1,9999999999);
        INSERT INTO jobs(id,kind,payload,state,idempotent,attempts,created_at,updated_at) VALUES
        ('context-routing-pending','context_routing','p1','queued',0,0,'2026-01-01','2026-01-01'),
        ('context-routing-running','context_routing','p1','running',0,1,'2026-01-01','2026-01-01'),
        ('unrelated','context_routing','p1','queued',0,0,'2026-01-01','2026-01-01');").unwrap();
    drop(db);
    let upgraded = storage_sqlite::SqliteStore::open(&path).unwrap();
    assert!(upgraded
        .begin("context-routing-pending", "consent")
        .unwrap()
        .is_none());
    assert!(upgraded
        .begin("context-routing-running", "consent")
        .unwrap()
        .is_none());
    let db = Connection::open(&path).unwrap();
    let cached: (String, String) = db
        .query_row(
            "SELECT state,result_json FROM context_routing_entries
        WHERE key='cache'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(cached, ("completed".into(), "[]".into()));
    let erased: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM context_routing_entries
        WHERE key IN ('pending','running') AND state='failed' AND request_json IS NULL
        AND owner_job_id IS NULL",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(erased, 2);
    let cancelled: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM jobs WHERE state='cancelled'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(cancelled, 2);
    let untouched: String = db
        .query_row("SELECT state FROM jobs WHERE id='unrelated'", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(untouched, "queued");
    let version: i64 = db
        .query_row("SELECT MAX(version) FROM schema_migrations", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(version, 29);
    // Fresh and upgraded databases have exactly the same ownership contract.
    let fresh = support::open("routing-fresh-29", &["p1"]);
    let fresh_db = Connection::open(fresh.root.join("app.db")).unwrap();
    for connection in [&db, &fresh_db] {
        let owners: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('context_routing_entries')
            WHERE name='owner_job_id'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(owners, 1);
        let index: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
            WHERE type='index' AND name='context_routing_owner'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(index, 1);
    }
    storage_sqlite::SqliteStore::open(&path).unwrap();
}
