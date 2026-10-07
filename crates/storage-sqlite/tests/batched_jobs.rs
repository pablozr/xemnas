//! Performance gate of the AI job queue: N adopted decisions cost ceil(N / 10)
//! provider calls per job kind, not N, and a batch settles every job it took.
//!
//! The model is a fake that counts calls and the subjects in each; the
//! decisions, jobs, validation and storage are the real ones.

mod support;

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use application::analysis::ExtractorFactory;
use application::batching::BATCH_SIZE;
use application::claim_suggestions::{ClaimFinder, CLAIM_JOB_KIND};
use application::extract::{
    CandidateExtractor, CandidateProposal, DecisionEvidence, ExtractError, RelevanceSignal,
};
use application::graph::{GraphStore, KnowledgeGraph, NewEntity};
use application::jobs::{JobFailure, JobRecord, JobRepository, JobState, Jobs};
use application::link_suggestions::{LinkFinder, LINK_JOB_KIND};
use application::overview::{JsonValue, StructuredModel};
use application::profile::{
    build_preview, grant_consent, offline_default_profile, AiProfile, AiSettings, ProfileError,
    ProfileKind, ProfileStore, SecretStore,
};
use application::relation_suggestions::{RelationFinder, RELATION_JOB_KIND};
use application::search_terms::{SearchTermFinder, MAX_COALESCED_JOBS, SEARCH_TERMS_JOB_KIND};
use domain::entities::EntityKind;
use storage_sqlite::SqliteStore;

struct Profiles(Mutex<AiProfile>);

impl ProfileStore for Profiles {
    fn load(&self) -> Result<Option<AiProfile>, ProfileError> {
        Ok(Some(self.0.lock().expect("profile").clone()))
    }
    fn save(&self, profile: &AiProfile) -> Result<(), ProfileError> {
        *self.0.lock().expect("profile") = profile.clone();
        Ok(())
    }
}

struct NoSecrets;

impl SecretStore for NoSecrets {
    fn set_secret(&self, _: &str, _: &str) -> Result<(), ProfileError> {
        Ok(())
    }
    fn get_secret(&self, _: &str) -> Result<Option<String>, ProfileError> {
        Ok(None)
    }
    fn delete_secret(&self, _: &str) -> Result<(), ProfileError> {
        Ok(())
    }
}

fn settings() -> AiSettings<Profiles, NoSecrets> {
    let profile = AiProfile {
        kind: ProfileKind::OpenAiCompatible,
        model: "local".into(),
        endpoint: Some("http://127.0.0.1:9/v1".into()),
        ..offline_default_profile()
    };
    let profile = grant_consent(
        &profile,
        &build_preview(&profile),
        "2026-01-01T00:00:00Z",
        true,
    )
    .expect("consent");
    AiSettings::new(Profiles(Mutex::new(profile)), NoSecrets)
}

/// Calls per schema name, with the subjects each carried.
#[derive(Default)]
struct Calls(Mutex<BTreeMap<String, Vec<usize>>>);

impl Calls {
    fn of(&self, schema: &str) -> Vec<usize> {
        self.0
            .lock()
            .expect("calls")
            .get(schema)
            .cloned()
            .unwrap_or_default()
    }
}

/// How the fake answers: well, or one subject malformed and one skipped, or
/// with a request to slow down.
#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Valid,
    OneMalformedOneSkipped,
    RateLimited,
}

#[derive(Clone)]
struct Model {
    calls: Arc<Calls>,
    mode: Mode,
}

impl CandidateExtractor for Model {
    fn extract(
        &self,
        _: &DecisionEvidence,
        _: &[RelevanceSignal],
    ) -> Result<Vec<CandidateProposal>, ExtractError> {
        Ok(Vec::new())
    }
}

/// Numbers of the subjects of a request, by the line each one starts with.
fn numbers(user: &str, prefix: &str) -> Vec<usize> {
    user.lines()
        .filter_map(|line| line.strip_prefix(prefix))
        .filter_map(|rest| rest.split_whitespace().next()?.parse().ok())
        .collect()
}

/// The `Choice:` line of subject `number` in a `## Decision n` request.
fn choice_of(user: &str, number: usize) -> String {
    let header = format!("## Decision {number}\n");
    let from = user.find(&header).expect("subject") + header.len();
    user[from..]
        .lines()
        .find_map(|line| line.strip_prefix("Choice: "))
        .expect("choice")
        .to_string()
}

impl StructuredModel for Model {
    fn complete(
        &self,
        _: &str,
        user: &str,
        schema_name: &str,
        _: &JsonValue,
    ) -> Result<String, ExtractError> {
        let (prefix, field) = match schema_name {
            "decision_links" => ("## Decision ", "links"),
            "decision_context" => ("## Decision ", "claims"),
            "decision_relations" => ("## New decision ", "relations"),
            "decision_search_terms" => ("id: ", "terms"),
            other => panic!("unexpected schema {other}"),
        };
        let subjects = numbers(user, prefix);
        self.calls
            .0
            .lock()
            .expect("calls")
            .entry(schema_name.to_string())
            .or_default()
            .push(subjects.len());
        if self.mode == Mode::RateLimited {
            return Err(ExtractError::RateLimited { retry_after: None });
        }
        let entries: Vec<serde_json::Value> = subjects
            .iter()
            .filter_map(|number| {
                let items = match (schema_name, self.mode, *number) {
                    (_, Mode::OneMalformedOneSkipped, 2) => {
                        return Some(serde_json::json!({"id": "2", field: "not a list"}));
                    }
                    (_, Mode::OneMalformedOneSkipped, 3) => return None,
                    // Links: one valid link to the only component per subject.
                    ("decision_links", _, number) => serde_json::json!([{
                        "component_id": "c1",
                        "quote": choice_of(user, number),
                        "reason": "A decisão rege o componente."
                    }]),
                    _ => serde_json::json!([]),
                };
                Some(serde_json::json!({"id": number.to_string(), field: items}))
            })
            .collect();
        Ok(serde_json::json!({ "decisions": entries }).to_string())
    }
}

#[derive(Clone)]
struct Factory(Model);

impl ExtractorFactory for Factory {
    type Extractor = Model;
    fn external(&self, _: &AiProfile, _: String) -> Result<Model, ExtractError> {
        Ok(self.0.clone())
    }
}

fn job(kind: &str, payload: &str, created_at: &str) -> JobRecord {
    JobRecord {
        id: format!("{kind}-{payload}-{created_at}"),
        kind: kind.into(),
        payload: payload.into(),
        state: JobState::Queued,
        idempotent: true,
        attempts: 0,
        last_error: None,
        created_at: created_at.into(),
        updated_at: created_at.into(),
    }
}

/// The jobs of the four suggestion kinds, registered like the app does.
fn suggestion_jobs(store: &SqliteStore, model: Model) -> Jobs<SqliteStore> {
    fn results<E>(
        results: Vec<Result<usize, E>>,
        deferral: impl Fn(&E) -> Option<Option<std::time::Duration>>,
    ) -> Vec<Result<(), JobFailure>> {
        results
            .into_iter()
            .map(|result| match result.as_ref().err().and_then(&deferral) {
                Some(retry_after) => Err(JobFailure::Deferred { retry_after }),
                None => Ok(()),
            })
            .collect()
    }
    let mut jobs = Jobs::new(store.clone());
    let finder = LinkFinder::new(store.clone(), settings(), Factory(model.clone()));
    jobs.register_batch(
        LINK_JOB_KIND,
        BATCH_SIZE,
        Arc::new(move |records: &[JobRecord]| {
            let ids: Vec<&str> = records.iter().map(|job| job.payload.as_str()).collect();
            results(finder.run_many(&ids), |error| match error {
                application::link_suggestions::LinkFindError::Deferred(after) => Some(*after),
                _ => None,
            })
        }),
    );
    let finder = RelationFinder::new(store.clone(), settings(), Factory(model.clone()));
    jobs.register_batch(
        RELATION_JOB_KIND,
        BATCH_SIZE,
        Arc::new(move |records: &[JobRecord]| {
            let ids: Vec<&str> = records.iter().map(|job| job.payload.as_str()).collect();
            results(finder.run_many(&ids), |error| match error {
                application::relation_suggestions::RelationFindError::Deferred(after) => {
                    Some(*after)
                }
                _ => None,
            })
        }),
    );
    let finder = ClaimFinder::new(store.clone(), settings(), Factory(model.clone()));
    jobs.register_batch(
        CLAIM_JOB_KIND,
        BATCH_SIZE,
        Arc::new(move |records: &[JobRecord]| {
            let ids: Vec<&str> = records.iter().map(|job| job.payload.as_str()).collect();
            results(finder.run_many(&ids), |error| match error {
                application::claim_suggestions::ClaimSuggestionError::Deferred(after) => {
                    Some(*after)
                }
                _ => None,
            })
        }),
    );
    let finder = SearchTermFinder::new(store.clone(), settings(), Factory(model));
    jobs.register_batch(
        SEARCH_TERMS_JOB_KIND,
        MAX_COALESCED_JOBS,
        Arc::new(move |records: &[JobRecord]| {
            let projects: Vec<&str> = records.iter().map(|job| job.payload.as_str()).collect();
            results(finder.run_projects(&projects), |error| match error {
                application::search_terms::SearchTermError::Deferred(after) => Some(*after),
                _ => None,
            })
        }),
    );
    jobs
}

/// `count` accepted decisions of project `p1` that share words (so each has
/// earlier decisions to be compared with) and no tie to the map, a component
/// to link them to, and the four jobs the adoption queues for each.
fn adopted(count: usize, tag: &str) -> (support::TestStore, Vec<String>) {
    let test = support::open(tag, &["p1"]);
    KnowledgeGraph::new(test.store.clone())
        .create_entity(NewEntity {
            project_id: "p1".into(),
            kind: Some(EntityKind::Component),
            name: "queue".into(),
            patterns: vec!["crates/queue/**".into()],
            ..NewEntity::default()
        })
        .expect("component");
    let ids: Vec<String> = (0..count)
        .map(|n| {
            support::decision(
                &test.store,
                "p1",
                &format!("d{n}"),
                &format!("Como tratar a fila compartilhada número {n}?"),
                &format!("Usar a fila compartilhada com prioridade {n}"),
            )
        })
        .collect();
    for (n, id) in ids.iter().enumerate() {
        let at = format!("2026-02-01T00:00:{n:02}Z");
        for kind in [RELATION_JOB_KIND, CLAIM_JOB_KIND, LINK_JOB_KIND] {
            test.store.insert(&job(kind, id, &at)).expect("queue");
        }
        test.store
            .insert(&job(SEARCH_TERMS_JOB_KIND, "p1", &at))
            .expect("queue terms");
    }
    (test, ids)
}

/// Runs batches until nothing is claimable; the outcome of every job.
fn drain(jobs: &Jobs<SqliteStore>) -> Vec<(String, JobState)> {
    let mut ran = Vec::new();
    loop {
        let group = jobs.run_next_group().expect("run");
        if group.is_empty() {
            return ran;
        }
        ran.extend(
            group
                .into_iter()
                .map(|outcome| (outcome.kind, outcome.state)),
        );
    }
}

#[test]
fn n_decisions_cost_ceil_n_over_ten_calls_per_kind() {
    const N: usize = 23;
    let (test, ids) = adopted(N, "batched-jobs-gate");
    let calls = Arc::new(Calls::default());
    let jobs = suggestion_jobs(
        &test.store,
        Model {
            calls: calls.clone(),
            mode: Mode::Valid,
        },
    );

    let ran = drain(&jobs);

    let expected = N.div_ceil(BATCH_SIZE);
    for schema in [
        "decision_links",
        "decision_relations",
        "decision_context",
        "decision_search_terms",
    ] {
        let per_call = calls.of(schema);
        assert_eq!(
            per_call.len(),
            expected,
            "{schema}: {N} decisions in {per_call:?} subjects per call"
        );
        assert!(per_call.iter().all(|subjects| *subjects <= BATCH_SIZE));
        assert_eq!(
            per_call.iter().sum::<usize>(),
            N,
            "{schema}: every subject asked"
        );
    }
    println!(
        "gate batched jobs: {N} decisions -> links {}, relations {}, rules {}, terms {} calls \
         (one call per decision would be {N} each)",
        calls.of("decision_links").len(),
        calls.of("decision_relations").len(),
        calls.of("decision_context").len(),
        calls.of("decision_search_terms").len(),
    );
    // Every queued job reached a final state; none was lost or left running.
    let count = |kind: &str| ran.iter().filter(|(each, _)| each == kind).count();
    for kind in [RELATION_JOB_KIND, CLAIM_JOB_KIND, LINK_JOB_KIND] {
        assert_eq!(count(kind), N, "{kind}");
    }
    assert!(
        ran.iter().all(|(_, state)| *state == JobState::Completed),
        "{ran:?}"
    );
    let open = test
        .store
        .list()
        .expect("jobs")
        .into_iter()
        .filter(|job| matches!(job.state, JobState::Queued | JobState::Running))
        .filter(|job| job.kind != "analyze_capture" && job.kind != "refresh_observations")
        .collect::<Vec<_>>();
    assert!(open.is_empty(), "left behind: {open:?}");
    // Stored as before: one pending link per decision.
    let edges = test.store.project_edges("p1").expect("edges");
    for id in &ids {
        assert_eq!(
            edges.iter().filter(|edge| edge.source_id == *id).count(),
            1,
            "a link for {id}"
        );
    }
}

#[test]
fn a_malformed_or_skipped_subject_loses_only_itself() {
    let (test, ids) = adopted(3, "batched-jobs-malformed");
    // Only the link jobs: the other kinds are not what this checks.
    for job in test.store.list().expect("jobs") {
        if job.kind != LINK_JOB_KIND && job.kind != "analyze_capture" {
            test.store
                .transition(&job.id, JobState::Queued, JobState::Cancelled, None)
                .expect("cancel");
        }
    }
    let calls = Arc::new(Calls::default());
    let jobs = suggestion_jobs(
        &test.store,
        Model {
            calls: calls.clone(),
            mode: Mode::OneMalformedOneSkipped,
        },
    );

    let group = jobs.run_next_group().expect("run");

    assert_eq!(calls.of("decision_links"), [3], "one call for the three");
    assert_eq!(group.len(), 3);
    assert!(group
        .iter()
        .all(|outcome| outcome.state == JobState::Completed));
    let edges = test.store.project_edges("p1").expect("edges");
    let linked: Vec<&str> = edges.iter().map(|edge| edge.source_id.as_str()).collect();
    assert_eq!(
        linked,
        [ids[0].as_str()],
        "only the readable subject got a link"
    );
}

#[test]
fn a_provider_pause_defers_every_job_of_the_batch() {
    let (test, _) = adopted(3, "batched-jobs-paused");
    let calls = Arc::new(Calls::default());
    let jobs = suggestion_jobs(
        &test.store,
        Model {
            calls: calls.clone(),
            mode: Mode::RateLimited,
        },
    );

    let group = jobs.run_next_group().expect("run");

    // Links go first (priority), the three of them in one call.
    assert_eq!(calls.of("decision_links"), [3]);
    assert_eq!(group.len(), 3);
    for outcome in &group {
        assert_eq!(outcome.kind, LINK_JOB_KIND);
        assert_eq!(
            outcome.state,
            JobState::Queued,
            "back to the queue, not failed"
        );
        assert_eq!(outcome.attempts, 1);
    }
    let stored = test.store.list().expect("jobs");
    let deferred = stored
        .iter()
        .filter(|job| job.kind == LINK_JOB_KIND && job.state == JobState::Queued)
        .count();
    assert_eq!(deferred, 3);
}
