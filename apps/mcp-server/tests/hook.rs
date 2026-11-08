//! End to end: `xemnas-mcp hook prompt|stop|session-end` against a fake local API.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

/// Every hook spawn holds this for reading; the latency test holds it for writing,
/// so it measures the hook alone and not the other tests' processes.
static SPAWNS: RwLock<()> = RwLock::new(());

fn temporary_directory(tag: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!("xemnas-hook-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(directory.join("state")).expect("state dir");
    directory
}

/// Answers every request with `status` and `body`, recording the raw requests.
fn fake_api(status: &'static str, body: Value) -> (u16, Arc<Mutex<Vec<String>>>) {
    fake_api_by(move |_| status, body)
}

/// Like `fake_api`, with the status chosen from the raw request.
fn fake_api_by(
    status_of: impl Fn(&str) -> &'static str + Send + 'static,
    body: Value,
) -> (u16, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let recorder = seen.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { break };
            let mut request = Vec::new();
            let mut buffer = [0u8; 8192];
            // Read until the declared body is complete.
            loop {
                let read = stream.read(&mut buffer).unwrap_or(0);
                request.extend_from_slice(&buffer[..read]);
                let text = String::from_utf8_lossy(&request).to_string();
                let complete = text.split_once("\r\n\r\n").is_some_and(|(head, rest)| {
                    let length = head
                        .to_ascii_lowercase()
                        .lines()
                        .find_map(|line| line.strip_prefix("content-length: ").map(str::to_string))
                        .and_then(|value| value.trim().parse::<usize>().ok())
                        .unwrap_or(0);
                    rest.len() >= length
                });
                if read == 0 || complete {
                    break;
                }
            }
            let text = String::from_utf8_lossy(&request).to_string();
            let status = status_of(&text);
            recorder.lock().expect("lock").push(text);
            let payload = body.to_string();
            let _ = write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
                payload.len()
            );
        }
    });
    (port, seen)
}

fn write_runtime(data_dir: &Path, port: u16) {
    std::fs::write(
        data_dir.join("state").join("discovery.json"),
        json!({ "protocol_version": 1, "port": port, "instance_id": "i" }).to_string(),
    )
    .expect("discovery");
    std::fs::write(data_dir.join("state").join("api-token"), "tok-123").expect("token");
}

fn hook(data_dir: &Path, subcommand: &str, event: &Value) -> Output {
    let _shared = SPAWNS
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    spawn_hook(data_dir, subcommand, event)
}

fn spawn_hook(data_dir: &Path, subcommand: &str, event: &Value) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_xemnas-mcp"))
        .args(["hook", subcommand])
        .env("XEMNAS_DATA_DIR", data_dir)
        .env_remove("XEMNAS_OUTBOX_DIR")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(event.to_string().as_bytes())
        .expect("write");
    child.wait_with_output().expect("output")
}

fn prompt_event(transcript: &Path) -> Value {
    json!({
        "session_id": "sess-1",
        "transcript_path": transcript,
        "cwd": "C:/proj",
        "hook_event_name": "UserPromptSubmit",
        "prompt": "como escolhemos o banco?"
    })
}

fn write_transcript(path: &Path) {
    let lines = [
        json!({ "type": "user", "uuid": "p1", "message": { "content": "edite a" } }),
        json!({ "type": "assistant", "uuid": "a1", "message": { "content": [
            { "type": "text", "text": "feito" },
            { "type": "tool_use", "id": "t1", "name": "Edit", "input": { "file_path": "C:\\proj\\src\\a.rs" } }
        ] } }),
        json!({ "type": "user", "uuid": "r1", "toolUseResult": { "filePath": "C:\\proj\\src\\a.rs",
            "structuredPatch": [{ "oldStart": 1, "oldLines": 1, "newStart": 1, "newLines": 1, "lines": ["-a", "+b"] }] },
            "message": { "content": [{ "type": "tool_result", "tool_use_id": "t1" }] } }),
        json!({ "type": "user", "uuid": "p2", "message": { "content": "agora b" } }),
        json!({ "type": "assistant", "uuid": "a2", "message": { "content": [
            { "type": "text", "text": "ok" },
            { "type": "tool_use", "id": "t2", "name": "Read", "input": { "file_path": "C:\\proj\\src\\b.rs" } }
        ] } }),
    ];
    let text: Vec<String> = lines.iter().map(Value::to_string).collect();
    std::fs::write(path, text.join("\n") + "\n").expect("transcript");
}

#[test]
fn hook_prompt_prints_the_additional_context() {
    let data_dir = temporary_directory("prompt");
    let transcript = data_dir.join("t.jsonl");
    write_transcript(&transcript);
    let (port, seen) = fake_api(
        "200 OK",
        json!({ "mode": "balanced", "context": "<xemnas-context>D:abc</xemnas-context>",
            "tokens": 5, "items": 1, "omitted": 0 }),
    );
    write_runtime(&data_dir, port);

    let output = hook(&data_dir, "prompt", &prompt_event(&transcript));

    assert!(output.status.success());
    let line = String::from_utf8(output.stdout).expect("utf8");
    let printed: Value = serde_json::from_str(line.trim()).expect("one json line");
    assert_eq!(line.trim().lines().count(), 1);
    assert_eq!(
        printed["hookSpecificOutput"],
        json!({ "hookEventName": "UserPromptSubmit",
            "additionalContext": "<xemnas-context>D:abc</xemnas-context>" })
    );

    let requests = seen.lock().expect("lock");
    let request = &requests[0];
    assert!(request.starts_with("POST /v1/context "));
    assert!(request
        .to_ascii_lowercase()
        .contains("authorization: bearer tok-123"));
    let body: Value =
        serde_json::from_str(request.split_once("\r\n\r\n").expect("body").1).expect("json body");
    assert_eq!(body["canonical_path"], json!("C:/proj"));
    assert_eq!(body["session_id"], json!("sess-1"));
    assert_eq!(body["prompt"], json!("como escolhemos o banco?"));
    assert_eq!(body["files"], json!(["src/b.rs", "src/a.rs"]));
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn hook_prompt_prints_nothing_without_context_or_when_the_app_is_closed() {
    let data_dir = temporary_directory("prompt-closed");
    let transcript = data_dir.join("missing.jsonl");

    let closed = hook(&data_dir, "prompt", &prompt_event(&transcript));
    assert!(closed.status.success());
    assert!(closed.stdout.is_empty() && closed.stderr.is_empty());

    let (port, _) = fake_api(
        "200 OK",
        json!({ "mode": "balanced", "context": null, "tokens": 0, "items": 0, "omitted": 0 }),
    );
    write_runtime(&data_dir, port);
    let empty = hook(&data_dir, "prompt", &prompt_event(&transcript));
    assert!(empty.status.success() && empty.stdout.is_empty());

    let garbage = hook(&data_dir, "prompt", &json!("nao e um evento"));
    assert!(garbage.status.success() && garbage.stdout.is_empty());
    assert!(String::from_utf8_lossy(&garbage.stderr).lines().count() <= 1);
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn hook_prompt_latency_stays_under_the_budget() {
    let data_dir = temporary_directory("latency");
    let transcript = data_dir.join("t.jsonl");
    write_transcript(&transcript);
    let (port, _) = fake_api(
        "200 OK",
        json!({ "mode": "balanced", "context": "ctx", "tokens": 1, "items": 1, "omitted": 0 }),
    );
    write_runtime(&data_dir, port);
    let event = prompt_event(&transcript);

    // The first spawn of a fresh executable pays the antivirus scan; not the hook.
    let _alone = SPAWNS
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    spawn_hook(&data_dir, "prompt", &event);
    let mut times: Vec<Duration> = (0..10)
        .map(|_| {
            let started = Instant::now();
            let output = spawn_hook(&data_dir, "prompt", &event);
            let elapsed = started.elapsed();
            assert!(!output.stdout.is_empty());
            elapsed
        })
        .collect();
    times.sort();
    println!(
        "hook prompt wall time over 10 runs, spawn included: min {:?}, median {:?}, p95 {:?}",
        times[0], times[5], times[9]
    );
    assert!(times[9] <= Duration::from_millis(300), "{:?}", times[9]);
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn hook_stop_posts_one_envelope_per_exchange_holding_the_newest_back() {
    let data_dir = temporary_directory("stop");
    let transcript = data_dir.join("t.jsonl");
    write_transcript(&transcript);
    let (port, seen) = fake_api("202 Accepted", json!({ "capture_id": "c" }));
    write_runtime(&data_dir, port);
    let event = json!({ "session_id": "sess-1", "transcript_path": transcript,
        "cwd": "C:/proj", "hook_event_name": "Stop", "stop_hook_active": false });

    let output = hook(&data_dir, "stop", &event);
    assert!(output.status.success() && output.stdout.is_empty());

    let requests = seen.lock().expect("lock").clone();
    assert_eq!(requests.len(), 1, "p2 is held back for a possible fold");
    for (request, prompt) in requests.iter().zip(["p1"]) {
        assert!(request.starts_with("POST /v1/captures "));
        let lower = request.to_ascii_lowercase();
        assert!(lower.contains("authorization: bearer tok-123"));
        assert!(lower.contains(&format!("idempotency-key: claude-code:sess-1:{prompt}:")));
        let body: Value = serde_json::from_str(request.split_once("\r\n\r\n").expect("body").1)
            .expect("envelope");
        assert_eq!(body["source"]["adapter"], json!("claude-code"));
        assert_eq!(body["source"]["message_id"], json!(prompt));
    }
    let first: Value = serde_json::from_str(requests[0].split_once("\r\n\r\n").expect("body").1)
        .expect("envelope");
    let kinds: Vec<&str> = first["artifacts"]
        .as_array()
        .expect("artifacts")
        .iter()
        .filter_map(|artifact| artifact["kind"].as_str())
        .collect();
    assert_eq!(
        kinds,
        ["user_text", "assistant_text", "diff_hunk", "tool_summary"]
    );

    // Nothing new: a second Stop sends nothing.
    hook(&data_dir, "stop", &event);
    assert_eq!(seen.lock().expect("lock").len(), 1);

    // SessionEnd flushes the held turn, silently.
    let end = json!({ "session_id": "sess-1", "transcript_path": transcript,
        "cwd": "C:/proj", "hook_event_name": "SessionEnd", "reason": "other" });
    let output = hook(&data_dir, "session-end", &end);
    assert!(output.status.success() && output.stdout.is_empty() && output.stderr.is_empty());
    let requests = seen.lock().expect("lock").clone();
    assert_eq!(requests.len(), 2);
    assert!(requests[1]
        .to_ascii_lowercase()
        .contains("idempotency-key: claude-code:sess-1:p2:"));
    hook(&data_dir, "session-end", &end);
    assert_eq!(seen.lock().expect("lock").len(), 2);
    assert!(!data_dir.join("outbox").join("pending").exists());
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn hook_stop_writes_to_the_outbox_when_the_app_is_closed() {
    let data_dir = temporary_directory("stop-closed");
    let transcript = data_dir.join("t.jsonl");
    write_transcript(&transcript);
    let event = json!({ "session_id": "sess-1", "transcript_path": transcript,
        "cwd": "C:/proj", "hook_event_name": "Stop", "stop_hook_active": false });

    let output = hook(&data_dir, "stop", &event);
    assert!(output.status.success() && output.stdout.is_empty());
    hook(&data_dir, "session-end", &event);

    let pending = data_dir.join("outbox").join("pending");
    let files: Vec<_> = std::fs::read_dir(&pending)
        .expect("pending")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .to_string()
        })
        .collect();
    assert_eq!(files.len(), 2);
    assert!(files
        .iter()
        .all(|name| name.len() == 64 + 5 && name.ends_with(".json")));
    let envelope: Value = serde_json::from_str(
        &std::fs::read_to_string(pending.join(&files[0])).expect("envelope file"),
    )
    .expect("json");
    assert_eq!(envelope["schema_version"], json!(1));
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn hook_stop_passes_over_unregistered_projects_without_jamming() {
    let data_dir = temporary_directory("stop-403");
    let transcript = data_dir.join("t.jsonl");
    write_transcript(&transcript);
    let (port, seen) = fake_api("403 Forbidden", json!({ "error": "project_not_found" }));
    write_runtime(&data_dir, port);
    let event = json!({ "session_id": "sess-1", "transcript_path": transcript,
        "cwd": "C:/proj", "hook_event_name": "Stop", "stop_hook_active": false });

    let output = hook(&data_dir, "stop", &event);
    assert!(output.status.success() && output.stdout.is_empty() && output.stderr.is_empty());
    assert_eq!(seen.lock().expect("lock").len(), 1, "p1 only; p2 is held");
    hook(&data_dir, "session-end", &event);
    assert_eq!(
        seen.lock().expect("lock").len(),
        2,
        "the flush tries p2 once"
    );
    assert!(!data_dir.join("outbox").join("pending").exists());
    let checkpoint = data_dir
        .join("adapter")
        .join("claude-code")
        .join("sess-1.json");
    let saved: Value =
        serde_json::from_str(&std::fs::read_to_string(checkpoint).expect("checkpoint"))
            .expect("json");
    assert_eq!(saved["last_prompt_uuid"], json!("p2"));

    hook(&data_dir, "session-end", &event);
    assert_eq!(seen.lock().expect("lock").len(), 2, "nothing new to send");
    let _ = std::fs::remove_dir_all(&data_dir);
}

// ---- Sessions whose cwd is not a repository (OpenBot agents edit other repos by absolute path).

fn git(directory: &Path, args: &[&str]) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(directory)
        .args(["-c", "user.name=t", "-c", "user.email=t@t"])
        .args(args)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn git_available() -> bool {
    git(Path::new("."), &["--version"])
}

/// Canonical path with forward slashes and without the Windows verbatim prefix.
fn plain(path: &Path) -> String {
    let canonical = std::fs::canonicalize(path).expect("canonical");
    let text = canonical.to_string_lossy().replace('\\', "/");
    text.strip_prefix("//?/").unwrap_or(&text).to_string()
}

/// A git repository `name` under `root`, and its work tree path as git reports it.
fn repo(root: &Path, name: &str) -> String {
    let path = root.join(name);
    std::fs::create_dir_all(path.join("src")).expect("dir");
    assert!(git(&path, &["init", "-q"]));
    assert!(git(&path, &["commit", "-q", "--allow-empty", "-m", "init"]));
    plain(&path)
}

/// One prompt, a file-tool call per path (`Edit` with a result, or `Read`) and a text answer.
fn turn_lines(uuid: &str, edits: &[&str], reads: &[&str]) -> Vec<Value> {
    let mut blocks = vec![json!({ "type": "text", "text": "feito" })];
    let mut results = Vec::new();
    for (index, path) in edits.iter().enumerate() {
        let id = format!("{uuid}-e{index}");
        blocks.push(
            json!({ "type": "tool_use", "id": id, "name": "Edit", "input": { "file_path": path } }),
        );
        results.push(json!({ "type": "user", "uuid": format!("{id}-r"),
            "toolUseResult": { "filePath": path,
                "structuredPatch": [{ "oldStart": 1, "oldLines": 1, "newStart": 1, "newLines": 1, "lines": ["-a", "+b"] }] },
            "message": { "content": [{ "type": "tool_result", "tool_use_id": id }] } }));
    }
    for (index, path) in reads.iter().enumerate() {
        blocks.push(
            json!({ "type": "tool_use", "id": format!("{uuid}-r{index}"), "name": "Read",
            "input": { "file_path": path } }),
        );
    }
    let mut lines = vec![
        json!({ "type": "user", "uuid": uuid, "message": { "content": format!("tarefa {uuid}") } }),
        json!({ "type": "assistant", "uuid": format!("{uuid}-a"), "message": { "content": blocks } }),
    ];
    lines.extend(results);
    lines
}

fn write_lines(path: &Path, lines: &[Value]) {
    let text: Vec<String> = lines.iter().map(Value::to_string).collect();
    std::fs::write(path, text.join("\n") + "\n").expect("transcript");
}

fn session_event(transcript: &Path, cwd: &Path, name: &str) -> Value {
    json!({ "session_id": "sess-roots", "transcript_path": transcript, "cwd": cwd,
        "hook_event_name": name, "stop_hook_active": false })
}

/// JSON bodies of the recorded requests.
fn bodies(seen: &Arc<Mutex<Vec<String>>>) -> Vec<Value> {
    seen.lock()
        .expect("lock")
        .iter()
        .map(|request| {
            serde_json::from_str(request.split_once("\r\n\r\n").expect("body").1).expect("json")
        })
        .collect()
}

/// Files of the diff artifacts of one envelope body.
fn diff_files(body: &Value) -> Vec<String> {
    body["artifacts"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|artifact| artifact["metadata"]["file"].as_str().map(str::to_string))
        .collect()
}

/// A scratch directory with a non-repository `cwd` folder inside it.
fn outside_cwd(tag: &str) -> (PathBuf, PathBuf) {
    let data_dir = temporary_directory(tag);
    let cwd = data_dir.join("agent");
    std::fs::create_dir_all(&cwd).expect("cwd");
    (data_dir, cwd)
}

#[test]
fn a_session_outside_a_repo_is_captured_for_the_repo_it_edited() {
    if !git_available() {
        return;
    }
    let (data_dir, cwd) = outside_cwd("root-a");
    let a = repo(&data_dir, "a");
    let transcript = data_dir.join("t.jsonl");
    write_lines(
        &transcript,
        &turn_lines("p1", &[&format!("{a}/src/x.rs")], &[]),
    );
    let (port, seen) = fake_api("200 OK", json!({}));
    write_runtime(&data_dir, port);

    hook(
        &data_dir,
        "session-end",
        &session_event(&transcript, &cwd, "SessionEnd"),
    );

    let posted = bodies(&seen);
    assert_eq!(posted.len(), 1);
    assert_eq!(posted[0]["project"]["canonical_path"], json!(a));
    assert_eq!(diff_files(&posted[0]), ["src/x.rs"]);
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn a_turn_in_two_repos_goes_to_the_one_edited_most_without_the_others_diff() {
    if !git_available() {
        return;
    }
    let (data_dir, cwd) = outside_cwd("root-b");
    let a = repo(&data_dir, "a");
    let b = repo(&data_dir, "b");
    let transcript = data_dir.join("t.jsonl");
    let (a1, a2, b1) = (
        format!("{a}/src/one.rs"),
        format!("{a}/src/two.rs"),
        format!("{b}/src/other.rs"),
    );
    write_lines(&transcript, &turn_lines("p1", &[&a1, &b1, &a2], &[]));
    let (port, seen) = fake_api("200 OK", json!({}));
    write_runtime(&data_dir, port);

    hook(
        &data_dir,
        "session-end",
        &session_event(&transcript, &cwd, "SessionEnd"),
    );

    let posted = bodies(&seen);
    assert_eq!(posted.len(), 1, "one envelope, of the dominant repo");
    assert_eq!(posted[0]["project"]["canonical_path"], json!(a));
    assert_eq!(diff_files(&posted[0]), ["src/one.rs", "src/two.rs"]);
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn reads_alone_pick_the_repo_and_no_files_keep_the_cwd() {
    if !git_available() {
        return;
    }
    let (data_dir, cwd) = outside_cwd("root-c");
    let a = repo(&data_dir, "a");
    let transcript = data_dir.join("t.jsonl");
    write_lines(
        &transcript,
        &turn_lines("p1", &[], &[&format!("{a}/src/x.rs")]),
    );
    let (port, seen) = fake_api("200 OK", json!({}));
    write_runtime(&data_dir, port);
    hook(
        &data_dir,
        "session-end",
        &session_event(&transcript, &cwd, "SessionEnd"),
    );
    assert_eq!(bodies(&seen)[0]["project"]["canonical_path"], json!(a));

    // No file tool at all: the cwd, as before.
    let bare = data_dir.join("bare.jsonl");
    write_lines(
        &bare,
        &[
            json!({ "type": "user", "uuid": "q1", "message": { "content": "uma pergunta" } }),
            json!({ "type": "assistant", "uuid": "q1-a", "message": { "content": [
                { "type": "tool_use", "id": "t", "name": "Bash", "input": { "command": "ls" } }] } }),
        ],
    );
    let mut event = session_event(&bare, &cwd, "SessionEnd");
    event["session_id"] = json!("sess-bare");
    hook(&data_dir, "session-end", &event);
    let posted = bodies(&seen);
    assert_eq!(posted.len(), 2);
    assert_eq!(
        posted[1]["project"]["canonical_path"],
        json!(cwd.to_string_lossy())
    );
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn a_cwd_inside_a_repo_stays_the_root_even_with_edits_elsewhere() {
    if !git_available() {
        return;
    }
    let data_dir = temporary_directory("root-d");
    let home = repo(&data_dir, "home");
    let other = repo(&data_dir, "other");
    let transcript = data_dir.join("t.jsonl");
    write_lines(
        &transcript,
        &turn_lines(
            "p1",
            &[
                &format!("{home}/src/mine.rs"),
                &format!("{other}/src/a.rs"),
                &format!("{other}/src/b.rs"),
            ],
            &[],
        ),
    );
    let (port, seen) = fake_api("200 OK", json!({}));
    write_runtime(&data_dir, port);

    hook(
        &data_dir,
        "session-end",
        &session_event(&transcript, Path::new(&home), "SessionEnd"),
    );

    let posted = bodies(&seen);
    assert_eq!(posted.len(), 1);
    assert_eq!(posted[0]["project"]["canonical_path"], json!(home));
    assert_eq!(diff_files(&posted[0]), ["src/mine.rs"]);
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn a_403_for_one_repo_does_not_stop_the_next_turn_of_another() {
    if !git_available() {
        return;
    }
    let (data_dir, cwd) = outside_cwd("root-e");
    let a = repo(&data_dir, "a");
    let b = repo(&data_dir, "b");
    let transcript = data_dir.join("t.jsonl");
    let mut lines = turn_lines("p1", &[&format!("{a}/src/x.rs")], &[]);
    lines.extend(turn_lines("p2", &[&format!("{b}/src/y.rs")], &[]));
    lines.extend(turn_lines("p3", &[&format!("{a}/src/z.rs")], &[]));
    write_lines(&transcript, &lines);
    let (port, seen) = fake_api_by(
        move |request| {
            if request.contains(&format!("\"canonical_path\":\"{a}\"")) {
                "403 Forbidden"
            } else {
                "200 OK"
            }
        },
        json!({}),
    );
    write_runtime(&data_dir, port);

    hook(
        &data_dir,
        "session-end",
        &session_event(&transcript, &cwd, "SessionEnd"),
    );

    let roots: Vec<Value> = bodies(&seen)
        .iter()
        .map(|body| body["project"]["canonical_path"].clone())
        .collect();
    // p1 (a) is refused, p2 (b) is accepted, p3 (a) is passed over without a POST.
    assert_eq!(roots.len(), 2, "{roots:?}");
    assert_eq!(roots[1], json!(b));
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn hook_prompt_outside_a_repo_asks_for_the_repo_of_the_files_read() {
    if !git_available() {
        return;
    }
    let (data_dir, cwd) = outside_cwd("root-f");
    let a = repo(&data_dir, "a");
    let transcript = data_dir.join("t.jsonl");
    write_lines(
        &transcript,
        &turn_lines(
            "p1",
            &[],
            &[&format!("{a}/src/x.rs"), &format!("{a}/src/y.rs")],
        ),
    );
    let (port, seen) = fake_api("200 OK", json!({ "context": "ctx" }));
    write_runtime(&data_dir, port);
    let mut event = session_event(&transcript, &cwd, "UserPromptSubmit");
    event["prompt"] = json!("segue");

    let output = hook(&data_dir, "prompt", &event);

    assert!(!output.stdout.is_empty());
    let posted = bodies(&seen);
    assert_eq!(posted[0]["canonical_path"], json!(a));
    assert_eq!(posted[0]["files"], json!(["src/y.rs", "src/x.rs"]));
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn with_the_app_closed_the_envelope_is_queued_for_the_repo_edited() {
    if !git_available() {
        return;
    }
    let (data_dir, cwd) = outside_cwd("root-g");
    let a = repo(&data_dir, "a");
    let transcript = data_dir.join("t.jsonl");
    write_lines(
        &transcript,
        &turn_lines("p1", &[&format!("{a}/src/x.rs")], &[]),
    );

    hook(
        &data_dir,
        "session-end",
        &session_event(&transcript, &cwd, "SessionEnd"),
    );

    let pending = data_dir.join("outbox").join("pending");
    let files: Vec<_> = std::fs::read_dir(&pending).expect("pending").collect();
    assert_eq!(files.len(), 1);
    let envelope: Value = serde_json::from_str(
        &std::fs::read_to_string(files[0].as_ref().expect("entry").path()).expect("file"),
    )
    .expect("json");
    assert_eq!(envelope["project"]["canonical_path"], json!(a));
    assert_eq!(diff_files(&envelope), ["src/x.rs"]);
    let _ = std::fs::remove_dir_all(&data_dir);
}

/// The `user_text` of an envelope body, or `None` when it has none.
fn user_text(body: &Value) -> Option<String> {
    body["artifacts"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|artifact| artifact["kind"] == "user_text")
        .and_then(|artifact| artifact["content"].as_str().map(str::to_string))
}

/// A prompt line, an assistant answer of `answer` and no tools.
fn prompt_lines(uuid: &str, prompt: &Value, extra: Value, answer: &str) -> Vec<Value> {
    let mut entry = json!({ "type": "user", "uuid": uuid, "message": { "content": prompt } });
    if let (Some(entry), Some(extra)) = (entry.as_object_mut(), extra.as_object()) {
        entry.extend(extra.clone());
    }
    vec![
        entry,
        json!({ "type": "assistant", "uuid": format!("{uuid}-a"),
            "message": { "content": [{ "type": "text", "text": answer }] } }),
    ]
}

#[test]
fn a_notification_never_mixes_with_the_answer_folded_into_its_turn() {
    let data_dir = temporary_directory("speaker-a");
    let transcript = data_dir.join("t.jsonl");
    let mut lines = prompt_lines("p1", &json!("edite a"), json!({}), "feito");
    lines.extend(prompt_lines(
        "p2",
        &json!("<task-notification><task-id>a1</task-id><summary>concluido</summary></task-notification>"),
        json!({ "promptSource": "system" }),
        &"o subagente terminou e o resultado esta no relatorio. ".repeat(20),
    ));
    lines.extend(prompt_lines("p3", &json!("pode"), json!({}), "ok"));
    write_lines(&transcript, &lines);
    let (port, seen) = fake_api("202 Accepted", json!({ "capture_id": "c" }));
    write_runtime(&data_dir, port);
    let event = json!({ "session_id": "sess-1", "transcript_path": transcript,
        "cwd": "C:/proj", "hook_event_name": "SessionEnd", "reason": "other" });

    hook(&data_dir, "session-end", &event);

    let posted = bodies(&seen);
    assert_eq!(posted.len(), 2, "p3 folds into the notification's turn");
    assert_eq!(user_text(&posted[0]).as_deref(), Some("edite a"));
    assert_eq!(user_text(&posted[1]).as_deref(), Some("pode"));
    assert!(!posted[1].to_string().contains("task-notification"));
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn a_teammate_message_keeps_its_diff_and_loses_its_user_text() {
    if !git_available() {
        return;
    }
    let data_dir = temporary_directory("speaker-b");
    let a = repo(&data_dir, "a");
    let transcript = data_dir.join("t.jsonl");
    let mut lines = turn_lines("p1", &[&format!("{a}/src/x.rs")], &[]);
    lines[0] = json!({ "type": "user", "uuid": "p1", "message": { "content":
        "Message from OpenBot teammate Agente A: deixe o v2 de lado a partir de agora" } });
    write_lines(&transcript, &lines);
    let (port, seen) = fake_api("200 OK", json!({}));
    write_runtime(&data_dir, port);

    hook(
        &data_dir,
        "session-end",
        &session_event(&transcript, Path::new(&a), "SessionEnd"),
    );

    let posted = bodies(&seen);
    assert_eq!(posted.len(), 1);
    assert_eq!(user_text(&posted[0]), None);
    assert_eq!(diff_files(&posted[0]), ["src/x.rs"]);
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn a_wake_is_captured_as_the_human_words() {
    let data_dir = temporary_directory("speaker-c");
    let transcript = data_dir.join("t.jsonl");
    let wake = "<wake reason=\"message\"><message id=\"00000000-0000-0000-0000-000000000001\" \
                from=\"human\" at=\"2026-01-01T00:00:00Z\">esquece o redis, vamos de sqlite</message></wake>";
    write_lines(
        &transcript,
        &prompt_lines("p1", &json!(wake), json!({}), "entendido"),
    );
    let (port, seen) = fake_api("202 Accepted", json!({ "capture_id": "c" }));
    write_runtime(&data_dir, port);
    let event = json!({ "session_id": "sess-1", "transcript_path": transcript,
        "cwd": "C:/proj", "hook_event_name": "SessionEnd", "reason": "other" });

    hook(&data_dir, "session-end", &event);

    let posted = bodies(&seen);
    assert_eq!(posted.len(), 1);
    assert_eq!(
        user_text(&posted[0]).as_deref(),
        Some("esquece o redis, vamos de sqlite")
    );
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn a_cwd_in_a_subfolder_of_a_repo_roots_at_the_toplevel() {
    if !git_available() {
        return;
    }
    let data_dir = temporary_directory("root-top");
    let a = repo(&data_dir, "a");
    let sub = format!("{a}/src");
    let transcript = data_dir.join("t.jsonl");
    write_lines(
        &transcript,
        &turn_lines("p1", &[&format!("{a}/src/x.rs")], &[]),
    );
    let (port, seen) = fake_api("200 OK", json!({ "context": "ctx" }));
    write_runtime(&data_dir, port);

    hook(
        &data_dir,
        "session-end",
        &session_event(&transcript, Path::new(&sub), "SessionEnd"),
    );
    let mut event = session_event(&transcript, Path::new(&sub), "UserPromptSubmit");
    event["prompt"] = json!("segue");
    hook(&data_dir, "prompt", &event);

    let posted = bodies(&seen);
    assert_eq!(posted.len(), 2);
    assert_eq!(posted[0]["project"]["canonical_path"], json!(a));
    assert_eq!(diff_files(&posted[0]), ["src/x.rs"]);
    assert_eq!(posted[1]["canonical_path"], json!(a));
    let _ = std::fs::remove_dir_all(&data_dir);
}
