//! End-to-end ingest tests over a real loopback server and a real `SqliteStore`.
//!
//! The server binds `127.0.0.1:0` on its own runtime thread and is driven with a
//! minimal blocking HTTP client, so the tests exercise the actual socket, auth
//! middleware, body limit and timeout rather than only the router.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use application::captures::{
    CaptureApi, CaptureError, CaptureIngest, CaptureReceiptRecord, CaptureRepository, CaptureWrite,
    IngestError, IngestOutcome, Receipt,
};
use application::jobs::{JobRepository, JobState, Jobs, ANALYZE_CAPTURE_KIND};
use application::projects::{ProjectError, ProjectRecord, ProjectRepository, Projects};
use integration_contracts::capture::{artifact_fingerprint, CaptureEnvelope};
use local_api::{ApiServer, ApiServerConfig, DiscoveryInfo, RunningApi};
use rusqlite::OptionalExtension;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use storage_sqlite::SqliteStore;

fn temporary_directory(tag: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let directory = std::env::temp_dir().join(format!(
        "xemnas-local-api-{tag}-{}-{nanos}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir_all(&directory).expect("create temporary directory");
    directory
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("two levels above the crate")
        .to_path_buf()
}

fn fixture(name: &str) -> Value {
    let path = workspace_root()
        .join("tests")
        .join("fixtures")
        .join("capture")
        .join(name);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|error| panic!("parse {}: {error}", path.display()))
}

fn register_project(store: &SqliteStore, root: &Path) -> String {
    let directory = root.join("project");
    std::fs::create_dir_all(&directory).expect("create project dir");
    let project = Projects::new(store.clone())
        .register(&directory)
        .expect("register project");
    project.location().to_string()
}

fn envelope_for(location: &str) -> Value {
    let mut value = fixture("valid-complete.json");
    value["project"]["canonical_path"] = json!(location);
    value
}

fn envelope_key(value: &Value) -> String {
    value["idempotency_key"].as_str().expect("key").to_string()
}

fn body_of(value: &Value) -> Vec<u8> {
    serde_json::to_vec(value).expect("serialize envelope")
}

fn sha256_hex(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn start(
    store: &SqliteStore,
    max_body_bytes: usize,
    request_timeout: Duration,
    runtime_dir: PathBuf,
) -> RunningApi {
    let api: Arc<dyn CaptureApi> = Arc::new(CaptureIngest::new(store.clone()));
    start_with(api, max_body_bytes, request_timeout, runtime_dir)
}

fn start_with(
    api: Arc<dyn CaptureApi>,
    max_body_bytes: usize,
    request_timeout: Duration,
    runtime_dir: PathBuf,
) -> RunningApi {
    let mut config = ApiServerConfig::new(api, runtime_dir);
    config.max_body_bytes = max_body_bytes;
    config.request_timeout = request_timeout;
    ApiServer::start(config).expect("start local api")
}

// --- raw HTTP client -------------------------------------------------------

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn decode_chunked(raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut index = 0;
    while let Some(line_end) = find_subsequence(&raw[index..], b"\r\n") {
        let line_end = index + line_end;
        let size_text = String::from_utf8_lossy(&raw[index..line_end]);
        let size = usize::from_str_radix(size_text.split(';').next().unwrap_or("0").trim(), 16)
            .unwrap_or(0);
        index = line_end + 2;
        if size == 0 {
            break;
        }
        out.extend_from_slice(&raw[index..index + size]);
        index += size + 2;
    }
    out
}

fn raw_request(
    port: u16,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: Option<&[u8]>,
) -> (u16, Vec<u8>) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .expect("set read timeout");
    let mut request =
        format!("{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n");
    for (name, value) in headers {
        request.push_str(name);
        request.push_str(": ");
        request.push_str(value);
        request.push_str("\r\n");
    }
    if let Some(body) = body {
        request.push_str(&format!("Content-Length: {}\r\n", body.len()));
    }
    request.push_str("\r\n");
    stream.write_all(request.as_bytes()).expect("write head");
    if let Some(body) = body {
        stream.write_all(body).expect("write body");
    }
    stream.flush().expect("flush");

    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).expect("read response");
    let split = find_subsequence(&raw, b"\r\n\r\n").expect("header terminator");
    let head = String::from_utf8_lossy(&raw[..split]).to_string();
    let status = head
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|token| token.parse::<u16>().ok())
        .expect("status code");
    let mut body = raw[split + 4..].to_vec();
    if head
        .to_ascii_lowercase()
        .contains("transfer-encoding: chunked")
    {
        body = decode_chunked(&body);
    }
    (status, body)
}

fn get(
    server: &RunningApi,
    path: &str,
    token: Option<&str>,
    origin: Option<&str>,
) -> (u16, Vec<u8>) {
    let mut headers: Vec<(&str, String)> = Vec::new();
    if let Some(token) = token {
        headers.push(("Authorization", format!("Bearer {token}")));
    }
    if let Some(origin) = origin {
        headers.push(("Origin", origin.to_string()));
    }
    let pairs: Vec<(&str, &str)> = headers.iter().map(|(k, v)| (*k, v.as_str())).collect();
    raw_request(server.address().port(), "GET", path, &pairs, None)
}

fn post(
    server: &RunningApi,
    path: &str,
    token: Option<&str>,
    idempotency_key: Option<&str>,
    origin: Option<&str>,
    body: &[u8],
) -> (u16, Vec<u8>) {
    let mut headers: Vec<(&str, String)> = vec![("Content-Type", "application/json".to_string())];
    if let Some(token) = token {
        headers.push(("Authorization", format!("Bearer {token}")));
    }
    if let Some(key) = idempotency_key {
        headers.push(("Idempotency-Key", key.to_string()));
    }
    if let Some(origin) = origin {
        headers.push(("Origin", origin.to_string()));
    }
    let pairs: Vec<(&str, &str)> = headers.iter().map(|(k, v)| (*k, v.as_str())).collect();
    raw_request(server.address().port(), "POST", path, &pairs, Some(body))
}

fn table_count(connection: &rusqlite::Connection, table: &str) -> i64 {
    connection
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .expect("count")
}

/// Reads `(message_id, capture_id, observed_at)` for an adapter session.
fn checkpoint_row(
    connection: &rusqlite::Connection,
    session_id: &str,
) -> Option<(String, String, String)> {
    connection
        .query_row(
            "SELECT message_id, capture_id, observed_at FROM adapter_checkpoints \
             WHERE adapter = 'opencode' AND session_id = ?1",
            [session_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .expect("query checkpoint")
}

// --- log capture -----------------------------------------------------------

#[derive(Clone, Default)]
struct Buffer(Arc<Mutex<Vec<u8>>>);

struct BufferWriter(Arc<Mutex<Vec<u8>>>);

impl Write for BufferWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for Buffer {
    type Writer = BufferWriter;
    fn make_writer(&'a self) -> Self::Writer {
        BufferWriter(self.0.clone())
    }
}

/// Process-wide captured log buffer, installed once for the whole test binary.
static LOG_BUFFER: std::sync::OnceLock<Buffer> = std::sync::OnceLock::new();

fn capture_logs() -> &'static Buffer {
    LOG_BUFFER.get_or_init(|| {
        let buffer = Buffer::default();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(buffer.clone())
            .with_ansi(false)
            .finish();
        let _ = tracing::subscriber::set_global_default(subscriber);
        buffer
    })
}

fn logged_text() -> String {
    String::from_utf8(
        capture_logs()
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone(),
    )
    .expect("utf8")
}

// --- tests -----------------------------------------------------------------

#[test]
fn bind_is_loopback_and_discovery_files_match() {
    let root = temporary_directory("discovery");
    let store = SqliteStore::open(root.join("app.db")).expect("open store");
    let server = start(
        &store,
        local_api::DEFAULT_MAX_BODY_BYTES,
        Duration::from_secs(5),
        root.clone(),
    );

    assert!(
        server.address().ip().is_loopback(),
        "must bind loopback only"
    );
    let discovery_text =
        std::fs::read_to_string(root.join(local_api::DISCOVERY_FILE)).expect("discovery file");
    let discovery: DiscoveryInfo = serde_json::from_str(&discovery_text).expect("parse");
    assert_eq!(discovery.port, server.address().port());
    assert_eq!(discovery.protocol_version, local_api::PROTOCOL_VERSION);
    assert!(!discovery.instance_id.is_empty());
    assert_eq!(&discovery, server.discovery());
    assert_eq!(
        std::fs::read_to_string(root.join(local_api::TOKEN_FILE)).expect("token file"),
        server.token()
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn missing_or_wrong_token_is_unauthorized_and_never_logged() {
    let root = temporary_directory("auth");
    let store = SqliteStore::open(root.join("app.db")).expect("open store");
    let server = start(
        &store,
        local_api::DEFAULT_MAX_BODY_BYTES,
        Duration::from_secs(5),
        root.clone(),
    );
    let buffer = capture_logs();

    assert_eq!(get(&server, "/v1/health", None, None).0, 401);
    assert_eq!(get(&server, "/v1/health", Some("wrong-token"), None).0, 401);
    assert_eq!(
        get(&server, "/v1/health", Some(server.token()), None).0,
        200
    );

    let logged = String::from_utf8(
        buffer
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone(),
    )
    .expect("utf8");
    assert!(
        !logged.contains(server.token()),
        "the bearer token must never reach a log: {logged}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn origin_header_is_forbidden_even_with_a_valid_token() {
    let root = temporary_directory("origin");
    let store = SqliteStore::open(root.join("app.db")).expect("open store");
    let server = start(
        &store,
        local_api::DEFAULT_MAX_BODY_BYTES,
        Duration::from_secs(5),
        root.clone(),
    );

    let (status, _) = get(
        &server,
        "/v1/health",
        Some(server.token()),
        Some("https://evil.example"),
    );
    assert_eq!(status, 403);

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn valid_capture_is_persisted_and_scheduled() {
    let root = temporary_directory("ingest");
    let database = root.join("app.db");
    let store = SqliteStore::open(&database).expect("open store");
    let location = register_project(&store, &root);
    let server = start(
        &store,
        local_api::DEFAULT_MAX_BODY_BYTES,
        Duration::from_secs(5),
        root.clone(),
    );

    let envelope = envelope_for(&location);
    let key = envelope_key(&envelope);
    let (status, body) = post(
        &server,
        "/v1/captures",
        Some(server.token()),
        Some(&key),
        None,
        &body_of(&envelope),
    );
    assert_eq!(status, 201, "body: {}", String::from_utf8_lossy(&body));
    let receipt: Value = serde_json::from_slice(&body).expect("receipt");
    assert_eq!(receipt["capture_id"], envelope["capture_id"]);
    assert_eq!(receipt["idempotency_key"], json!(key));
    assert_eq!(receipt["artifact_count"], json!(4));
    assert!(receipt["received_at"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));

    let connection = rusqlite::Connection::open(&database).expect("open raw connection");
    assert_eq!(table_count(&connection, "capture_receipts"), 1);
    assert_eq!(table_count(&connection, "capture_artifacts"), 4);
    assert_eq!(table_count(&connection, "jobs"), 1);

    let job_kind: String = connection
        .query_row("SELECT kind FROM jobs", [], |row| row.get(0))
        .expect("job kind");
    assert_eq!(job_kind, ANALYZE_CAPTURE_KIND);

    let mut statement = connection
        .prepare("SELECT content, fingerprint FROM capture_artifacts")
        .expect("prepare");
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .expect("query")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect");
    for (content, fingerprint) in rows {
        assert_eq!(fingerprint, sha256_hex(&content));
    }

    // The adapter checkpoint is written in the same transaction and matches the
    // accepted envelope's source coordinates.
    assert_eq!(table_count(&connection, "adapter_checkpoints"), 1);
    let checkpoint = checkpoint_row(&connection, "session-synthetic-complete")
        .expect("checkpoint for the accepted session");
    assert_eq!(checkpoint.0, "message-synthetic-complete");
    assert_eq!(
        checkpoint.1,
        envelope["capture_id"].as_str().expect("capture id")
    );
    assert_eq!(
        checkpoint.2,
        envelope["observed_at"].as_str().expect("observed at")
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn replay_returns_200_without_duplicating() {
    let root = temporary_directory("replay");
    let database = root.join("app.db");
    let store = SqliteStore::open(&database).expect("open store");
    let location = register_project(&store, &root);
    let server = start(
        &store,
        local_api::DEFAULT_MAX_BODY_BYTES,
        Duration::from_secs(5),
        root.clone(),
    );

    let envelope = envelope_for(&location);
    let key = envelope_key(&envelope);
    let body = body_of(&envelope);
    let (first_status, first_body) = post(
        &server,
        "/v1/captures",
        Some(server.token()),
        Some(&key),
        None,
        &body,
    );
    let (second_status, second_body) = post(
        &server,
        "/v1/captures",
        Some(server.token()),
        Some(&key),
        None,
        &body,
    );
    assert_eq!(first_status, 201);
    assert_eq!(second_status, 200);
    assert_eq!(first_body, second_body);

    let connection = rusqlite::Connection::open(&database).expect("open raw connection");
    assert_eq!(table_count(&connection, "capture_receipts"), 1);
    assert_eq!(table_count(&connection, "capture_artifacts"), 4);
    assert_eq!(table_count(&connection, "jobs"), 1);
    assert_eq!(table_count(&connection, "adapter_checkpoints"), 1);
    let checkpoint = checkpoint_row(&connection, "session-synthetic-complete")
        .expect("checkpoint remains after replay");
    assert_eq!(
        checkpoint.1,
        envelope["capture_id"].as_str().expect("capture id")
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn idempotency_key_header_is_required_and_must_match() {
    let root = temporary_directory("idempotency");
    let store = SqliteStore::open(root.join("app.db")).expect("open store");
    let location = register_project(&store, &root);
    let server = start(
        &store,
        local_api::DEFAULT_MAX_BODY_BYTES,
        Duration::from_secs(5),
        root.clone(),
    );

    let envelope = envelope_for(&location);
    let body = body_of(&envelope);
    assert_eq!(
        post(
            &server,
            "/v1/captures",
            Some(server.token()),
            None,
            None,
            &body
        )
        .0,
        400
    );
    assert_eq!(
        post(
            &server,
            "/v1/captures",
            Some(server.token()),
            Some("a-different-key"),
            None,
            &body,
        )
        .0,
        400
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn malformed_json_is_400_and_invalid_envelopes_are_422() {
    let root = temporary_directory("validation");
    let store = SqliteStore::open(root.join("app.db")).expect("open store");
    let server = start(
        &store,
        local_api::DEFAULT_MAX_BODY_BYTES,
        Duration::from_secs(5),
        root.clone(),
    );

    let (malformed, _) = post(
        &server,
        "/v1/captures",
        Some(server.token()),
        Some("anything"),
        None,
        b"{ not json",
    );
    assert_eq!(malformed, 400);

    for name in [
        "invalid-extra-field.json",
        "incompatible-schema-version-2.json",
    ] {
        let value = fixture(name);
        let key = envelope_key(&value);
        let (status, body) = post(
            &server,
            "/v1/captures",
            Some(server.token()),
            Some(&key),
            None,
            &body_of(&value),
        );
        assert_eq!(status, 422, "{name}: {}", String::from_utf8_lossy(&body));
    }

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn unregistered_or_uncanonicalizable_project_is_forbidden() {
    let root = temporary_directory("forbidden");
    let store = SqliteStore::open(root.join("app.db")).expect("open store");
    let server = start(
        &store,
        local_api::DEFAULT_MAX_BODY_BYTES,
        Duration::from_secs(5),
        root.clone(),
    );

    let unregistered = fixture("valid-complete.json");
    let key = envelope_key(&unregistered);
    assert_eq!(
        post(
            &server,
            "/v1/captures",
            Some(server.token()),
            Some(&key),
            None,
            &body_of(&unregistered),
        )
        .0,
        403
    );

    let mut missing = fixture("valid-complete.json");
    missing["project"]["canonical_path"] = json!("Z:/does/not/exist");
    let key = envelope_key(&missing);
    assert_eq!(
        post(
            &server,
            "/v1/captures",
            Some(server.token()),
            Some(&key),
            None,
            &body_of(&missing),
        )
        .0,
        403
    );

    let connection = rusqlite::Connection::open(root.join("app.db")).expect("open raw connection");
    assert_eq!(table_count(&connection, "capture_receipts"), 0);
    assert_eq!(table_count(&connection, "adapter_checkpoints"), 0);

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn oversized_body_is_rejected() {
    let root = temporary_directory("oversized");
    let store = SqliteStore::open(root.join("app.db")).expect("open store");
    let server = start(&store, 256, Duration::from_secs(5), root.clone());

    let oversized = vec![b'a'; 4096];
    let (status, _) = post(
        &server,
        "/v1/captures",
        Some(server.token()),
        Some("key"),
        None,
        &oversized,
    );
    assert_eq!(status, 413);

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn duplicate_artifact_id_is_conflict_and_rolls_back() {
    let root = temporary_directory("conflict");
    let database = root.join("app.db");
    let store = SqliteStore::open(&database).expect("open store");
    let location = register_project(&store, &root);
    let server = start(
        &store,
        local_api::DEFAULT_MAX_BODY_BYTES,
        Duration::from_secs(5),
        root.clone(),
    );

    let mut envelope = envelope_for(&location);
    let first = envelope["artifacts"][0].clone();
    envelope["artifacts"]
        .as_array_mut()
        .expect("artifacts")
        .push(first);
    let key = envelope_key(&envelope);
    let (status, _) = post(
        &server,
        "/v1/captures",
        Some(server.token()),
        Some(&key),
        None,
        &body_of(&envelope),
    );
    assert_eq!(status, 409);

    let connection = rusqlite::Connection::open(&database).expect("open raw connection");
    assert_eq!(table_count(&connection, "capture_receipts"), 0);
    assert_eq!(table_count(&connection, "capture_artifacts"), 0);
    assert_eq!(table_count(&connection, "jobs"), 0);

    let _ = std::fs::remove_dir_all(&root);
}

/// A use case that always blocks, to exercise the timeout deterministically.
struct SlowApi {
    delay: Duration,
}

impl CaptureApi for SlowApi {
    fn ingest(
        &self,
        _envelope: &CaptureEnvelope,
        _idempotency_key: &str,
    ) -> Result<IngestOutcome, IngestError> {
        std::thread::sleep(self.delay);
        Err(IngestError::Storage("slow".to_string()))
    }

    fn receipt(&self, _capture_id: &str) -> Result<Receipt, IngestError> {
        Err(IngestError::NotFound)
    }
}

#[test]
fn slow_use_case_times_out_with_gateway_timeout() {
    let root = temporary_directory("timeout");
    let api: Arc<dyn CaptureApi> = Arc::new(SlowApi {
        delay: Duration::from_millis(500),
    });
    let server = start_with(
        api,
        local_api::DEFAULT_MAX_BODY_BYTES,
        Duration::from_millis(50),
        root.clone(),
    );

    let envelope = fixture("valid-complete.json");
    let key = envelope_key(&envelope);
    let started = Instant::now();
    let (status, _) = post(
        &server,
        "/v1/captures",
        Some(server.token()),
        Some(&key),
        None,
        &body_of(&envelope),
    );
    let elapsed = started.elapsed();
    assert_eq!(status, 504);
    assert!(
        elapsed < Duration::from_millis(400),
        "timeout was not prompt: {elapsed:?}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn health_capabilities_and_receipt_lookup_shapes() {
    let root = temporary_directory("shapes");
    let store = SqliteStore::open(root.join("app.db")).expect("open store");
    let location = register_project(&store, &root);
    let server = start(
        &store,
        local_api::DEFAULT_MAX_BODY_BYTES,
        Duration::from_secs(5),
        root.clone(),
    );

    let (status, body) = get(&server, "/v1/health", Some(server.token()), None);
    assert_eq!(status, 200);
    let health: Value = serde_json::from_slice(&body).expect("health");
    assert_eq!(health["status"], json!("ok"));
    assert_eq!(
        health["protocol_version"],
        json!(local_api::PROTOCOL_VERSION)
    );
    assert!(health["instance_id"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));

    let (status, body) = get(&server, "/v1/capabilities", Some(server.token()), None);
    assert_eq!(status, 200);
    let capabilities: Value = serde_json::from_slice(&body).expect("capabilities");
    assert_eq!(capabilities["capture_schema_versions"], json!([1]));
    assert_eq!(
        capabilities["max_body_bytes"],
        json!(local_api::DEFAULT_MAX_BODY_BYTES)
    );

    let envelope = envelope_for(&location);
    let key = envelope_key(&envelope);
    let (status, body) = post(
        &server,
        "/v1/captures",
        Some(server.token()),
        Some(&key),
        None,
        &body_of(&envelope),
    );
    assert_eq!(status, 201);
    let receipt: Value = serde_json::from_slice(&body).expect("receipt");
    let capture_id = receipt["capture_id"]
        .as_str()
        .expect("capture id")
        .to_string();

    assert_eq!(
        get(
            &server,
            &format!("/v1/captures/{capture_id}"),
            Some(server.token()),
            None,
        )
        .0,
        200
    );
    assert_eq!(
        get(
            &server,
            "/v1/captures/018f2d3c-4b5a-7c6d-8e9f-0000000000ff",
            Some(server.token()),
            None,
        )
        .0,
        404
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn restart_keeps_receipt_and_job_and_recover_leaves_queued_alone() {
    let root = temporary_directory("restart");
    let database = root.join("app.db");
    let location;
    {
        let store = SqliteStore::open(&database).expect("open store");
        location = register_project(&store, &root);
        let envelope = envelope_for(&location);
        let ingest = CaptureIngest::new(store.clone());
        let value: CaptureEnvelope = serde_json::from_value(envelope.clone()).expect("deserialize");
        let outcome = ingest
            .ingest(&value, &envelope_key(&envelope))
            .expect("ingest");
        assert!(!outcome.replayed);
    }

    let reopened = SqliteStore::open(&database).expect("reopen store");
    let ingest = CaptureIngest::new(reopened.clone());
    let value: CaptureEnvelope =
        serde_json::from_value(envelope_for(&location)).expect("deserialize");
    let stored = ingest.receipt(&value.capture_id).expect("receipt");
    assert_eq!(stored.artifact_count, 4);

    let jobs = Jobs::new(reopened.clone());
    let report = jobs.recover().expect("recover");
    assert_eq!(report.requeued, 0);
    assert_eq!(report.failed, 0);
    let job = JobRepository::list(&reopened)
        .expect("list jobs")
        .into_iter()
        .find(|record| record.kind == ANALYZE_CAPTURE_KIND)
        .expect("analysis job");
    assert_eq!(job.state, JobState::Queued);

    let _ = std::fs::remove_dir_all(&root);
}

/// A repository that removes the project between the use-case pre-check and the
/// transactional write, simulating a UI removal inside that window.
struct RemovingProjectRepository {
    store: SqliteStore,
    project_id: String,
}

impl ProjectRepository for RemovingProjectRepository {
    fn insert(&self, record: &ProjectRecord) -> Result<(), ProjectError> {
        ProjectRepository::insert(&self.store, record)
    }
    fn list(&self) -> Result<Vec<ProjectRecord>, ProjectError> {
        ProjectRepository::list(&self.store)
    }
    fn get(&self, id: &str) -> Result<Option<ProjectRecord>, ProjectError> {
        ProjectRepository::get(&self.store, id)
    }
    fn find_by_location(&self, location: &str) -> Result<Option<ProjectRecord>, ProjectError> {
        ProjectRepository::find_by_location(&self.store, location)
    }
    fn remove(&self, id: &str) -> Result<bool, ProjectError> {
        ProjectRepository::remove(&self.store, id)
    }
}

impl CaptureRepository for RemovingProjectRepository {
    fn insert_capture(&self, write: &CaptureWrite) -> Result<(), CaptureError> {
        let _ = ProjectRepository::remove(&self.store, &self.project_id);
        // `SqliteStore` implements both `CaptureRepository` and `JobRepository`,
        // which both declare `insert`; the call is disambiguated explicitly.
        CaptureRepository::insert_capture(&self.store, write)
    }
    fn find_receipt(&self, capture_id: &str) -> Result<Option<CaptureReceiptRecord>, CaptureError> {
        self.store.find_receipt(capture_id)
    }
    fn find_receipt_by_idempotency_key(
        &self,
        idempotency_key: &str,
    ) -> Result<Option<CaptureReceiptRecord>, CaptureError> {
        self.store.find_receipt_by_idempotency_key(idempotency_key)
    }
}

#[test]
fn project_removed_between_check_and_insert_is_forbidden() {
    let root = temporary_directory("toctou");
    let database = root.join("app.db");
    let store = SqliteStore::open(&database).expect("open store");
    let directory = root.join("project");
    std::fs::create_dir_all(&directory).expect("create project dir");
    let project = Projects::new(store.clone())
        .register(&directory)
        .expect("register project");
    let project_id = project.id().as_str().to_string();
    let location = project.location().to_string();

    let repository = RemovingProjectRepository {
        store: store.clone(),
        project_id,
    };
    let ingest = CaptureIngest::new(repository);
    let value = envelope_for(&location);
    let envelope: CaptureEnvelope = serde_json::from_value(value.clone()).expect("deserialize");
    assert_eq!(
        ingest.ingest(&envelope, &envelope_key(&value)),
        Err(IngestError::Forbidden)
    );

    let connection = rusqlite::Connection::open(&database).expect("open raw connection");
    assert_eq!(table_count(&connection, "capture_receipts"), 0);
    assert_eq!(table_count(&connection, "capture_artifacts"), 0);
    assert_eq!(table_count(&connection, "jobs"), 0);

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn stalled_request_body_times_out() {
    let root = temporary_directory("stall");
    let api: Arc<dyn CaptureApi> = Arc::new(SlowApi {
        delay: Duration::from_millis(0),
    });
    let server = start_with(
        api,
        local_api::DEFAULT_MAX_BODY_BYTES,
        Duration::from_millis(300),
        root.clone(),
    );

    let mut stream = TcpStream::connect(("127.0.0.1", server.address().port())).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .expect("read timeout");
    let head = format!(
        "POST /v1/captures HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nConnection: close\r\n\
         Authorization: Bearer {}\r\nContent-Type: application/json\r\n\
         Idempotency-Key: stall\r\nContent-Length: 1000\r\n\r\n",
        server.address().port(),
        server.token()
    );
    stream.write_all(head.as_bytes()).expect("write head");
    stream
        .write_all(b"{\"partial\":")
        .expect("write partial body");
    stream.flush().expect("flush");

    let started = Instant::now();
    let mut raw = Vec::new();
    let _ = stream.read_to_end(&mut raw);
    let elapsed = started.elapsed();
    assert!(
        elapsed < Duration::from_secs(2),
        "a stalled request body must be cut off within ~2s: {elapsed:?}"
    );
    if let Some(split) = find_subsequence(&raw, b"\r\n\r\n") {
        let head = String::from_utf8_lossy(&raw[..split]);
        let status = head
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|token| token.parse::<u16>().ok());
        assert_eq!(status, Some(504), "unexpected response: {head}");
    }
    // An empty read means the server closed the connection instead of replying;
    // the brief accepts either, and the elapsed assertion already holds.

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn schema_failure_never_logs_client_content() {
    let root = temporary_directory("schema-log");
    let store = SqliteStore::open(root.join("app.db")).expect("open store");
    let server = start(
        &store,
        local_api::DEFAULT_MAX_BODY_BYTES,
        Duration::from_secs(5),
        root.clone(),
    );
    let _buffer = capture_logs();

    let marker = "SECRET_MARKER_XYZ";
    let mut value = fixture("valid-minimal.json");
    // Schema-invalid and carrying the marker in value positions.
    value["observed_at"] = json!(marker);
    value["idempotency_key"] = json!(marker);
    let (status, _) = post(
        &server,
        "/v1/captures",
        Some(server.token()),
        Some(marker),
        None,
        &body_of(&value),
    );
    assert_eq!(status, 422);

    let logged = logged_text();
    assert!(
        logged.contains("validate_capture"),
        "the validation failure should have been logged"
    );
    assert!(
        !logged.contains(marker),
        "schema failure leaked client content: {logged}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn tampered_fingerprint_is_unprocessable_and_not_persisted() {
    let root = temporary_directory("tampered");
    let database = root.join("app.db");
    let store = SqliteStore::open(&database).expect("open store");
    let location = register_project(&store, &root);
    let server = start(
        &store,
        local_api::DEFAULT_MAX_BODY_BYTES,
        Duration::from_secs(5),
        root.clone(),
    );

    let mut envelope = envelope_for(&location);
    envelope["artifacts"][0]["fingerprint"] = json!("0".repeat(64));
    let key = envelope_key(&envelope);
    let (status, _) = post(
        &server,
        "/v1/captures",
        Some(server.token()),
        Some(&key),
        None,
        &body_of(&envelope),
    );
    assert_eq!(status, 422);

    let connection = rusqlite::Connection::open(&database).expect("open raw connection");
    assert_eq!(table_count(&connection, "capture_receipts"), 0);
    assert_eq!(table_count(&connection, "capture_artifacts"), 0);
    assert_eq!(table_count(&connection, "jobs"), 0);
    assert_eq!(table_count(&connection, "adapter_checkpoints"), 0);

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn same_declared_fingerprint_for_different_contents_is_unprocessable() {
    let root = temporary_directory("fingerprint-dup");
    let database = root.join("app.db");
    let store = SqliteStore::open(&database).expect("open store");
    let location = register_project(&store, &root);
    let server = start(
        &store,
        local_api::DEFAULT_MAX_BODY_BYTES,
        Duration::from_secs(5),
        root.clone(),
    );

    let mut envelope = fixture("valid-minimal.json");
    envelope["project"]["canonical_path"] = json!(location);
    let first = envelope["artifacts"][0].clone();
    let declared = first["fingerprint"].clone();
    let mut second = first.clone();
    second["artifact_id"] = json!("018f2d3c-4b5a-7c6d-8e9f-000000000102");
    second["content"] = json!("a different synthetic content");
    second["fingerprint"] = declared;
    envelope["artifacts"]
        .as_array_mut()
        .expect("artifacts")
        .push(second);

    // Sanity: the first artifact is honest, so only the second can fail.
    assert_eq!(
        artifact_fingerprint("synthetic user text"),
        first["fingerprint"].as_str().expect("fingerprint")
    );

    let key = envelope_key(&envelope);
    let (status, _) = post(
        &server,
        "/v1/captures",
        Some(server.token()),
        Some(&key),
        None,
        &body_of(&envelope),
    );
    assert_eq!(status, 422);

    let connection = rusqlite::Connection::open(&database).expect("open raw connection");
    assert_eq!(table_count(&connection, "capture_receipts"), 0);
    assert_eq!(table_count(&connection, "capture_artifacts"), 0);
    assert_eq!(table_count(&connection, "jobs"), 0);

    let _ = std::fs::remove_dir_all(&root);
}
