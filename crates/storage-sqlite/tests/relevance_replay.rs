//! Replays the relevance filter over a read-only copy of a real database: how
//! many captures the extractor judged empty would reach it today, and at what
//! input cost. Manual (`#[ignore]`), never against the live database.
//!
//!     XEMNAS_REPLAY_DB=<copy of app.db> cargo test --locked -p storage-sqlite \
//!         --test relevance_replay -- --ignored --nocapture

use std::collections::BTreeMap;
use std::path::Path;

use application::extract::{
    filter_relevant, truncate_content, DecisionEvidence, EvidenceArtifact, MAX_EVIDENCE_ARTIFACTS,
    MAX_EVIDENCE_CONTENT_BYTES,
};
use application::{AppPaths, DEFAULT_MAX_INPUT_CHARS};
use rusqlite::{Connection, OpenFlags};

#[derive(Default)]
struct Tally {
    empty: usize,
    pass_now: usize,
    input_chars: usize,
}

impl Tally {
    fn line(&self, scope: &str) -> String {
        format!(
            "replay relevance {scope} empty={} pass_now={} calls={} input_chars={} est_tokens={}",
            self.empty,
            self.pass_now,
            self.pass_now,
            self.input_chars,
            self.input_chars / 4
        )
    }
}

/// Refuses the user's live database and anything under a `xemnas/state` folder.
fn refuse_live_database(path: &str) {
    let given = Path::new(path)
        .canonicalize()
        .expect("replay database exists");
    let live = AppPaths::from_env().database.canonicalize().ok();
    let lowered = given.to_string_lossy().to_lowercase().replace('\\', "/");
    assert!(
        live.as_ref() != Some(&given) && !lowered.contains("xemnas/state"),
        "refusing to replay over the live database: {path}"
    );
}

fn open_read_only(path: &str) -> Connection {
    let uri = path.replace('\\', "/");
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI;
    let connection = Connection::open_with_flags(format!("file:{uri}?mode=ro"), flags)
        .or_else(|_| Connection::open_with_flags(format!("file:{uri}?immutable=1"), flags))
        .expect("open read-only");
    connection
        .execute_batch("PRAGMA query_only = 1;")
        .expect("query_only");
    connection
}

#[test]
#[ignore = "manual: needs XEMNAS_REPLAY_DB"]
fn relevance_replay() {
    let Ok(path) = std::env::var("XEMNAS_REPLAY_DB") else {
        return;
    };
    refuse_live_database(&path);
    let connection = open_read_only(&path);

    let mut captures = connection
        .prepare(
            "SELECT r.capture_id, r.canonical_path FROM capture_receipts r
             WHERE EXISTS (SELECT 1 FROM jobs j WHERE j.kind = 'analyze_capture'
                           AND j.payload = r.capture_id)
               AND (SELECT a.outcome FROM assessments a WHERE a.capture_id = r.capture_id
                    ORDER BY a.finished_at DESC, a.rowid DESC LIMIT 1) = 'empty'
             ORDER BY r.capture_id",
        )
        .expect("captures");
    let rows: Vec<(String, String)> = captures
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .expect("rows")
        .map(|row| row.expect("row"))
        .collect();

    let mut artifacts = connection
        .prepare(
            "SELECT artifact_id, kind, content, metadata FROM capture_artifacts
             WHERE capture_id = ?1 ORDER BY artifact_id LIMIT ?2",
        )
        .expect("artifacts");

    let mut total = Tally::default();
    let mut per_project: BTreeMap<String, Tally> = BTreeMap::new();
    for (capture_id, canonical_path) in rows {
        let evidence = DecisionEvidence {
            capture_id: capture_id.clone(),
            project_id: canonical_path.clone(),
            adapter: None,
            session_id: None,
            observed_at: None,
            artifacts: artifacts
                .query_map((&capture_id, MAX_EVIDENCE_ARTIFACTS as i64), |row| {
                    let content: String = row.get(2)?;
                    Ok(EvidenceArtifact {
                        artifact_id: row.get(0)?,
                        kind: row.get(1)?,
                        content: truncate_content(&content, MAX_EVIDENCE_CONTENT_BYTES),
                        metadata: row.get(3)?,
                    })
                })
                .expect("artifact rows")
                .map(|row| row.expect("artifact"))
                .collect(),
        };
        let project = canonical_path
            .trim_end_matches(['/', '\\'])
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or("?")
            .to_string();
        let signals = filter_relevant(&evidence);
        let mut tallies = [&mut total, per_project.entry(project).or_default()];
        for tally in &mut tallies {
            tally.empty += 1;
        }
        if signals.is_empty() {
            continue;
        }
        let chars: usize = evidence
            .artifacts
            .iter()
            .map(|artifact| {
                application::external::limited_text(&artifact.content, DEFAULT_MAX_INPUT_CHARS)
                    .chars()
                    .count()
            })
            .sum();
        for tally in &mut tallies {
            tally.pass_now += 1;
            tally.input_chars += chars;
        }
        let names: Vec<&str> = signals.iter().map(|signal| signal.as_str()).collect();
        println!(
            "replay pass {} signals={}",
            &capture_id[..capture_id.len().min(13)],
            names.join(",")
        );
    }
    for (project, tally) in &per_project {
        println!("{}", tally.line(&format!("project={project}")));
    }
    println!("{}", total.line("total"));
}
