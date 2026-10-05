//! End to end: the real binary over stdio, talking to a fake local API.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

fn temporary_directory(tag: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!("xemnas-mcp-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(directory.join("state")).expect("state dir");
    directory
}

/// Answers every request with `body` and records the raw requests.
fn fake_api(body: Value) -> (u16, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let recorder = seen.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { break };
            let mut buffer = [0u8; 8192];
            let read = stream.read(&mut buffer).unwrap_or(0);
            recorder
                .lock()
                .expect("lock")
                .push(String::from_utf8_lossy(&buffer[..read]).to_string());
            let payload = body.to_string();
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
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

/// Runs the binary with `messages` on stdin and returns every response line.
fn run(data_dir: &Path, project: &str, messages: &[Value]) -> Vec<Value> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_xemnas-mcp"))
        .args(["--project", project])
        .env("XEMNAS_DATA_DIR", data_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn");
    {
        let mut stdin = child.stdin.take().expect("stdin");
        for message in messages {
            writeln!(stdin, "{message}").expect("write");
        }
        writeln!(stdin, "not json").expect("write");
    }
    let stdout = child.stdout.take().expect("stdout");
    let lines: Vec<Value> = BufReader::new(stdout)
        .lines()
        .map(|line| serde_json::from_str(&line.expect("line")).expect("json response"))
        .collect();
    assert!(child.wait().expect("exit").success());
    lines
}

#[test]
fn a_session_initializes_lists_tools_and_opens_a_decision() {
    let data_dir = temporary_directory("session");
    let (port, seen) = fake_api(json!({ "text": "pergunta: Qual banco?" }));
    write_runtime(&data_dir, port);

    let responses = run(
        &data_dir,
        "C:/projeto",
        &[
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": { "protocolVersion": "2025-06-18" } }),
            json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
            json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }),
            json!({ "jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": { "name": "get_decision", "arguments": { "reference": "D:bbbbcccc" } } }),
        ],
    );

    assert_eq!(responses.len(), 4, "the notification gets no response");
    assert_eq!(
        responses[0]["result"]["serverInfo"]["name"],
        json!("xemnas")
    );
    assert_eq!(
        responses[1]["result"]["tools"].as_array().map(Vec::len),
        Some(3),
        "get_decision, search_context and file_context"
    );
    assert_eq!(
        responses[2]["result"]["content"][0]["text"],
        json!("pergunta: Qual banco?")
    );
    assert_eq!(responses[3]["error"]["code"], json!(-32700));

    let requests = seen.lock().expect("lock");
    let request = requests.first().expect("one API call");
    assert!(request.starts_with("POST /v1/agent/decision "));
    assert!(request
        .to_ascii_lowercase()
        .contains("authorization: bearer tok-123"));
    assert!(request.contains("\"reference\":\"D:bbbbcccc\""));
    assert!(request.contains("\"canonical_path\":\"C:/projeto\""));
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn a_closed_app_is_a_friendly_tool_error() {
    let data_dir = temporary_directory("closed");
    let responses = run(
        &data_dir,
        "C:/projeto",
        &[
            json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": "search_context", "arguments": { "query": "banco" } } }),
        ],
    );
    assert_eq!(responses[0]["result"]["isError"], json!(true));
    assert!(responses[0]["result"]["content"][0]["text"]
        .as_str()
        .is_some_and(|text| text.contains("fechado")));
    let _ = std::fs::remove_dir_all(&data_dir);
}
