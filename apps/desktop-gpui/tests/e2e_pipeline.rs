//! Explicitly invoked end-to-end run of the real pipeline over a real project.
//!
//! Starts from an empty data directory (never the user's), wires the same job
//! handlers as the desktop composition root, and writes a report of every
//! step: counts, timings, failures with their full messages, candidates, what
//! the automatic review did, rules and links, and the context packs of some
//! sample tasks. See `docs/operacao/avaliacoes.md` ("Execução ponta a ponta").
//!
//! Environment:
//! - `XEMNAS_E2E_PROJECT`: project folder to register (required).
//! - `XEMNAS_E2E_IMPORT`: one documentation file to import (optional).
//! - `XEMNAS_E2E_TASKS`: sample tasks, separated by `;` (optional).
//! - `XEMNAS_E2E_OUT`: data and report directory (default: a fresh temp dir).
//!
//! The AI profile is copied, read only, from the user's profile; the secret
//! comes from the same OS key vault the app uses.

use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use application::auto_approval::{ApprovalsApi, Mode};
use application::context::{ContextProvider, ContextRequest};
use application::jobs::{JobFailure, JobRecord, JobState, Jobs};
use application::observations::{ObservationStore, RefreshRequest};
use storage_sqlite::SqliteStore;

type Settings = application::profile::AiSettings<
    application::profile::FileProfileStore,
    ai_provider::KeyringSecretStore,
>;

/// Longest the runner waits for deferred jobs to come due.
const DRAIN_LIMIT: Duration = Duration::from_secs(15 * 60);

/// Writes each line to stdout and to the report file as it happens.
struct Report {
    file: std::fs::File,
}

impl Report {
    fn line(&mut self, text: impl AsRef<str>) {
        println!("{}", text.as_ref());
        let _ = writeln!(self.file, "{}", text.as_ref());
    }
}

/// Failures the handlers saw, with the full text the job log would drop.
type Failures = Arc<Mutex<Vec<String>>>;

fn clip(text: &str, max: usize) -> String {
    let flat = text.replace(['\r', '\n'], " ");
    match flat.char_indices().nth(max) {
        Some((end, _)) => format!("{}...", &flat[..end]),
        None => flat,
    }
}

fn settings(profile: &Path) -> Settings {
    application::profile::AiSettings::new(
        application::profile::FileProfileStore::new(profile),
        ai_provider::KeyringSecretStore::new(),
    )
}

/// The handlers of `apps/desktop-gpui/src/main.rs`, with failures recorded.
fn provider_jobs(
    store: &SqliteStore,
    profile: &Path,
    failures: &Failures,
    limiter: &application::limiter::ProviderLimiter,
) -> Jobs<SqliteStore> {
    let mut jobs = Jobs::new(store.clone());
    let chatgpt = Arc::new(ai_provider::ChatGptSession::default());
    let factory =
        || ai_provider::ProviderFactory::new(chatgpt.clone()).with_limiter(limiter.clone());
    let note = |failures: &Failures, operation: &str, error: String| {
        failures
            .lock()
            .expect("failures")
            .push(format!("{operation}: {error}"));
    };

    let analyze = Arc::new({
        let analysis =
            application::analysis::AnalyzeCapture::new(store.clone(), settings(profile), factory());
        let failures = failures.clone();
        move |record: &JobRecord| {
            use application::analysis::AnalysisOutcome;
            match analysis.run(&record.payload, Some(record.id.clone())) {
                Ok(AnalysisOutcome::Extracted(_)) | Ok(AnalysisOutcome::Skipped) => Ok(()),
                Ok(AnalysisOutcome::SetupFailed(error)) => {
                    failures.lock().expect("failures").push(format!(
                        "{} setup: {}",
                        record.kind,
                        error.code()
                    ));
                    Err(JobFailure::Failed)
                }
                Err(error) => {
                    failures
                        .lock()
                        .expect("failures")
                        .push(format!("{} {}: {error}", record.kind, record.payload));
                    Err(error.job_failure())
                }
            }
        }
    });
    jobs.register(application::jobs::ANALYZE_CAPTURE_KIND, analyze.clone());
    jobs.register(application::jobs::ANALYZE_DOCUMENT_KIND, analyze);

    // Like the app, the four suggestion kinds batch their queued jobs.
    let relations = application::relation_suggestions::RelationFinder::new(
        store.clone(),
        settings(profile),
        factory(),
    );
    let failed = failures.clone();
    jobs.register_batch(
        application::relation_suggestions::RELATION_JOB_KIND,
        application::batching::BATCH_SIZE,
        Arc::new(move |records: &[JobRecord]| {
            let ids: Vec<&str> = records.iter().map(|job| job.payload.as_str()).collect();
            relations
                .run_many(&ids)
                .into_iter()
                .map(|result| match result {
                    Ok(_) => Ok(()),
                    Err(application::relation_suggestions::RelationFindError::Deferred(
                        retry_after,
                    )) => Err(JobFailure::Deferred { retry_after }),
                    Err(error) => {
                        note(&failed, "suggest_relations", error.to_string());
                        Ok(())
                    }
                })
                .collect()
        }),
    );

    let claims = application::claim_suggestions::ClaimFinder::new(
        store.clone(),
        settings(profile),
        factory(),
    );
    let failed = failures.clone();
    jobs.register_batch(
        application::claim_suggestions::CLAIM_JOB_KIND,
        application::batching::BATCH_SIZE,
        Arc::new(move |records: &[JobRecord]| {
            let ids: Vec<&str> = records.iter().map(|job| job.payload.as_str()).collect();
            claims
                .run_many(&ids)
                .into_iter()
                .map(|result| match result {
                    Ok(_) => Ok(()),
                    Err(application::claim_suggestions::ClaimSuggestionError::Deferred(
                        retry_after,
                    )) => Err(JobFailure::Deferred { retry_after }),
                    Err(error) => {
                        failed
                            .lock()
                            .expect("failures")
                            .push(format!("derive_claims: {error}"));
                        Ok(())
                    }
                })
                .collect()
        }),
    );

    let terms = application::search_terms::SearchTermFinder::new(
        store.clone(),
        settings(profile),
        factory(),
    );
    let failed = failures.clone();
    jobs.register_batch(
        application::search_terms::SEARCH_TERMS_JOB_KIND,
        application::search_terms::MAX_COALESCED_JOBS,
        Arc::new(move |records: &[JobRecord]| {
            let projects: Vec<&str> = records.iter().map(|job| job.payload.as_str()).collect();
            terms
                .run_projects(&projects)
                .into_iter()
                .map(|result| match result {
                    Ok(_) => Ok(()),
                    Err(application::search_terms::SearchTermError::Deferred(retry_after)) => {
                        Err(JobFailure::Deferred { retry_after })
                    }
                    Err(error) => {
                        failed
                            .lock()
                            .expect("failures")
                            .push(format!("derive_search_terms: {error}"));
                        Ok(())
                    }
                })
                .collect()
        }),
    );

    let links =
        application::link_suggestions::LinkFinder::new(store.clone(), settings(profile), factory());
    let failed = failures.clone();
    jobs.register_batch(
        application::link_suggestions::LINK_JOB_KIND,
        application::batching::BATCH_SIZE,
        Arc::new(move |records: &[JobRecord]| {
            let ids: Vec<&str> = records.iter().map(|job| job.payload.as_str()).collect();
            links
                .run_many(&ids)
                .into_iter()
                .map(|result| match result {
                    Ok(_) => Ok(()),
                    Err(application::link_suggestions::LinkFindError::Deferred(retry_after)) => {
                        Err(JobFailure::Deferred { retry_after })
                    }
                    Err(error) => {
                        failed
                            .lock()
                            .expect("failures")
                            .push(format!("suggest_links: {error}"));
                        Ok(())
                    }
                })
                .collect()
        }),
    );
    jobs
}

/// The local observation queue of the desktop.
fn observation_jobs(store: &SqliteStore) -> Jobs<SqliteStore> {
    let mut jobs = Jobs::new(store.clone());
    let refresh = application::observations::refresh::RefreshObservations::new(
        store.clone(),
        application::observations::reader::LocalObservationReader,
    );
    jobs.register(
        application::observations::refresh::REFRESH_OBSERVATIONS_KIND,
        Arc::new(move |record| refresh.run(record)),
    );
    jobs
}

/// Runs jobs until none is left, waiting for deferred ones to come due.
fn drain(
    report: &mut Report,
    label: &str,
    jobs: &Jobs<SqliteStore>,
    failures: &Failures,
    limiter: &application::limiter::ProviderLimiter,
) {
    let started = Instant::now();
    let calls_before = limiter.calls();
    let mut ran: Vec<(String, String)> = Vec::new();
    loop {
        // A batch settles several jobs at once; each one counts as a job run.
        loop {
            let group = jobs.run_next_group().expect("run job");
            if group.is_empty() {
                break;
            }
            for outcome in group {
                ran.push((outcome.kind, outcome.state.as_str().to_string()));
            }
        }
        let waiting = jobs
            .list()
            .expect("list jobs")
            .iter()
            .any(|job| job.state == JobState::Queued || job.state == JobState::Running);
        if !waiting || started.elapsed() > DRAIN_LIMIT {
            break;
        }
        std::thread::sleep(Duration::from_secs(3));
    }
    let mut summary: std::collections::BTreeMap<String, usize> = Default::default();
    for (kind, state) in &ran {
        *summary.entry(format!("{kind}:{state}")).or_default() += 1;
    }
    report.line(format!(
        "[{label}] {} job run(s), {} provider call(s) in {:.1}s: {summary:?}",
        ran.len(),
        limiter.calls() - calls_before,
        started.elapsed().as_secs_f64()
    ));
    for failure in failures.lock().expect("failures").drain(..) {
        report.line(format!("  FAILURE {}", clip(&failure, 1500)));
    }
}

/// Dumps rows of a query as `column=value` pairs, values clipped.
fn dump(report: &mut Report, connection: &rusqlite::Connection, title: &str, sql: &str) {
    let mut statement = match connection.prepare(sql) {
        Ok(statement) => statement,
        Err(error) => {
            report.line(format!("[{title}] query error: {error}"));
            return;
        }
    };
    let names: Vec<String> = statement
        .column_names()
        .iter()
        .map(|name| name.to_string())
        .collect();
    let rows = statement
        .query_map([], |row| {
            let mut text = String::new();
            for (index, name) in names.iter().enumerate() {
                let value: rusqlite::types::Value = row.get(index)?;
                let value = match value {
                    rusqlite::types::Value::Null => "null".to_string(),
                    rusqlite::types::Value::Integer(value) => value.to_string(),
                    rusqlite::types::Value::Real(value) => format!("{value:.2}"),
                    rusqlite::types::Value::Text(value) => clip(&value, 220),
                    rusqlite::types::Value::Blob(_) => "<blob>".to_string(),
                };
                let _ = write!(text, " {name}={value}");
            }
            Ok(text)
        })
        .expect("query")
        .filter_map(Result::ok)
        .collect::<Vec<_>>();
    report.line(format!("[{title}] {} row(s)", rows.len()));
    for row in rows {
        report.line(format!(" {row}"));
    }
}

fn snapshot(report: &mut Report, database: &Path, project_id: &str) {
    let connection =
        rusqlite::Connection::open_with_flags(database, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .expect("read-only connection");
    let by = |sql: &str| sql.replace("{p}", &format!("'{project_id}'"));
    report.line("");
    report.line("== Database snapshot ==");
    dump(
        report,
        &connection,
        "documents",
        &by("SELECT path, kind, bytes FROM project_documents WHERE project_id = {p} ORDER BY path"),
    );
    dump(
        report,
        &connection,
        "assessments by outcome",
        &by(
            "SELECT outcome, error_code, reason, COUNT(*) AS n, SUM(durable_count) AS durable, \
         SUM(detail_count) AS detail FROM assessments GROUP BY outcome, error_code, reason",
        ),
    );
    dump(
        report,
        &connection,
        "jobs by kind and state",
        &by(
            "SELECT kind, state, COUNT(*) AS n, SUM(attempts) AS attempts FROM jobs \
         GROUP BY kind, state ORDER BY kind",
        ),
    );
    dump(
        report,
        &connection,
        "failed jobs",
        &by("SELECT kind, attempts, last_error FROM jobs WHERE state = 'failed'"),
    );
    dump(report, &connection, "candidates by status", &by(
        "SELECT status, COUNT(*) AS n FROM decision_candidates WHERE project_id = {p} GROUP BY status",
    ));
    dump(
        report,
        &connection,
        "candidates",
        &by(
            "SELECT c.status, c.kind, c.confidence, c.question, c.choice, c.qualifiers \
         FROM decision_candidates c WHERE c.project_id = {p} ORDER BY c.created_at",
        ),
    );
    dump(
        report,
        &connection,
        "auto review ledger",
        &by(
            "SELECT item_kind, verdict, decided_by, reason, title FROM auto_reviews \
         WHERE project_id = {p} ORDER BY created_at",
        ),
    );
    dump(report, &connection, "auto review calls", &by(
        "SELECT COUNT(*) AS calls, SUM(succeeded) AS ok, SUM(items) AS items FROM auto_review_calls",
    ));
    dump(
        report,
        &connection,
        "decisions",
        &by(
            "SELECT decision_id, question, choice, scope FROM engineering_decisions \
         WHERE project_id = {p}",
        ),
    );
    dump(
        report,
        &connection,
        "rules (claims)",
        &by(
            "SELECT claim_id, kind, statement, inherited_scope, source_decision_id, valid_until \
         FROM context_claims WHERE project_id = {p} ORDER BY created_at",
        ),
    );
    dump(report, &connection, "entities", &by(
        "SELECT kind, name FROM entities WHERE project_id = {p} AND retired_at IS NULL ORDER BY kind, name",
    ));
    dump(
        report,
        &connection,
        "links (entity edges)",
        &by(
            "SELECT e.kind, e.source_kind, e.source_id, n.name AS entity, e.origin, \
         e.confirmed_at IS NOT NULL AS confirmed, e.reason \
         FROM entity_edges e JOIN entities n ON n.entity_id = e.entity_id \
         WHERE e.project_id = {p} AND e.invalidated_at IS NULL",
        ),
    );
    dump(
        report,
        &connection,
        "relation suggestions",
        &by("SELECT kind, outcome, reason FROM relation_suggestions WHERE project_id = {p}"),
    );
    dump(
        report,
        &connection,
        "claim suggestions",
        &by("SELECT kind, outcome, statement FROM claim_suggestions WHERE project_id = {p}"),
    );
    dump(
        report,
        &connection,
        "observations",
        &by("SELECT COUNT(*) AS records FROM observation_records"),
    );
}

#[test]
#[ignore = "calls the live provider; requires explicit user authorization"]
fn run_the_pipeline_over_a_real_project() {
    let project_dir =
        std::env::var("XEMNAS_E2E_PROJECT").expect("XEMNAS_E2E_PROJECT: the project folder");
    let out = std::env::var_os("XEMNAS_E2E_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::temp_dir().join(format!(
                "xemnas-e2e-{}",
                chrono::Utc::now().format("%Y%m%d-%H%M%S")
            ))
        });
    let paths = application::AppPaths::under(out.clone(), None);
    assert!(
        std::env::var_os("LOCALAPPDATA")
            .map(|root| !out.starts_with(PathBuf::from(root).join("xemnas")))
            .unwrap_or(true),
        "the runner must never use the real data directory"
    );
    std::fs::create_dir_all(paths.ai_profile.parent().expect("settings dir")).expect("settings");
    let live = PathBuf::from(std::env::var_os("LOCALAPPDATA").expect("LOCALAPPDATA"))
        .join("xemnas")
        .join("settings")
        .join("ai-profile.json");
    std::fs::copy(&live, &paths.ai_profile).expect("copy the AI profile (read only)");

    let mut report = Report {
        file: std::fs::File::create(out.join("report.txt")).expect("report file"),
    };
    report.line(format!("data dir: {}", out.display()));
    let profile = settings(&paths.ai_profile)
        .load()
        .expect("profile")
        .expect("profile present");
    report.line(format!(
        "profile: kind={:?} model={:?} external_calls={}",
        profile.kind, profile.model, profile.external_calls_enabled
    ));

    let store = SqliteStore::open(&paths.database).expect("open the store");
    let failures: Failures = Arc::default();
    // One provider call is one permit of this limiter; every call of the run
    // goes through it, so the report can say what the pipeline cost.
    let limiter = application::limiter::ProviderLimiter::new(1);
    let jobs = provider_jobs(&store, &paths.ai_profile, &failures, &limiter);
    let local = observation_jobs(&store);

    // Register.
    let project = application::graph::prepared_projects(store.clone())
        .register(&project_dir)
        .expect("register the project");
    let project_id = project.id().as_str().to_string();
    report.line(format!("project {project_id} at {project_dir}"));

    // Index and propose documents.
    let documents = application::graph::prepared_documents(store.clone());
    let started = Instant::now();
    let index = documents.index(&project_id).expect("index");
    let queued = documents.propose(&project_id, 40).expect("propose");
    report.line(format!(
        "[documents] indexed {} (changed {}, truncated {}), queued {queued} in {:.1}s",
        index.total,
        index.changed,
        index.truncated,
        started.elapsed().as_secs_f64()
    ));
    drain(&mut report, "analyze documents", &jobs, &failures, &limiter);

    // Observations, like the reconciler.
    store
        .request_refresh(&RefreshRequest {
            project_id: project_id.clone(),
            capture_trigger: "reconciler".into(),
            requested_at: chrono::Utc::now().to_rfc3339(),
        })
        .expect("request refresh");
    drain(
        &mut report,
        "refresh observations",
        &local,
        &failures,
        &limiter,
    );

    // The single import of the failing document.
    if let Ok(file) = std::env::var("XEMNAS_E2E_IMPORT") {
        match documents.import_file(
            application::repo_identity::shared(),
            &project_id,
            Path::new(&file),
        ) {
            Ok(imported) => report.line(format!(
                "[import] {} queued={}",
                imported.document.path, imported.queued
            )),
            Err(error) => report.line(format!("[import] failed: {error:?}")),
        }
        drain(
            &mut report,
            "analyze imported document",
            &jobs,
            &failures,
            &limiter,
        );
    }

    // Overview, which may queue more documents.
    let chatgpt = Arc::new(ai_provider::ChatGptSession::default());
    let overview = application::overview::ProjectOverviews::new(
        store.clone(),
        settings(&paths.ai_profile),
        ai_provider::ProviderFactory::new(chatgpt.clone()).with_limiter(limiter.clone()),
    );
    let started = Instant::now();
    match application::overview::OverviewApi::generate(
        &overview,
        &project_id,
        application::output_language::OutputLanguage::ENGLISH,
    ) {
        Ok(view) => report.line(format!(
            "[overview] {} paragraph(s), {} flow(s), queued {} document(s) in {:.1}s",
            view.overview.summary.len(),
            view.overview.flows.len(),
            view.queued_documents,
            started.elapsed().as_secs_f64()
        )),
        Err(error) => report.line(format!("[overview] failed: {error:?}")),
    }
    drain(&mut report, "after overview", &jobs, &failures, &limiter);

    // Automatic review until stable; accepting queues follow-up jobs.
    let adoption: Arc<dyn application::adoption::AdoptionApi> =
        Arc::new(application::adoption::Adoption::new(store.clone()));
    let approvals = application::auto_approval::Approvals::new(
        store.clone(),
        adoption,
        settings(&paths.ai_profile),
        ai_provider::ProviderFactory::new(chatgpt).with_limiter(limiter.clone()),
    );
    approvals.set_mode(Mode::Automatic).expect("automatic mode");
    for pass in 1..=8 {
        let started = Instant::now();
        match approvals.run(&project_id) {
            Ok(run) => {
                report.line(format!(
                    "[auto review {pass}] accepted {} discarded {} left {} asked {} in {:.1}s",
                    run.accepted,
                    run.discarded,
                    run.left,
                    run.asked,
                    started.elapsed().as_secs_f64()
                ));
                drain(&mut report, "follow-up jobs", &jobs, &failures, &limiter);
                drain(
                    &mut report,
                    "follow-up observations",
                    &local,
                    &failures,
                    &limiter,
                );
                if !run.changed() {
                    break;
                }
            }
            Err(error) => {
                report.line(format!("[auto review {pass}] failed: {error}"));
                break;
            }
        }
    }

    report.line(format!(
        "provider calls in the whole run: {} (overview and automatic review included)",
        limiter.calls()
    ));
    snapshot(&mut report, &paths.database, &project_id);

    // Context packs.
    report.line("");
    report.line("== Context packs ==");
    let tasks = std::env::var("XEMNAS_E2E_TASKS").unwrap_or_default();
    let packs = application::context::ContextPacks::new(store.clone());
    for task in tasks
        .split(';')
        .map(str::trim)
        .filter(|task| !task.is_empty())
    {
        report.line(format!("-- task: {task}"));
        match packs.build_pack(ContextRequest {
            project_id: project_id.clone(),
            task: task.to_string(),
            as_of: None,
            budget_chars: None,
            files: Vec::new(),
        }) {
            Ok(pack) => {
                report.line(format!(
                    "   {} decision(s), {} rule(s), {} observation(s), omitted {}, {} chars",
                    pack.decisions.len(),
                    pack.claims.len(),
                    pack.observations.len(),
                    pack.omitted,
                    pack.used_chars
                ));
                for decision in &pack.decisions {
                    report.line(format!(
                        "   D {} -> {} scope={:?}",
                        clip(&decision.question, 100),
                        clip(&decision.choice, 140),
                        decision.scope
                    ));
                }
                for claim in &pack.claims {
                    report.line(format!(
                        "   R[{}] matched={} {} scope={:?}",
                        claim.kind,
                        claim.matched,
                        clip(&claim.statement, 160),
                        claim.inherited_scope
                    ));
                }
                match application::injection::render_compact(
                    &pack,
                    application::injection::DEFAULT_BUDGET_TOKENS,
                    &Default::default(),
                ) {
                    Some(block) => report.line(format!(
                        "   injected block ({} tokens):\n{}",
                        block.tokens, block.text
                    )),
                    None => report.line("   nothing would be injected"),
                }
            }
            Err(error) => report.line(format!("   pack failed: {error:?}")),
        }
    }
    report.line(format!("report: {}", out.join("report.txt").display()));
}

/// The runner composes registration like the app: the map is prepared, so an
/// npm workspace's packages exist as components right after `register`.
#[test]
fn registering_through_the_runner_composition_prepares_the_map() {
    let root = std::env::temp_dir().join(format!("xemnas-e2e-compose-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let project_dir = root.join("project");
    for (dir, name) in [("core", "@demo/core"), ("plugin", "@demo/plugin")] {
        let package = project_dir.join("packages").join(dir);
        std::fs::create_dir_all(&package).expect("package dir");
        std::fs::write(
            package.join("package.json"),
            format!("{{\"name\":\"{name}\"}}"),
        )
        .expect("package manifest");
    }
    std::fs::write(
        project_dir.join("package.json"),
        "{\"name\":\"demo\",\"workspaces\":[\"packages/*\"]}",
    )
    .expect("root manifest");

    let store = SqliteStore::open(root.join("app.db")).expect("open the store");
    let project = application::graph::prepared_projects(store.clone())
        .register(&project_dir)
        .expect("register");
    let names: Vec<String> = application::graph::KnowledgeGraph::new(store)
        .entities(project.id().as_str())
        .expect("entities")
        .into_iter()
        .map(|entity| entity.name)
        .collect();
    let _ = std::fs::remove_dir_all(&root);

    assert!(names.contains(&"@demo/core".to_string()), "{names:?}");
    assert!(names.contains(&"@demo/plugin".to_string()), "{names:?}");
}
