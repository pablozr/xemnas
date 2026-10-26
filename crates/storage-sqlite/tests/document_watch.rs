//! Gate for the document watch of the reconciler: a parked project costs no
//! job, an edited document costs exactly one (after one pass of settling),
//! bursts of edits cost one, archived and never indexed projects cost none,
//! and the pass itself is cheap for many projects.

mod support;

use application::documents::watch::{DocumentWatch, WatchOutcome};
use application::documents::{Documents, PROPOSE_PER_RUN};
use application::projects::{ProjectRecord, ProjectRepository};
use std::path::PathBuf;
use std::time::Instant;
use support::{count, raw};

/// Idle passes simulated, one per 60 s of the real reconciler.
const IDLE_TICKS: usize = 60;
/// Projects, documents per project and passes of the cost gate.
const PROJECTS: usize = 64;
const DOCUMENTS: usize = 40;
const COST_TICKS: usize = 50;
/// Ceiling for one pass over 64 projects with 40 documents each (2,560 files,
/// listed and stamped, none opened). Measured on 17/10/2026 in four runs: p50
/// 67 to 87 ms, p95 83 to 221 ms (the listing is filesystem bound and noisy);
/// about five times a typical p95, like the other latency gates.
const WATCH_TICK_P95_CEILING_MS: f64 = 500.0;

const ADR: &str = "docs/adr/0001-sqlite.md";

/// A decision-bearing ADR; `note` changes its size.
fn adr_text(note: &str) -> String {
    format!(
        "# Usar SQLite\n\n## Decisão\nGuardar tudo em SQLite, em crates/core/src/lib.rs. {note}\n"
    )
}

fn analysis_jobs(test: &support::TestStore) -> i64 {
    count(
        test,
        "SELECT COUNT(*) FROM jobs WHERE kind = 'analyze_document'",
    )
}

fn receipts(test: &support::TestStore) -> i64 {
    count(test, "SELECT COUNT(*) FROM capture_receipts")
}

fn write(repo: &std::path::Path, path: &str, text: &str) {
    let file = repo.join(path);
    std::fs::create_dir_all(file.parent().expect("parent")).expect("dirs");
    std::fs::write(file, text).expect("write");
}

fn register(test: &support::TestStore, project: &str, files: &[(&str, String)]) -> PathBuf {
    let repo = test.root.join(format!("repo-{project}"));
    for (path, text) in files {
        write(&repo, path, text);
    }
    test.store
        .insert(&ProjectRecord::new(
            project.into(),
            repo.to_string_lossy().replace('\\', "/"),
            "2026-01-01T00:00:00Z".into(),
        ))
        .expect("project");
    repo
}

/// A project already read and with nothing waiting: the state of a project
/// that was parked before the watch started.
fn parked(
    test: &support::TestStore,
    documents: &Documents<storage_sqlite::SqliteStore>,
    project: &str,
) -> PathBuf {
    let repo = register(test, project, &[(ADR, adr_text("v1"))]);
    documents.index(project).expect("index");
    documents.propose(project, 10).expect("propose");
    repo
}

fn tick(
    watch: &mut DocumentWatch,
    documents: &Documents<storage_sqlite::SqliteStore>,
    project: &str,
) -> WatchOutcome {
    watch.tick(documents, project).expect("tick")
}

/// File named by the artifact of the newest analysis job.
fn newest_job_file(test: &support::TestStore) -> String {
    let connection = raw(test);
    let capture: String = connection
        .query_row(
            "SELECT payload FROM jobs WHERE kind = 'analyze_document'
             ORDER BY created_at DESC, rowid DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .expect("job");
    let metadata: String = connection
        .query_row(
            "SELECT metadata FROM capture_artifacts WHERE capture_id = ?1",
            [&capture],
            |row| row.get(0),
        )
        .expect("artifact");
    let metadata: serde_json::Value = serde_json::from_str(&metadata).expect("json");
    metadata["file"].as_str().expect("file").to_string()
}

#[test]
fn document_watch_gate() {
    let test = support::open("document-watch", &[]);
    let documents = Documents::new(test.store.clone());
    let mut watch = DocumentWatch::default();

    // 1. A parked project: settling, one empty refresh, then nothing at all.
    let repo = parked(&test, &documents, "idle");
    let (jobs, receipts_before) = (analysis_jobs(&test), receipts(&test));
    assert_eq!(tick(&mut watch, &documents, "idle"), WatchOutcome::Settling);
    assert_eq!(
        tick(&mut watch, &documents, "idle"),
        WatchOutcome::Refreshed { queued: 0 }
    );
    for _ in 0..IDLE_TICKS {
        assert_eq!(
            tick(&mut watch, &documents, "idle"),
            WatchOutcome::Unchanged
        );
    }
    assert_eq!(analysis_jobs(&test) - jobs, 0, "idle jobs");
    assert_eq!(receipts(&test) - receipts_before, 0, "idle receipts");

    // 2. An edited ADR: one pass of settling, then exactly one job for it,
    // and no overview in the way.
    write(&repo, ADR, &adr_text("v2 com mais uma frase"));
    assert_eq!(tick(&mut watch, &documents, "idle"), WatchOutcome::Settling);
    assert_eq!(
        tick(&mut watch, &documents, "idle"),
        WatchOutcome::Refreshed { queued: 1 }
    );
    let edited_jobs = analysis_jobs(&test) - jobs;
    assert_eq!(edited_jobs, 1);
    assert_eq!(newest_job_file(&test), ADR);
    assert_eq!(count(&test, "SELECT COUNT(*) FROM project_overviews"), 0);
    assert_eq!(
        tick(&mut watch, &documents, "idle"),
        WatchOutcome::Unchanged
    );

    // 3. Three edits in three passes in a row, then one parked pass: one job.
    let before_burst = analysis_jobs(&test);
    for note in ["a", "bb", "ccc"] {
        write(&repo, ADR, &adr_text(note));
        assert_eq!(tick(&mut watch, &documents, "idle"), WatchOutcome::Settling);
    }
    assert_eq!(
        tick(&mut watch, &documents, "idle"),
        WatchOutcome::Refreshed { queued: 1 }
    );
    let burst_jobs = analysis_jobs(&test) - before_burst;
    assert_eq!(burst_jobs, 1);

    // 4. A new ADR: one job.
    let before_new = analysis_jobs(&test);
    write(&repo, "docs/adr/0002-novo.md", &adr_text("outro"));
    assert_eq!(tick(&mut watch, &documents, "idle"), WatchOutcome::Settling);
    assert_eq!(
        tick(&mut watch, &documents, "idle"),
        WatchOutcome::Refreshed { queued: 1 }
    );
    let new_jobs = analysis_jobs(&test) - before_new;
    assert_eq!(new_jobs, 1);
    assert_eq!(newest_job_file(&test), "docs/adr/0002-novo.md");

    // 5. A document in an archive folder is indexed but never queued.
    let before_archived = analysis_jobs(&test);
    write(&repo, "docs/archive-x/0002.md", &adr_text("velho"));
    assert_eq!(tick(&mut watch, &documents, "idle"), WatchOutcome::Settling);
    assert_eq!(
        tick(&mut watch, &documents, "idle"),
        WatchOutcome::Refreshed { queued: 0 }
    );
    write(&repo, "docs/archive-x/0002.md", &adr_text("velho editado"));
    tick(&mut watch, &documents, "idle");
    tick(&mut watch, &documents, "idle");
    let archived_jobs = analysis_jobs(&test) - before_archived;
    assert_eq!(archived_jobs, 0);

    // 6. A project nobody indexed: the watch indexes and proposes nothing.
    register(&test, "unindexed", &[(ADR, adr_text("v1"))]);
    let before_unindexed = analysis_jobs(&test);
    for _ in 0..4 {
        tick(&mut watch, &documents, "unindexed");
    }
    let unindexed_jobs = analysis_jobs(&test) - before_unindexed;
    assert_eq!(unindexed_jobs, 0);
    assert_eq!(
        count(
            &test,
            "SELECT COUNT(*) FROM project_documents WHERE project_id = 'unindexed'"
        ),
        0
    );

    println!(
        "gate document_watch idle_ticks={IDLE_TICKS} jobs={} edited_jobs={edited_jobs} \
         burst_jobs={burst_jobs} new_jobs={new_jobs} archived_jobs={archived_jobs} \
         unindexed_jobs={unindexed_jobs}",
        analysis_jobs(&test) - jobs - edited_jobs - burst_jobs - new_jobs
    );
}

#[test]
fn document_watch_tick_latency_gate() {
    let test = support::open("document-watch-cost", &[]);
    let documents = Documents::new(test.store.clone());
    let ids: Vec<String> = (0..PROJECTS).map(|index| format!("p{index}")).collect();
    for id in &ids {
        let files: Vec<(String, String)> = (0..DOCUMENTS)
            .map(|index| {
                let folder = match index % 3 {
                    0 => "docs",
                    1 => "docs/adr",
                    _ => "docs/guias/a",
                };
                (
                    format!("{folder}/doc-{index}.md"),
                    adr_text(&format!("{index}")),
                )
            })
            .collect();
        let files: Vec<(&str, String)> = files
            .iter()
            .map(|(path, text)| (path.as_str(), text.clone()))
            .collect();
        register(&test, id, &files);
        documents.index(id).expect("index");
    }

    let mut watch = DocumentWatch::default();
    for id in &ids {
        assert_eq!(tick(&mut watch, &documents, id), WatchOutcome::Settling);
    }
    // The settling pass: the folder is read again, nothing changed.
    let start = Instant::now();
    for id in &ids {
        assert_eq!(
            documents
                .refresh_changed(id, PROPOSE_PER_RUN)
                .expect("refresh"),
            0
        );
    }
    let settle_ms = start.elapsed().as_secs_f64() * 1000.0;
    println!("latency document_watch_settle_pass ms={settle_ms:.2} projects={PROJECTS}");
    for id in &ids {
        tick(&mut watch, &documents, id);
    }

    let mut samples = Vec::new();
    for _ in 0..COST_TICKS {
        let start = Instant::now();
        for id in &ids {
            assert_eq!(tick(&mut watch, &documents, id), WatchOutcome::Unchanged);
        }
        samples.push(start.elapsed().as_micros());
    }
    samples.sort_unstable();
    let p50 = samples[samples.len() / 2] as f64 / 1000.0;
    let p95 = samples[samples.len() * 95 / 100] as f64 / 1000.0;
    println!(
        "latency document_watch_tick p50={p50:.2}ms p95={p95:.2}ms projects={PROJECTS} \
         documents={DOCUMENTS}"
    );
    assert!(
        p95 <= WATCH_TICK_P95_CEILING_MS,
        "p95 {p95}ms above {WATCH_TICK_P95_CEILING_MS}ms"
    );
}
