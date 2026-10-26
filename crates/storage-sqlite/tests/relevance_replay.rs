//! Replays the relevance step over a read-only copy of a real database: how many
//! analyzed captures would go straight to the extractor, to the turn triage or
//! nowhere today, at what input cost, and which captures that once produced
//! candidates would no longer reach the extractor by structure alone. Makes no
//! AI call. Manual (`#[ignore]`), never against the live database.
//!
//!     XEMNAS_REPLAY_DB=<copy of app.db> cargo test --locked -p storage-sqlite \
//!         --test relevance_replay -- --ignored --nocapture

use std::collections::BTreeMap;
use std::path::Path;

use application::extract::{
    assess_relevance, triage_input_chars, truncate_content, DecisionEvidence, EvidenceArtifact,
    Relevance, MAX_EVIDENCE_ARTIFACTS, MAX_EVIDENCE_CONTENT_BYTES,
};
use application::{AppPaths, DEFAULT_MAX_INPUT_CHARS};
use rusqlite::{Connection, OpenFlags};

#[derive(Default)]
struct Tally {
    captures: usize,
    pass: usize,
    triage: usize,
    empty: usize,
    triage_input_chars: usize,
    extractor_input_chars: usize,
    now_triage: usize,
    now_empty: usize,
}

impl Tally {
    fn line(&self, scope: &str) -> String {
        format!(
            "replay relevance {scope} captures={} pass={} triage={} empty={} \
             triage_input_chars={} triage_est_tokens={} extractor_input_chars={} \
             extractor_est_tokens={} at_risk={} now_triage={} now_empty={}",
            self.captures,
            self.pass,
            self.triage,
            self.empty,
            self.triage_input_chars,
            self.triage_input_chars / 4,
            self.extractor_input_chars,
            self.extractor_input_chars / 4,
            self.now_triage + self.now_empty,
            self.now_triage,
            self.now_empty,
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

    // Every analyzed capture, with what its latest assessment produced.
    let mut captures = connection
        .prepare(
            "SELECT r.capture_id, r.canonical_path,
                    (SELECT a.candidates FROM assessments a WHERE a.capture_id = r.capture_id
                     ORDER BY a.finished_at DESC, a.rowid DESC LIMIT 1)
             FROM capture_receipts r
             WHERE EXISTS (SELECT 1 FROM jobs j WHERE j.kind = 'analyze_capture'
                           AND j.payload = r.capture_id)
               AND EXISTS (SELECT 1 FROM assessments a WHERE a.capture_id = r.capture_id)
             ORDER BY r.capture_id",
        )
        .expect("captures");
    let rows: Vec<(String, String, i64)> = captures
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
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
    for (capture_id, canonical_path, candidates) in rows {
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
        let relevance = assess_relevance(&evidence);
        let extractor_chars: usize = evidence
            .artifacts
            .iter()
            .map(|artifact| {
                application::external::limited_text(&artifact.content, DEFAULT_MAX_INPUT_CHARS)
                    .chars()
                    .count()
            })
            .sum();
        let at_risk = candidates > 0 && !matches!(relevance, Relevance::Pass(_));
        let mut tallies = [&mut total, per_project.entry(project).or_default()];
        for tally in &mut tallies {
            tally.captures += 1;
            match &relevance {
                Relevance::Pass(_) => {
                    tally.pass += 1;
                    tally.extractor_input_chars += extractor_chars;
                }
                Relevance::Triage(input) => {
                    tally.triage += 1;
                    tally.triage_input_chars += triage_input_chars(input);
                    tally.now_triage += usize::from(at_risk);
                }
                Relevance::Empty => {
                    tally.empty += 1;
                    tally.now_empty += usize::from(at_risk);
                }
            }
        }
        if at_risk {
            let now = if matches!(relevance, Relevance::Triage(_)) {
                "triage"
            } else {
                "empty"
            };
            println!(
                "replay risk {} was_candidates={candidates} now={now}",
                &capture_id[..capture_id.len().min(13)]
            );
        }
    }
    for (project, tally) in &per_project {
        println!("{}", tally.line(&format!("project={project}")));
    }
    println!("{}", total.line("total"));
}
