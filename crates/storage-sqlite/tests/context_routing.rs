//! Background-only routing: baseline, strict output, CAS, consent and bounds.
mod support;

use application::{
    analysis::ExtractorFactory,
    claims::{Claims, NewClaim},
    context::{ContextPacks, ContextProvider, ContextRequest},
    context_routing::{JudgeContextAmbiguity, RoutingLookup, CONTEXT_ROUTING_KIND},
    extract::{
        CandidateExtractor, CandidateProposal, DecisionEvidence, ExtractError, RelevanceSignal,
    },
    jobs::{JobRepository, Jobs},
    overview::StructuredModel,
    profile::{self, AiProfile, AiSettings, ProfileError, ProfileStore, SecretStore},
};
use domain::claims::ClaimKind;
use rusqlite::Connection;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

#[derive(Clone)]
struct Profiles(Arc<Mutex<AiProfile>>);
impl ProfileStore for Profiles {
    fn load(&self) -> Result<Option<AiProfile>, ProfileError> {
        Ok(Some(self.0.lock().unwrap().clone()))
    }
    fn save(&self, p: &AiProfile) -> Result<(), ProfileError> {
        *self.0.lock().unwrap() = p.clone();
        Ok(())
    }
}
#[derive(Clone)]
struct Secrets(Arc<AtomicUsize>);
impl SecretStore for Secrets {
    fn get_secret(&self, _: &str) -> Result<Option<String>, ProfileError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(None)
    }
    fn set_secret(&self, _: &str, _: &str) -> Result<(), ProfileError> {
        Ok(())
    }
    fn delete_secret(&self, _: &str) -> Result<(), ProfileError> {
        Ok(())
    }
}

type Hook = Arc<dyn Fn() + Send + Sync>;
#[derive(Clone)]
struct Factory {
    calls: Arc<AtomicUsize>,
    mode: &'static str,
    hook: Option<Hook>,
}
impl ExtractorFactory for Factory {
    type Extractor = Model;
    fn external(&self, _: &AiProfile, _: String) -> Result<Model, ExtractError> {
        Ok(Model(self.clone()))
    }
}
struct Model(Factory);
impl CandidateExtractor for Model {
    fn extract(
        &self,
        _: &DecisionEvidence,
        _: &[RelevanceSignal],
    ) -> Result<Vec<CandidateProposal>, ExtractError> {
        panic!("routing does not extract memory")
    }
}
impl StructuredModel for Model {
    fn complete(
        &self,
        _: &str,
        user: &str,
        _: &str,
        _: &serde_json::Value,
    ) -> Result<String, ExtractError> {
        self.0.calls.fetch_add(1, Ordering::SeqCst);
        if let Some(hook) = &self.0.hook {
            hook();
        }
        let input: serde_json::Value = serde_json::from_str(user).unwrap();
        let mut rows: Vec<_> = input["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(index, c)| {
                serde_json::json!({"id":c["id"],"kind":c["kind"],
                "revision":c["revision"],"label":if index == 0 {"irrelevant"} else {"abstain"}})
            })
            .collect();
        match self.0.mode {
            "malformed" => return Ok("not json".into()),
            "foreign" => rows[0]["id"] = "foreign".into(),
            "revision" => rows[0]["revision"] = "stale".into(),
            "extra" => rows[0]["command"] = "delete rules".into(),
            "duplicate" => rows[1] = rows[0].clone(),
            "all_irrelevant" => {
                for row in &mut rows {
                    row["label"] = "irrelevant".into()
                }
            }
            _ => (),
        }
        Ok(serde_json::json!({"judgments":rows}).to_string())
    }
}

fn settings() -> (AiSettings<Profiles, Secrets>, Profiles, Arc<AtomicUsize>) {
    let mut p = profile::offline_default_profile();
    p.kind = profile::ProfileKind::OpenAiCompatible;
    p.endpoint = Some("http://127.0.0.1:9/v1".into());
    p.model = "fake-routing-test".into();
    p.max_input_chars = 12_000;
    let p = profile::grant_consent(
        &p,
        &profile::build_preview(&p),
        "2026-01-01T00:00:00Z",
        true,
    )
    .unwrap();
    let profiles = Profiles(Arc::new(Mutex::new(p)));
    let secrets = Arc::new(AtomicUsize::new(0));
    (
        AiSettings::new(profiles.clone(), Secrets(secrets.clone())),
        profiles,
        secrets,
    )
}
fn request() -> ContextRequest {
    ContextRequest {
        project_id: "p1".into(),
        task: "cache migration".into(),
        as_of: None,
        budget_chars: None,
        files: vec![],
    }
}
fn seed(test: &support::TestStore) -> String {
    // Creation schedules a refresh: make the local inventory clean for this fixture.
    Connection::open(test.root.join("app.db"))
        .unwrap()
        .execute("UPDATE observation_refresh SET dirty=0", [])
        .unwrap();
    support::decision(
        &test.store,
        "p1",
        "cache",
        "Cache response policy?",
        "Retain short responses",
    );
    support::decision(
        &test.store,
        "p1",
        "migration",
        "Migration execution policy?",
        "Run serially",
    );
    Claims::new(test.store.clone())
        .create(NewClaim {
            source_version: None,
            qualifiers: vec![],
            project_id: "p1".into(),
            kind: ClaimKind::Constraint,
            statement: "cache migration must preserve user data".into(),
            valid_from: Some("2020-01-01".into()),
            valid_until: None,
            source_decision_id: None,
        })
        .unwrap()
        .claim_id
}
fn factory(mode: &'static str, hook: Option<Hook>) -> Factory {
    Factory {
        calls: Arc::new(AtomicUsize::new(0)),
        mode,
        hook,
    }
}
fn routing_job(test: &support::TestStore) -> String {
    Connection::open(test.root.join("app.db"))
        .unwrap()
        .query_row(
            "SELECT owner_job_id FROM context_routing_entries
         WHERE state='pending' ORDER BY created_at DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap()
}

#[test]
fn original28_completed_cache_is_rekeyed_on_actual_lookup_without_another_call() {
    let test = support::open("routing-legacy-key", &["p1"]);
    seed(&test);
    let (settings, _, _) = settings();
    let packs = ContextPacks::new(test.store.clone()).with_routing(Some(Arc::new(
        RoutingLookup::new(Arc::new(test.store.clone()), Arc::new(settings.clone())),
    )));
    packs.build_pack(request()).unwrap();
    let raw = Connection::open(test.root.join("app.db")).unwrap();
    let (key, snapshot, generation, encoded): (String, String, i64, String) = raw
        .query_row(
            "SELECT key,snapshot,generation,request_json FROM context_routing_entries",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .unwrap();
    let legacy =
        application::context_routing::fingerprint(&format!("{snapshot}:{generation}:{encoded}"));
    let f = factory("valid", None);
    JudgeContextAmbiguity::new(test.store.clone(), settings, f.clone())
        .run(&routing_job(&test))
        .unwrap();
    let expected = packs.build_pack(request()).unwrap();
    // Original v28 completed rows have only the generation-based key, no payload.
    raw.execute(
        "UPDATE context_routing_entries SET key=?1,owner_job_id=NULL",
        [&legacy],
    )
    .unwrap();
    raw.execute_batch(
        "DROP INDEX context_routing_owner;
        ALTER TABLE context_routing_entries DROP COLUMN owner_job_id;
        DELETE FROM schema_migrations WHERE version=33;",
    )
    .unwrap();
    let reopened = storage_sqlite::SqliteStore::open(test.root.join("app.db")).unwrap();
    // Change generation with identical clean semantics, as a later recheck does.
    raw.execute(
        "UPDATE observation_refresh SET generation=generation+1,dirty=0",
        [],
    )
    .unwrap();
    let (settings, _, _) = self::settings();
    let upgraded = ContextPacks::new(reopened.clone()).with_routing(Some(Arc::new(
        RoutingLookup::new(Arc::new(reopened), Arc::new(settings)),
    )));
    // Different task cannot borrow this cache or rekey it.
    upgraded
        .build_pack(ContextRequest {
            task: "cache migration different".into(),
            ..request()
        })
        .unwrap();
    let stored: String = raw
        .query_row("SELECT key FROM context_routing_entries", [], |r| r.get(0))
        .unwrap();
    assert_eq!(stored, legacy);
    assert_eq!(upgraded.build_pack(request()).unwrap(), expected);
    let stored: String = raw
        .query_row("SELECT key FROM context_routing_entries", [], |r| r.get(0))
        .unwrap();
    assert_eq!(stored, key);
    assert_eq!(
        count(
            &test,
            "SELECT COUNT(*) FROM jobs WHERE kind='context_routing'"
        ),
        1
    );
    assert_eq!(f.calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        count(
            &test,
            "SELECT SUM(logical_invocations) FROM context_routing_entries"
        ),
        1
    );
}

#[test]
fn identical_refresh_reuses_cache_and_expired_key_renews() {
    let test = support::open("routing-refresh-expire", &["p1"]);
    seed(&test);
    let (settings, _, _) = settings();
    let packs = ContextPacks::new(test.store.clone()).with_routing(Some(Arc::new(
        RoutingLookup::new(Arc::new(test.store.clone()), Arc::new(settings.clone())),
    )));
    packs.build_pack(request()).unwrap();
    let f = factory("valid", None);
    let judge = JudgeContextAmbiguity::new(test.store.clone(), settings, f.clone());
    judge.run(&routing_job(&test)).unwrap();
    let cached = packs.build_pack(request()).unwrap();
    let raw = Connection::open(test.root.join("app.db")).unwrap();
    raw.execute(
        "UPDATE observation_refresh SET generation=generation+1,dirty=1",
        [],
    )
    .unwrap();
    assert_eq!(packs.build_pack(request()).unwrap().decisions.len(), 2);
    raw.execute("UPDATE observation_refresh SET dirty=0", [])
        .unwrap();
    assert_eq!(packs.build_pack(request()).unwrap(), cached);
    assert_eq!(
        count(&test, "SELECT COUNT(*) FROM context_routing_entries"),
        1
    );
    raw.execute(
        "UPDATE context_routing_entries SET expires_at=0,created_at=created_at-61",
        [],
    )
    .unwrap();
    assert_eq!(packs.build_pack(request()).unwrap().decisions.len(), 2);
    assert_eq!(
        count(
            &test,
            "SELECT COUNT(*) FROM context_routing_entries WHERE state='pending'"
        ),
        1
    );
    judge.run(&routing_job(&test)).unwrap();
    assert_eq!(f.calls.load(Ordering::SeqCst), 2);
    assert_eq!(packs.build_pack(request()).unwrap(), cached);
}

#[test]
fn orphan_cleanup_commits_on_early_return_and_job_owns_exact_entry() {
    use application::context_routing::{profile_hash, RoutingStore};
    let test = support::open("routing-owner-cleanup", &["p1"]);
    seed(&test);
    let (settings, profiles, _) = settings();
    let packs = ContextPacks::new(test.store.clone()).with_routing(Some(Arc::new(
        RoutingLookup::new(Arc::new(test.store.clone()), Arc::new(settings)),
    )));
    packs.build_pack(request()).unwrap();
    let a = routing_job(&test);
    let raw = Connection::open(test.root.join("app.db")).unwrap();
    raw.execute(
        "UPDATE context_routing_entries SET created_at=created_at-61",
        [],
    )
    .unwrap();
    packs
        .build_pack(ContextRequest {
            task: "cache migration second".into(),
            ..request()
        })
        .unwrap();
    let b = JobRepository::list(&test.store)
        .unwrap()
        .into_iter()
        .find(|j| j.kind == CONTEXT_ROUTING_KIND && j.id != a)
        .unwrap()
        .id;
    assert_ne!(a, b);
    raw.execute("UPDATE jobs SET state='cancelled' WHERE id=?1", [&a])
        .unwrap();
    // Cooldown return must commit A's terminal erasure, across reopen.
    packs.build_pack(request()).unwrap();
    let reopened = storage_sqlite::SqliteStore::open(test.root.join("app.db")).unwrap();
    assert_eq!(count(&test,"SELECT COUNT(*) FROM context_routing_entries WHERE state='failed' AND request_json IS NULL"),1);
    let hash = profile_hash(&profiles.0.lock().unwrap());
    assert!(reopened.begin(&a, &hash).unwrap().is_none());
    let work = reopened.begin(&b, &hash).unwrap().unwrap();
    assert_eq!(work.request.task, "cache migration second");
    assert!(b.starts_with(&format!("context-routing-{}:", work.key)));
    assert_eq!(
        count(
            &test,
            "SELECT SUM(logical_invocations) FROM context_routing_entries"
        ),
        0
    );
}

fn count(test: &support::TestStore, sql: &str) -> i64 {
    Connection::open(test.root.join("app.db"))
        .unwrap()
        .query_row(sql, [], |r| r.get(0))
        .unwrap()
}

#[test]
fn first_lookup_baseline_then_worker_filters_only_weak_and_erases_content() {
    let test = support::open("routing-baseline", &["p1"]);
    let pinned = seed(&test);
    let (settings, _, secrets) = settings();
    let router = Arc::new(RoutingLookup::new(
        Arc::new(test.store.clone()),
        Arc::new(settings.clone()),
    ));
    let packs = ContextPacks::new(test.store.clone()).with_routing(Some(router));
    let baseline = ContextPacks::new(test.store.clone())
        .build_pack(request())
        .unwrap();
    assert_eq!(packs.build_pack(request()).unwrap(), baseline);
    assert_eq!(
        secrets.load(Ordering::SeqCst),
        0,
        "query never uses keychain"
    );
    assert_eq!(
        count(&test, "SELECT COUNT(*) FROM context_routing_entries"),
        1
    );
    assert_eq!(packs.build_pack(request()).unwrap(), baseline);
    assert_eq!(
        count(
            &test,
            "SELECT COUNT(*) FROM jobs WHERE kind='context_routing'"
        ),
        1
    );
    let factory = factory("valid", None);
    JudgeContextAmbiguity::new(test.store.clone(), settings, factory.clone())
        .run(&routing_job(&test))
        .unwrap();
    let cached = packs.build_pack(request()).unwrap();
    assert_eq!(cached.decisions.len(), baseline.decisions.len() - 1);
    assert!(cached.claims.iter().any(|c| c.claim_id == pinned));
    assert_eq!(cached.observations, baseline.observations);
    assert_eq!(
        cached.used_chars, baseline.used_chars,
        "no budget refill or eviction"
    );
    assert_eq!(factory.calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        count(
            &test,
            "SELECT SUM(logical_invocations) FROM context_routing_entries"
        ),
        1
    );
    assert_eq!(
        count(
            &test,
            "SELECT COUNT(*) FROM context_routing_entries WHERE request_json IS NOT NULL"
        ),
        0
    );
    assert_eq!(
        count(
            &test,
            "SELECT COUNT(*) FROM context_routing_entries WHERE actual_content_attempts IS NULL"
        ),
        1
    );
}

#[test]
fn strict_failures_and_all_irrelevant_fall_back_without_repair() {
    for mode in [
        "malformed",
        "foreign",
        "revision",
        "extra",
        "duplicate",
        "all_irrelevant",
    ] {
        let test = support::open(mode, &["p1"]);
        seed(&test);
        let (settings, _, _) = settings();
        let packs = ContextPacks::new(test.store.clone()).with_routing(Some(Arc::new(
            RoutingLookup::new(Arc::new(test.store.clone()), Arc::new(settings.clone())),
        )));
        let before = packs.build_pack(request()).unwrap();
        let factory = factory(mode, None);
        JudgeContextAmbiguity::new(test.store.clone(), settings, factory.clone())
            .run(&routing_job(&test))
            .unwrap();
        assert_eq!(packs.build_pack(request()).unwrap(), before, "{mode}");
        assert_eq!(factory.calls.load(Ordering::SeqCst), 1);
        assert_eq!(count(&test,"SELECT COUNT(*) FROM context_routing_entries WHERE result_json IS NOT NULL OR request_json IS NOT NULL"),0);
    }
}

#[test]
fn historical_files_empty_strong_and_revoked_do_not_enqueue() {
    let test = support::open("routing-gates", &["p1"]);
    seed(&test);
    let (settings, profiles, secrets) = settings();
    let packs = ContextPacks::new(test.store.clone()).with_routing(Some(Arc::new(
        RoutingLookup::new(Arc::new(test.store.clone()), Arc::new(settings)),
    )));
    for req in [
        ContextRequest {
            as_of: Some("2026-01-01".into()),
            ..request()
        },
        ContextRequest {
            files: vec!["src/lib.rs".into()],
            ..request()
        },
        ContextRequest {
            task: String::new(),
            files: vec!["src/lib.rs".into()],
            ..request()
        },
        ContextRequest {
            task: "Cache response".into(),
            ..request()
        },
    ] {
        packs.build_pack(req).unwrap();
    }
    profiles.0.lock().unwrap().external_calls_enabled = false;
    packs.build_pack(request()).unwrap();
    assert_eq!(
        count(&test, "SELECT COUNT(*) FROM context_routing_entries"),
        0
    );
    assert_eq!(secrets.load(Ordering::SeqCst), 0);
}

#[test]
fn changes_during_provider_block_publication_and_revoke_blocks_consumption() {
    for mutation in ["claim", "new_item", "dirty", "profile", "purge"] {
        let test = support::open(mutation, &["p1"]);
        let pinned = seed(&test);
        let (settings, profiles, _) = settings();
        let packs = ContextPacks::new(test.store.clone()).with_routing(Some(Arc::new(
            RoutingLookup::new(Arc::new(test.store.clone()), Arc::new(settings.clone())),
        )));
        let before = packs.build_pack(request()).unwrap();
        let store = test.store.clone();
        let path = test.root.join("app.db");
        let profiles = profiles.clone();
        let hook: Hook = Arc::new(move || match mutation {
            "claim" => {
                Connection::open(&path)
                    .unwrap()
                    .execute(
                        "UPDATE context_claims SET statement='changed claim' WHERE claim_id=?1",
                        [&pinned],
                    )
                    .unwrap();
            }
            "new_item" => {
                support::decision(
                    &store,
                    "p1",
                    "unrelated",
                    "Unrelated memory?",
                    "A new unrelated fact",
                );
            }
            "dirty" => {
                Connection::open(&path)
                    .unwrap()
                    .execute(
                        "UPDATE observation_refresh SET dirty=1,generation=generation+1",
                        [],
                    )
                    .unwrap();
            }
            "profile" => {
                profiles.0.lock().unwrap().endpoint = Some("http://127.0.0.1:10/v1".into());
            }
            "purge" => {
                application::projects::ProjectRepository::purge(&store, "p1").unwrap();
            }
            _ => unreachable!(),
        });
        let factory = factory("valid", Some(hook));
        JudgeContextAmbiguity::new(test.store.clone(), settings, factory)
            .run(&routing_job(&test))
            .unwrap();
        assert_eq!(count(&test,"SELECT COUNT(*) FROM context_routing_entries WHERE result_json IS NOT NULL OR request_json IS NOT NULL"),0, "{mutation}");
        if mutation == "profile" {
            assert_eq!(packs.build_pack(request()).unwrap(), before);
        }
    }
}

#[test]
fn cached_result_does_not_bypass_revocation_and_reopen_deduplicates() {
    let test = support::open("routing-reopen", &["p1"]);
    seed(&test);
    let (settings, profiles, _) = self::settings();
    let router = Arc::new(RoutingLookup::new(
        Arc::new(test.store.clone()),
        Arc::new(settings.clone()),
    ));
    let packs = ContextPacks::new(test.store.clone()).with_routing(Some(router));
    let before = packs.build_pack(request()).unwrap();
    let reopened = storage_sqlite::SqliteStore::open(test.root.join("app.db")).unwrap();
    let other = ContextPacks::new(reopened.clone()).with_routing(Some(Arc::new(
        RoutingLookup::new(Arc::new(reopened.clone()), Arc::new(settings.clone())),
    )));
    other.build_pack(request()).unwrap();
    assert_eq!(
        count(&test, "SELECT COUNT(*) FROM context_routing_entries"),
        1
    );
    JudgeContextAmbiguity::new(reopened, settings, factory("valid", None))
        .run(&routing_job(&test))
        .unwrap();
    assert_eq!(packs.build_pack(request()).unwrap().decisions.len(), 1);
    profiles.0.lock().unwrap().external_calls_enabled = false;
    assert_eq!(packs.build_pack(request()).unwrap(), before);
}

#[test]
fn query_does_not_wait_for_provider_and_purge_does_not_stop_remote_worker() {
    use std::sync::mpsc;
    use std::time::Duration;
    let test = support::open("routing-barrier", &["p1", "p2"]);
    seed(&test);
    let (settings, _, _) = settings();
    let packs = ContextPacks::new(test.store.clone()).with_routing(Some(Arc::new(
        RoutingLookup::new(Arc::new(test.store.clone()), Arc::new(settings.clone())),
    )));
    let before = packs.build_pack(request()).unwrap();
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let release = Mutex::new(release_rx);
    let hook: Hook = Arc::new(move || {
        entered_tx.send(()).unwrap();
        release
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(10))
            .unwrap();
    });
    let judge =
        JudgeContextAmbiguity::new(test.store.clone(), settings, factory("valid", Some(hook)));
    let mut jobs = Jobs::new(test.store.clone());
    jobs.register(
        CONTEXT_ROUTING_KIND,
        Arc::new(move |job| {
            judge
                .run(&job.id)
                .map_err(|_| application::jobs::JobFailure::Failed)
        }),
    );
    let (events_tx, events_rx) = mpsc::channel();
    jobs.observe_with(move |event| {
        events_tx.send(event).unwrap();
    });
    let worker = jobs.spawn_workers(1);
    entered_rx.recv_timeout(Duration::from_secs(10)).unwrap();
    // This synchronous query completes while provider work is still held at the barrier.
    assert_eq!(packs.build_pack(request()).unwrap(), before);
    application::projects::ProjectRepository::purge(&test.store, "p1").unwrap();
    jobs.enqueue(CONTEXT_ROUTING_KIND, "p2", false).unwrap();
    release_tx.send(()).unwrap();
    let first = events_rx.recv_timeout(Duration::from_secs(10)).unwrap();
    let second = events_rx.recv_timeout(Duration::from_secs(10)).unwrap();
    worker.stop();
    worker.join().unwrap();
    assert!(
        matches!(first,application::jobs::JobEvent::Finished(o) if o.state==application::jobs::JobState::Cancelled)
    );
    assert!(
        matches!(second,application::jobs::JobEvent::Finished(o) if o.state==application::jobs::JobState::Completed)
    );
    assert_eq!(
        count(&test, "SELECT COUNT(*) FROM context_routing_entries"),
        0
    );
    assert!(JobRepository::list(&test.store)
        .unwrap()
        .iter()
        .all(|j| j.payload != "p1"));
}

#[test]
fn pending_daily_cooldown_and_input_limits_are_persisted() {
    let test = support::open("routing-quotas", &["p1"]);
    seed(&test);
    let (settings, _, _) = settings();
    let packs = ContextPacks::new(test.store.clone()).with_routing(Some(Arc::new(
        RoutingLookup::new(Arc::new(test.store.clone()), Arc::new(settings.clone())),
    )));
    packs.build_pack(request()).unwrap();
    packs
        .build_pack(ContextRequest {
            task: "cache migration another".into(),
            ..request()
        })
        .unwrap();
    assert_eq!(
        count(&test, "SELECT COUNT(*) FROM context_routing_entries"),
        1,
        "cooldown"
    );
    let raw = Connection::open(test.root.join("app.db")).unwrap();
    raw.execute(
        "UPDATE context_routing_entries SET created_at=created_at-61",
        [],
    )
    .unwrap();
    packs
        .build_pack(ContextRequest {
            task: "cache migration another".into(),
            ..request()
        })
        .unwrap();
    assert_eq!(
        count(&test, "SELECT COUNT(*) FROM context_routing_entries"),
        2
    );
    raw.execute(
        "UPDATE context_routing_entries SET created_at=created_at-61",
        [],
    )
    .unwrap();
    packs
        .build_pack(ContextRequest {
            task: "cache migration third".into(),
            ..request()
        })
        .unwrap();
    assert_eq!(
        count(&test, "SELECT COUNT(*) FROM context_routing_entries"),
        2,
        "pending cap"
    );
    raw.execute(
        "UPDATE context_routing_entries SET state='failed',request_json=NULL,logical_invocations=1",
        [],
    )
    .unwrap();
    for index in 0..6 {
        raw.execute(
            "INSERT INTO context_routing_entries (key,project_id,snapshot,generation,
            profile_hash,state,logical_invocations,created_at,expires_at) VALUES (?1,'p1','x',0,'x','failed',1,
            CAST(strftime('%s','now') AS INTEGER)-120,CAST(strftime('%s','now') AS INTEGER)+3600)",
            [format!("quota-{index}")],
        )
        .unwrap();
    }
    packs
        .build_pack(ContextRequest {
            task: "cache migration fourth".into(),
            ..request()
        })
        .unwrap();
    assert_eq!(
        count(&test, "SELECT COUNT(*) FROM context_routing_entries"),
        8,
        "daily cap"
    );
    let fresh = support::open("routing-input-cap", &["p1"]);
    seed(&fresh);
    let (settings, profiles, _) = self::settings();
    let mut p = profiles.0.lock().unwrap();
    p.max_input_chars = 100;
    *p = profile::grant_consent(
        &p,
        &profile::build_preview(&p),
        "2026-01-01T00:00:00Z",
        true,
    )
    .unwrap();
    drop(p);
    let packs = ContextPacks::new(fresh.store.clone()).with_routing(Some(Arc::new(
        RoutingLookup::new(Arc::new(fresh.store.clone()), Arc::new(settings)),
    )));
    let baseline = ContextPacks::new(fresh.store.clone())
        .build_pack(request())
        .unwrap();
    assert_eq!(packs.build_pack(request()).unwrap(), baseline);
    assert_eq!(
        count(&fresh, "SELECT COUNT(*) FROM context_routing_entries"),
        0,
        "no partial qualifiers"
    );
}

#[test]
fn cached_claim_changes_new_items_and_dirty_generation_invalidate() {
    for mutation in ["claim", "new_item", "dirty"] {
        let test = support::open("routing-cache-invalidation", &["p1"]);
        seed(&test);
        let (settings, _, _) = settings();
        let packs = ContextPacks::new(test.store.clone()).with_routing(Some(Arc::new(
            RoutingLookup::new(Arc::new(test.store.clone()), Arc::new(settings.clone())),
        )));
        let before = packs.build_pack(request()).unwrap();
        JudgeContextAmbiguity::new(test.store.clone(), settings, factory("valid", None))
            .run(&routing_job(&test))
            .unwrap();
        assert_eq!(packs.build_pack(request()).unwrap().decisions.len(), 1);
        let raw = Connection::open(test.root.join("app.db")).unwrap();
        match mutation {
            "claim" => {
                raw.execute(
                    "UPDATE context_claims SET updated_at='2099-01-01T00:00:00Z'",
                    [],
                )
                .unwrap();
            }
            "new_item" => {
                support::decision(
                    &test.store,
                    "p1",
                    "outside-shortlist",
                    "Unrelated?",
                    "New unrelated memory",
                );
            }
            "dirty" => {
                raw.execute(
                    "UPDATE observation_refresh SET generation=generation+1,dirty=1",
                    [],
                )
                .unwrap();
            }
            _ => unreachable!(),
        }
        assert_eq!(packs.build_pack(request()).unwrap(), before, "{mutation}");
    }
}
