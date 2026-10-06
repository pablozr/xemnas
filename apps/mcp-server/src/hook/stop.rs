//! `hook stop`: captures the finished turns of the session.
//!
//! **Folding.** One capture per substantive exchange: a trivial turn (see
//! [`Turn::is_trivial`]) is appended to the group before it in the same read.
//! A group already sent is never reopened or re-sent with more content; trivial
//! turns that arrive after it open their own group, and further trivial turns
//! fold into that one while it is pending. The group keeps its first turn's
//! `message_id` and idempotency key.
//!
//! **Hold-back.** Stop does not send the newest group, since the next turn may
//! fold into it; it goes out once a later non-trivial turn exists. `SessionEnd`
//! (`flush`) sends everything.
//!
//! **Checkpoint.** `<data_dir>/adapter/claude-code/<session>.json` (one file per
//! session, so concurrent sessions never share a write) keeps the byte offset of
//! the *start of the held group* and the uuid of the last turn sent. The next
//! run reads from that offset, so the held group is seen whole. After a flush
//! nothing is held: the offset is the last sent turn's line, which is skipped by
//! its uuid. The price is re-reading one group per run instead of the whole
//! file. When the file is shorter than the offset it restarts at 0 and relies on
//! the uuid and the idempotency key to avoid duplicates.

use std::path::Path;

use application::AppPaths;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::deliver::{Outcome, Sender};
use super::envelope::{build_envelope, now_rfc3339};
use super::transcript::{scan, Turn};
use super::worktree::Worktrees;

/// Captures (turn groups) delivered per Stop; the rest waits for the next one.
pub const MAX_TURNS_PER_STOP: usize = 20;

/// Trivial turns folded into one capture at most; bounds the read and the content.
pub const MAX_FOLDED_TURNS: usize = 10;

/// Turns read per run: enough for [`MAX_TURNS_PER_STOP`] full groups plus the held one.
const SCAN_LIMIT: usize = MAX_TURNS_PER_STOP * (1 + MAX_FOLDED_TURNS) + 1;

/// Runs a `SessionEnd` flush may take to drain a long backlog.
const MAX_FLUSH_RUNS: usize = 100;

/// Where a session's capture stopped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Byte offset of the first turn not captured yet (the held one), or of the
    /// last consumed turn when nothing is held.
    pub offset: u64,
    /// `uuid` of the last turn consumed by a captured group.
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
    /// A flush left groups behind (run cap); run again.
    pub more: bool,
}

/// Handles a `Stop` event, or a `SessionEnd` one when `flush`. `Err` carries a sanitized reason.
pub fn execute(event: &Value, paths: &AppPaths, flush: bool) -> Result<(), &'static str> {
    let (Some(cwd), Some(session_id), Some(transcript)) = (
        event["cwd"].as_str(),
        event["session_id"].as_str(),
        event["transcript_path"].as_str(),
    ) else {
        return Err("invalid input");
    };
    let mut report = capture(session_id, cwd, Path::new(transcript), paths, flush)?;
    // `flush` drains in several runs when more than one run's worth is pending.
    for _ in 0..MAX_FLUSH_RUNS {
        if !report.more {
            break;
        }
        let next = capture(session_id, cwd, Path::new(transcript), paths, flush)?;
        report = Report {
            sent: report.sent + next.sent,
            queued: report.queued + next.queued,
            ..next
        };
    }
    if let Some(status) = report.rejected {
        eprintln!(
            "xemnas hook: stop capture rejected status={status} sent={} queued={}",
            report.sent, report.queued
        );
    }
    Ok(())
}

/// A turn that opens a capture plus the trivial turns folded into it.
struct Group {
    /// The opening turn with the folded prompts and answers appended.
    turn: Turn,
    /// Last turn consumed by the group (the opening one when nothing folded).
    last_uuid: String,
    last_start: u64,
}

/// Groups `turns` (oldest first). A trivial turn folds into the group before it
/// in the same read, up to [`MAX_FOLDED_TURNS`]; with no group before it (the
/// first turn of the session, or the first after a flushed one) it opens its
/// own. A group already sent is never reopened: it is not in the read.
fn group(turns: Vec<Turn>) -> Vec<Group> {
    let mut groups: Vec<Group> = Vec::new();
    let mut folded = 0;
    for turn in turns {
        if let Some(open) = groups.last_mut() {
            if turn.is_trivial() && folded < MAX_FOLDED_TURNS {
                open.turn.fold(&turn);
                open.last_uuid = turn.prompt_uuid;
                open.last_start = turn.start_offset;
                folded += 1;
                continue;
            }
        }
        folded = 0;
        groups.push(Group {
            last_uuid: turn.prompt_uuid.clone(),
            last_start: turn.start_offset,
            turn,
        });
    }
    groups
}

/// Captures the groups of `session_id` not yet captured. Unless `flush`, the
/// newest group is held back: a later trivial turn may still fold into it.
pub fn capture(
    session_id: &str,
    cwd: &str,
    transcript: &Path,
    paths: &AppPaths,
    flush: bool,
) -> Result<Report, &'static str> {
    let checkpoint_path = checkpoint_path(&paths.data_dir, session_id);
    let known = load(&checkpoint_path);

    let scanned = scan(
        transcript,
        known.as_ref().map_or(0, |known| known.offset),
        SCAN_LIMIT,
    )
    .map_err(|_| "transcript unreadable")?;
    let mut report = Report {
        bytes_read: scanned.bytes_read,
        ..Report::default()
    };
    // The read stopped before the end of the file: the last group may be incomplete.
    let truncated = scanned.turns.len() == SCAN_LIMIT;
    let mut turns = scanned.turns;
    // Checkpoints that point at an already captured turn re-read it; skip it.
    let checkpointed = known.as_ref().map(|known| &known.last_prompt_uuid);
    if turns.first().map(|turn| &turn.prompt_uuid) == checkpointed {
        turns.remove(0);
    }
    let groups = group(turns);
    if groups.is_empty() {
        return Ok(report);
    }
    let limit = if flush && !truncated {
        groups.len()
    } else {
        groups.len() - 1
    };
    let limit = limit.min(MAX_TURNS_PER_STOP);
    if limit == 0 {
        return Ok(report);
    }
    report.more = flush && (truncated || limit < groups.len());

    let mut sender = Sender::new(&paths.runtime_dir, &paths.outbox_dir);
    let mut done = 0;
    let mut worktrees = Worktrees::new(cwd);
    // A 403 means the directory is not a registered project; it will not become
    // one mid-run, so the remaining turns are passed over without a POST.
    let mut unregistered = false;
    for group in &groups[..limit] {
        let turn = &group.turn;
        if unregistered {
            report.skipped += 1;
        } else if let Some(envelope) =
            build_envelope(session_id, cwd, &mut worktrees, turn, now_rfc3339())
        {
            match sender.send(&envelope) {
                Outcome::Sent => report.sent += 1,
                Outcome::Queued => report.queued += 1,
                Outcome::Rejected(403) => {
                    unregistered = true;
                    report.skipped += 1;
                }
                Outcome::Rejected(status) => {
                    report.rejected = Some(status);
                    report.more = false;
                    break;
                }
                Outcome::Failed => return Err("capture could not be stored"),
            }
        }
        done += 1;
    }

    if done > 0 {
        let last = &groups[done - 1];
        // The next group (held or over the cap) is re-read whole; with none, the
        // last consumed turn is re-read and skipped by its uuid.
        let checkpoint = Checkpoint {
            offset: groups
                .get(done)
                .map_or(last.last_start, |next| next.turn.start_offset),
            last_prompt_uuid: last.last_uuid.clone(),
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

    /// A substantive turn: a prompt over the trivial limit.
    fn exchange(uuid: &str, answer: &str) -> [Value; 2] {
        [
            prompt(uuid, &format!("{uuid} {}", "pedido ".repeat(20))),
            assistant(json!([{ "type": "text", "text": answer }])),
        ]
    }

    fn ok(uuid: &str, text: &str) -> [Value; 2] {
        [
            prompt(uuid, text),
            assistant(json!([{ "type": "text", "text": "certo" }])),
        ]
    }

    /// `(message_id, user_text, assistant_text)` of every pending envelope.
    fn envelopes(paths: &AppPaths) -> Vec<(String, String, String)> {
        let mut found = Vec::new();
        for entry in std::fs::read_dir(paths.outbox_dir.join("pending")).expect("pending") {
            let text = std::fs::read_to_string(entry.expect("entry").path()).expect("read");
            let envelope: Value = serde_json::from_str(&text).expect("json");
            let artifact = |kind: &str| {
                envelope["artifacts"]
                    .as_array()
                    .and_then(|all| all.iter().find(|a| a["kind"] == kind))
                    .and_then(|a| a["content"].as_str())
                    .unwrap_or_default()
                    .to_string()
            };
            let message_id = envelope["source"]["message_id"].as_str();
            found.push((
                message_id.unwrap_or_default().to_string(),
                artifact("user_text"),
                artifact("assistant_text"),
            ));
        }
        found
    }

    fn stop(paths: &AppPaths, transcript: &Path) -> Report {
        capture("s", CWD, transcript, paths, false).expect("stop")
    }

    fn session_end(paths: &AppPaths, transcript: &Path) -> Report {
        capture("s", CWD, transcript, paths, true).expect("session end")
    }

    #[test]
    fn the_second_stop_captures_only_the_new_turn() {
        let dir = directory("incremental");
        let paths = AppPaths::under(dir.join("data"), None);
        let transcript = dir.join("t.jsonl");
        append(&transcript, &exchange("p1", "a"));
        append(&transcript, &exchange("p2", "b"));

        let first = capture("s1", CWD, &transcript, &paths, false).expect("stop");
        assert_eq!((first.queued, pending(&paths)), (1, 1), "p2 is held back");

        let again = capture("s1", CWD, &transcript, &paths, false).expect("stop");
        assert_eq!((again.queued, pending(&paths)), (0, 1));

        append(&transcript, &exchange("p3", "c"));
        let third = capture("s1", CWD, &transcript, &paths, false).expect("stop");
        assert_eq!((third.queued, pending(&paths)), (1, 2));
        let other = capture("s2", CWD, &transcript, &paths, false).expect("stop");
        assert_eq!(other.queued, 2, "checkpoints are per session");

        // A rewritten (shorter) transcript restarts; the keys deduplicate.
        std::fs::write(&transcript, "").expect("truncate");
        append(&transcript, &exchange("p1", "a"));
        let restarted = capture("s1", CWD, &transcript, &paths, true).expect("stop");
        assert_eq!(restarted.queued, 1);
        assert_eq!(pending(&paths), 4, "p1 of s1 reuses its pending file");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_trivial_turn_folds_into_the_one_before_it() {
        let dir = directory("fold");
        let paths = AppPaths::under(dir.join("data"), None);
        let transcript = dir.join("t.jsonl");
        append(&transcript, &exchange("p1", "proponho X"));
        append(&transcript, &ok("p2", "vamos de X?"));
        append(&transcript, &ok("p3", "pode"));

        let report = session_end(&paths, &transcript);

        assert_eq!(report.queued, 1);
        let sent = envelopes(&paths);
        assert_eq!(sent.len(), 1);
        let (message_id, user, answer) = &sent[0];
        assert_eq!(message_id, "p1");
        assert!(user.starts_with("p1 pedido") && user.ends_with("vamos de X?\n\npode"));
        assert_eq!(answer, "proponho X\n\ncerto\n\ncerto");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_first_trivial_turn_is_captured_alone() {
        let dir = directory("first-trivial");
        let paths = AppPaths::under(dir.join("data"), None);
        let transcript = dir.join("t.jsonl");
        append(&transcript, &ok("p1", "oi"));
        append(&transcript, &exchange("p2", "b"));

        let report = stop(&paths, &transcript);

        assert_eq!(report.queued, 1);
        assert_eq!(envelopes(&paths)[0].0, "p1");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_turn_with_tool_use_or_a_long_prompt_is_not_folded() {
        let dir = directory("not-trivial");
        let paths = AppPaths::under(dir.join("data"), None);
        let transcript = dir.join("t.jsonl");
        append(&transcript, &exchange("p1", "a"));
        append(
            &transcript,
            &[
                prompt("p2", "roda"),
                assistant(json!([{ "type": "tool_use", "id": "t", "name": "Bash", "input": {} }])),
            ],
        );
        append(&transcript, &exchange("p3", "c"));

        assert_eq!(session_end(&paths, &transcript).queued, 3);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_newest_turn_is_held_until_a_later_one_exists() {
        let dir = directory("hold");
        let paths = AppPaths::under(dir.join("data"), None);
        let transcript = dir.join("t.jsonl");
        append(&transcript, &exchange("p1", "a"));
        assert_eq!(stop(&paths, &transcript).queued, 0, "stop 1 sends nothing");

        append(&transcript, &exchange("p2", "b"));
        assert_eq!(stop(&paths, &transcript).queued, 1, "stop 2 sends p1");

        append(&transcript, &ok("p3", "pode"));
        assert_eq!(
            stop(&paths, &transcript).queued,
            0,
            "stop 3 sends nothing new"
        );
        assert_eq!(pending(&paths), 1);

        assert_eq!(session_end(&paths, &transcript).queued, 1);
        let sent = envelopes(&paths);
        let p2 = sent.iter().find(|(id, ..)| id == "p2").expect("p2 sent");
        assert!(p2.1.ends_with("pode"), "pode folded into p2");
        assert_eq!(pending(&paths), 2);

        // Nothing left to flush; a later trivial turn opens its own group.
        assert_eq!(session_end(&paths, &transcript).queued, 0);
        append(&transcript, &ok("p4", "obrigado"));
        assert_eq!(
            stop(&paths, &transcript).queued,
            0,
            "held: it opens its own group"
        );
        assert_eq!(session_end(&paths, &transcript).queued, 1);
        assert_eq!(pending(&paths), 3);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_checkpoint_points_at_the_held_turn() {
        let dir = directory("checkpoint");
        let paths = AppPaths::under(dir.join("data"), None);
        let transcript = dir.join("t.jsonl");
        append(&transcript, &exchange("p1", "a"));
        let held = std::fs::metadata(&transcript).expect("meta").len();
        append(&transcript, &exchange("p2", "b"));

        stop(&paths, &transcript);

        let saved = load(&checkpoint_path(&paths.data_dir, "s")).expect("checkpoint");
        assert_eq!(saved.offset, held);
        assert_eq!(saved.last_prompt_uuid, "p1");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn at_most_twenty_turns_per_stop_oldest_first() {
        let dir = directory("limit");
        let paths = AppPaths::under(dir.join("data"), None);
        let transcript = dir.join("t.jsonl");
        for n in 0..25 {
            append(&transcript, &exchange(&format!("p{n}"), "a"));
        }

        assert_eq!(stop(&paths, &transcript).queued, 20);
        assert_eq!(stop(&paths, &transcript).queued, 4, "the newest is held");
        assert_eq!(stop(&paths, &transcript).queued, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn session_end_drains_a_backlog_over_the_cap() {
        let dir = directory("drain");
        let paths = AppPaths::under(dir.join("data"), None);
        let transcript = dir.join("t.jsonl");
        for n in 0..25 {
            append(&transcript, &exchange(&format!("p{n}"), "a"));
        }
        let event = json!({ "cwd": CWD, "session_id": "s", "transcript_path": transcript });

        execute(&event, &paths, true).expect("session end");

        assert_eq!(pending(&paths), 25);
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
        let mut first = stop(&paths, &transcript);
        while first.queued > 0 {
            first = stop(&paths, &transcript);
        }
        append(
            &transcript,
            &[
                prompt("new", "novo"),
                assistant(json!([{ "type": "text", "text": "r".repeat(700) }])),
            ],
        );

        let started = std::time::Instant::now();
        let second = stop(&paths, &transcript);
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
