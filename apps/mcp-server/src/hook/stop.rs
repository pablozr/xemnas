//! `hook stop`: captures the finished turns of the session.
//!
//! **Checkpoint.** `<data_dir>/adapter/claude-code/<session>.json` (one file per
//! session, so concurrent sessions never share a write) keeps the byte offset of the *start of the last captured prompt* and its
//! uuid. The next Stop reads from that offset, so a turn that kept growing after
//! it was captured is seen whole, and skips the first turn when its uuid is the
//! checkpoint's. The price is re-reading one turn per Stop instead of the whole
//! file. When the file is shorter than the offset it restarts at 0 and relies on
//! the uuid and the idempotency key to avoid duplicates.

use std::path::Path;

use application::AppPaths;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::deliver::{Outcome, Sender};
use super::envelope::{build_envelope, now_rfc3339};
use super::transcript::scan;

/// Turns delivered per Stop; the rest waits for the next one.
pub const MAX_TURNS_PER_STOP: usize = 20;

/// Where a session's capture stopped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Byte offset of the last captured prompt's line.
    pub offset: u64,
    /// `uuid` of that prompt.
    pub last_prompt_uuid: String,
}

/// What one Stop did.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Report {
    /// Envelopes the app accepted.
    pub sent: usize,
    /// Envelopes written to the outbox.
    pub queued: usize,
    /// Turns skipped because the app does not know the project (403).
    pub skipped: usize,
    /// Status of the 4xx (other than 403) that stopped the run.
    pub rejected: Option<u16>,
    /// Transcript bytes read.
    pub bytes_read: u64,
}

/// Handles a `Stop` event. `Err` carries a sanitized reason.
pub fn execute(event: &Value, paths: &AppPaths) -> Result<(), &'static str> {
    let (Some(cwd), Some(session_id), Some(transcript)) = (
        event["cwd"].as_str(),
        event["session_id"].as_str(),
        event["transcript_path"].as_str(),
    ) else {
        return Err("invalid input");
    };
    let report = capture(session_id, cwd, Path::new(transcript), paths)?;
    if let Some(status) = report.rejected {
        eprintln!(
            "xemnas hook: stop capture rejected status={status} sent={} queued={}",
            report.sent, report.queued
        );
    }
    Ok(())
}

/// Captures the turns of `session_id` not yet captured.
pub fn capture(
    session_id: &str,
    cwd: &str,
    transcript: &Path,
    paths: &AppPaths,
) -> Result<Report, &'static str> {
    let checkpoint_path = checkpoint_path(&paths.data_dir, session_id);
    let known = load(&checkpoint_path);

    // One extra turn: the checkpointed one is read again and skipped.
    let scanned = scan(
        transcript,
        known.as_ref().map_or(0, |known| known.offset),
        MAX_TURNS_PER_STOP + 1,
    )
    .map_err(|_| "transcript unreadable")?;
    let mut report = Report {
        bytes_read: scanned.bytes_read,
        ..Report::default()
    };
    let mut turns = scanned.turns;
    let checkpointed = known.as_ref().map(|known| &known.last_prompt_uuid);
    if turns.first().map(|turn| &turn.prompt_uuid) == checkpointed {
        turns.remove(0);
    }
    turns.truncate(MAX_TURNS_PER_STOP);
    if turns.is_empty() {
        return Ok(report);
    }

    let mut sender = Sender::new(&paths.runtime_dir, &paths.outbox_dir);
    let mut last = None;
    // A 403 means the directory is not a registered project; it will not become
    // one mid-run, so the remaining turns are passed over without a POST.
    let mut unregistered = false;
    for turn in &turns {
        if unregistered {
            report.skipped += 1;
        } else if let Some(envelope) = build_envelope(session_id, cwd, turn, now_rfc3339()) {
            match sender.send(&envelope) {
                Outcome::Sent => report.sent += 1,
                Outcome::Queued => report.queued += 1,
                Outcome::Rejected(403) => {
                    unregistered = true;
                    report.skipped += 1;
                }
                Outcome::Rejected(status) => {
                    report.rejected = Some(status);
                    break;
                }
                Outcome::Failed => return Err("capture could not be stored"),
            }
        }
        last = Some(turn);
    }

    if let Some(turn) = last {
        let checkpoint = Checkpoint {
            offset: turn.start_offset,
            last_prompt_uuid: turn.prompt_uuid.clone(),
        };
        save(&checkpoint_path, &checkpoint).map_err(|_| "checkpoint not saved")?;
    }
    Ok(report)
}

/// `<data_dir>/adapter/claude-code/<session>.json`; the session id is reduced to
/// `[A-Za-z0-9-]` (anything else becomes `_`) and capped at 128 characters.
fn checkpoint_path(data_dir: &Path, session_id: &str) -> std::path::PathBuf {
    let name: String = session_id
        .chars()
        .take(128)
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    data_dir
        .join("adapter")
        .join("claude-code")
        .join(format!("{name}.json"))
}

fn load(path: &Path) -> Option<Checkpoint> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

/// Atomic write: temporary sibling, then rename.
fn save(path: &Path, checkpoint: &Checkpoint) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
    std::fs::write(&temporary, serde_json::to_vec(checkpoint)?)?;
    std::fs::rename(&temporary, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&temporary);
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Write;
    use std::path::PathBuf;

    const CWD: &str = "C:/proj";

    fn directory(tag: &str) -> PathBuf {
        let directory =
            std::env::temp_dir().join(format!("xemnas-hook-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("dir");
        directory
    }

    fn append(path: &Path, entries: &[Value]) {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .expect("open");
        for entry in entries {
            writeln!(file, "{entry}").expect("write");
        }
    }

    fn prompt(uuid: &str, text: &str) -> Value {
        json!({ "type": "user", "uuid": uuid, "message": { "role": "user", "content": text } })
    }

    fn assistant(blocks: Value) -> Value {
        json!({ "type": "assistant", "uuid": "a", "message": { "content": blocks } })
    }

    fn pending(paths: &AppPaths) -> usize {
        std::fs::read_dir(paths.outbox_dir.join("pending")).map_or(0, |entries| entries.count())
    }

    #[test]
    fn the_checkpoint_path_is_sanitized_and_capped() {
        let path = checkpoint_path(Path::new("d"), "ab-1/../x y");
        assert_eq!(
            path.file_name().and_then(|n| n.to_str()),
            Some("ab-1____x_y.json")
        );
        let long = checkpoint_path(Path::new("d"), &"a".repeat(300));
        assert_eq!(long.file_name().map(|n| n.len()), Some(128 + 5));
        assert_eq!(path.parent(), Some(Path::new("d/adapter/claude-code")));
    }

    #[test]
    fn groups_turns_and_ignores_what_is_not_a_prompt() {
        let dir = directory("group");
        let transcript = dir.join("t.jsonl");
        append(
            &transcript,
            &[
                json!({ "type": "summary", "summary": "x" }),
                prompt("p1", "primeiro"),
                json!({ "type": "user", "uuid": "m", "isMeta": true, "message": { "content": "meta" } }),
                json!({ "type": "user", "uuid": "c", "isCompactSummary": true,
                    "message": { "content": "This session is being continued" } }),
                prompt("slash", "<command-name>/clear</command-name>"),
                assistant(json!([
                    { "type": "thinking", "thinking": "pensando" },
                    { "type": "text", "text": "resposta" },
                    { "type": "tool_use", "id": "t1", "name": "Edit", "input": {} },
                    { "type": "tool_use", "id": "t2", "name": "Bash", "input": {} }
                ])),
                json!({ "type": "assistant", "isSidechain": true, "uuid": "s",
                    "message": { "content": [{ "type": "text", "text": "lateral" }] } }),
                json!({ "type": "user", "uuid": "r1", "toolUseResult": {
                    "filePath": "C:\\proj\\src\\a.rs",
                    "structuredPatch": [{ "oldStart": 1, "oldLines": 1, "newStart": 1, "newLines": 1,
                        "lines": ["-a", "+b"] }] },
                    "message": { "content": [{ "type": "tool_result", "tool_use_id": "t1", "content": "ok" }] } }),
                json!({ "type": "user", "uuid": "r2", "toolUseResult": "Error: boom",
                    "message": { "content": [{ "type": "tool_result", "tool_use_id": "t2", "is_error": true }] } }),
                json!({ "type": "user", "uuid": "w", "toolUseResult": {
                    "type": "create", "filePath": "C:\\proj\\n.txt", "content": "um\ndois",
                    "structuredPatch": [] },
                    "message": { "content": [{ "type": "tool_result", "tool_use_id": "t3" }] } }),
                prompt("p2", "segundo"),
                assistant(json!([{ "type": "text", "text": "ok2" }])),
            ],
        );
        let mut raw = std::fs::OpenOptions::new()
            .append(true)
            .open(&transcript)
            .expect("open");
        writeln!(raw, "linha quebrada {{").expect("write");

        let scanned = scan(&transcript, 0, 10).expect("scan");
        assert_eq!(scanned.turns.len(), 2);
        let first = &scanned.turns[0];
        assert_eq!(first.prompt_uuid, "p1");
        assert_eq!(first.assistant_texts, ["resposta"]);
        assert_eq!(first.tools.len(), 2);
        assert!(!first.tools[0].is_error && first.tools[1].is_error);
        assert_eq!(first.edits.len(), 2);
        assert_eq!(first.edits[0].hunks, "@@ -1,1 +1,1 @@\n-a\n+b\n");
        assert_eq!(first.edits[1].hunks, "@@ -0,0 +1,2 @@\n+um\n+dois\n");
        assert_eq!(scanned.turns[1].user_text, "segundo");
        assert_eq!(scan(&transcript, 0, 1).expect("scan").turns.len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_second_stop_captures_only_the_new_turn() {
        let dir = directory("incremental");
        let paths = AppPaths::under(dir.join("data"), None);
        let transcript = dir.join("t.jsonl");
        append(
            &transcript,
            &[
                prompt("p1", "um"),
                assistant(json!([{ "type": "text", "text": "a" }])),
                prompt("p2", "dois"),
                assistant(json!([{ "type": "text", "text": "b" }])),
            ],
        );

        let first = capture("s1", CWD, &transcript, &paths).expect("stop");
        assert_eq!((first.queued, pending(&paths)), (2, 2));

        let again = capture("s1", CWD, &transcript, &paths).expect("stop");
        assert_eq!((again.queued, pending(&paths)), (0, 2));

        append(
            &transcript,
            &[
                prompt("p3", "tres"),
                assistant(json!([{ "type": "text", "text": "c" }])),
            ],
        );
        let third = capture("s1", CWD, &transcript, &paths).expect("stop");
        assert_eq!((third.queued, pending(&paths)), (1, 3));
        let other = capture("s2", CWD, &transcript, &paths).expect("stop");
        assert_eq!(other.queued, 3, "checkpoints are per session");

        // A rewritten (shorter) transcript restarts; the keys deduplicate.
        std::fs::write(&transcript, "").expect("truncate");
        append(&transcript, &[prompt("p1", "um")]);
        let restarted = capture("s1", CWD, &transcript, &paths).expect("stop");
        assert_eq!(restarted.queued, 1);
        assert_eq!(pending(&paths), 6, "p1 of s1 reuses its pending file");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn at_most_twenty_turns_per_stop_oldest_first() {
        let dir = directory("limit");
        let paths = AppPaths::under(dir.join("data"), None);
        let transcript = dir.join("t.jsonl");
        let entries: Vec<Value> = (0..25).map(|n| prompt(&format!("p{n}"), "texto")).collect();
        append(&transcript, &entries);

        assert_eq!(
            capture("s", CWD, &transcript, &paths).expect("stop").queued,
            20
        );
        assert_eq!(
            capture("s", CWD, &transcript, &paths).expect("stop").queued,
            5
        );
        assert_eq!(
            capture("s", CWD, &transcript, &paths).expect("stop").queued,
            0
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_second_stop_of_a_large_transcript_reads_only_the_tail() {
        let dir = directory("large");
        let paths = AppPaths::under(dir.join("data"), None);
        let transcript = dir.join("t.jsonl");
        let filler = "x".repeat(20_000);
        let mut entries = Vec::new();
        // About 20 MB, in 1000 turns.
        for turn in 0..1_000 {
            entries.push(prompt(&format!("p{turn}"), "texto"));
            entries.push(assistant(json!([{ "type": "text", "text": filler }])));
        }
        append(&transcript, &entries);
        let size = std::fs::metadata(&transcript).expect("meta").len();
        assert!(size > 20_000_000);

        // The first Stop drains 20 turns per call; catch the checkpoint up.
        let mut first = capture("s", CWD, &transcript, &paths).expect("stop");
        while first.queued > 0 {
            first = capture("s", CWD, &transcript, &paths).expect("stop");
        }
        append(
            &transcript,
            &[
                prompt("new", "novo"),
                assistant(json!([{ "type": "text", "text": "r" }])),
            ],
        );

        let started = std::time::Instant::now();
        let second = capture("s", CWD, &transcript, &paths).expect("stop");
        let elapsed = started.elapsed();
        println!(
            "second stop: {elapsed:?}, read {} of {size} bytes, queued {}",
            second.bytes_read, second.queued
        );
        assert_eq!(second.queued, 1);
        assert!(second.bytes_read < 100_000, "{}", second.bytes_read);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
