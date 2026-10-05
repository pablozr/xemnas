//! End to end: `xemnas-mcp hook prompt|stop` against a fake local API.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

fn temporary_directory(tag: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!("xemnas-hook-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(directory.join("state")).expect("state dir");
    directory
}

/// Answers every request with `status` and `body`, recording the raw requests.
fn fake_api(status: &'static str, body: Value) -> (u16, Arc<Mutex<Vec<String>>>) {
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
            recorder
                .lock()
                .expect("lock")
                .push(String::from_utf8_lossy(&request).to_string());
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
    hook(&data_dir, "prompt", &event);
    let mut times: Vec<Duration> = (0..10)
        .map(|_| {
            let started = Instant::now();
            let output = hook(&data_dir, "prompt", &event);
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
fn hook_stop_posts_one_envelope_per_turn_with_the_idempotency_key() {
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
    assert_eq!(requests.len(), 2);
    for (request, prompt) in requests.iter().zip(["p1", "p2"]) {
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
    assert_eq!(
        seen.lock().expect("lock").len(),
        1,
        "one POST, then short-circuit"
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

    hook(&data_dir, "stop", &event);
    assert_eq!(seen.lock().expect("lock").len(), 1, "nothing new to send");
    let _ = std::fs::remove_dir_all(&data_dir);
}
