//! AI provider tests: consent gating, request shape, strict output validation
//! and sanitized failures, all against a minimal loopback HTTP server.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use ai_provider::{test_connection, OpenAiCompatibleExtractor};
use application::extract::{
    run_extraction, AssessmentRecord, AssessmentStore, CandidateExtractor, DecisionCandidateRecord,
    DecisionEvidence, EvidenceArtifact, ExtractError, ExtractionStore, RelevanceSignal, RunContext,
};
use application::profile::{build_preview, grant_consent, AiProfile, ProfileKind, SecretStore};

/// Injectable sleep used to keep the retry tests deterministic.
type SleepFn = Arc<dyn Fn(Duration) + Send + Sync>;

/// Minimal loopback server that answers every request with a fixed response.
struct MockServer {
    port: u16,
    requests: Arc<AtomicUsize>,
    last_request: Arc<Mutex<String>>,
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn content_length(headers: &str) -> usize {
    headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            if name.eq_ignore_ascii_case("content-length") {
                value.trim().parse().ok()
            } else {
                None
            }
        })
        .unwrap_or(0)
}

fn start_server(status_line: &'static str, body: Vec<u8>) -> MockServer {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind loopback");
    let port = listener.local_addr().expect("addr").port();
    let requests = Arc::new(AtomicUsize::new(0));
    let last_request = Arc::new(Mutex::new(String::new()));
    let requests_thread = requests.clone();
    let last_thread = last_request.clone();

    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { break };
            let mut collected = Vec::new();
            let mut buffer = [0u8; 4096];
            loop {
                let read = stream.read(&mut buffer).unwrap_or(0);
                if read == 0 {
                    break;
                }
                collected.extend_from_slice(&buffer[..read]);
                if let Some(position) = find_subsequence(&collected, b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&collected[..position]).to_string();
                    let body_start = position + 4;
                    let expected = body_start + content_length(&headers);
                    while collected.len() < expected {
                        let read = stream.read(&mut buffer).unwrap_or(0);
                        if read == 0 {
                            break;
                        }
                        collected.extend_from_slice(&buffer[..read]);
                    }
                    break;
                }
            }
            requests_thread.fetch_add(1, Ordering::SeqCst);
            *last_thread.lock().expect("lock") = String::from_utf8_lossy(&collected).to_string();
            let header = format!(
                "{status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(header.as_bytes());
            let _ = stream.write_all(&body);
            let _ = stream.flush();
        }
    });

    MockServer {
        port,
        requests,
        last_request,
    }
}

/// Reads one HTTP request fully from the socket and returns its raw text.
fn read_http_request(stream: &mut std::net::TcpStream) -> String {
    let mut collected = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        let read = stream.read(&mut buffer).unwrap_or(0);
        if read == 0 {
            break;
        }
        collected.extend_from_slice(&buffer[..read]);
        if let Some(position) = find_subsequence(&collected, b"\r\n\r\n") {
            let headers = String::from_utf8_lossy(&collected[..position]).to_string();
            let body_start = position + 4;
            let expected = body_start + content_length(&headers);
            while collected.len() < expected {
                let read = stream.read(&mut buffer).unwrap_or(0);
                if read == 0 {
                    break;
                }
                collected.extend_from_slice(&buffer[..read]);
            }
            break;
        }
    }
    String::from_utf8_lossy(&collected).to_string()
}

/// Answers with `responses` in order, repeating the last one for extra requests.
fn start_sequence_server(responses: Vec<(&'static str, Vec<u8>)>) -> MockServer {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind loopback");
    let port = listener.local_addr().expect("addr").port();
    let requests = Arc::new(AtomicUsize::new(0));
    let last_request = Arc::new(Mutex::new(String::new()));
    let requests_thread = requests.clone();
    let last_thread = last_request.clone();

    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { break };
            let request = read_http_request(&mut stream);
            let index = requests_thread.fetch_add(1, Ordering::SeqCst);
            *last_thread.lock().expect("lock") = request;
            let (status_line, body) = {
                let position = index.min(responses.len().saturating_sub(1));
                responses
                    .get(position)
                    .map(|(status, body)| (*status, body.clone()))
                    .unwrap_or(("HTTP/1.1 500 Internal Server Error", Vec::new()))
            };
            let header = format!(
                "{status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(header.as_bytes());
            let _ = stream.write_all(&body);
            let _ = stream.flush();
        }
    });

    MockServer {
        port,
        requests,
        last_request,
    }
}

/// The extractor only needs this classification helper in tests.
fn no_sleep() -> SleepFn {
    Arc::new(|_: Duration| {})
}

/// A sleep replacement that records each delay and never blocks.
fn recording_sleep() -> (SleepFn, Arc<Mutex<Vec<Duration>>>) {
    let delays = Arc::new(Mutex::new(Vec::new()));
    let sink = delays.clone();
    let sleep: SleepFn = Arc::new(move |delay| {
        sink.lock().expect("lock").push(delay);
    });
    (sleep, delays)
}

fn endpoint(port: u16) -> String {
    format!("http://127.0.0.1:{port}/v1")
}

fn base_profile(port: u16, max_input_chars: usize) -> AiProfile {
    AiProfile {
        id: "profile-test".to_string(),
        kind: ProfileKind::OpenAiCompatible,
        model: "gpt-test".to_string(),
        endpoint: Some(endpoint(port)),
        max_input_chars,
        external_calls_enabled: false,
        consent: None,
        chatgpt: None,
    }
}

/// A profile with a real, config-bound consent (hash of this exact preview).
fn granted_profile(port: u16, max_input_chars: usize) -> AiProfile {
    let profile = base_profile(port, max_input_chars);
    let preview = build_preview(&profile);
    grant_consent(&profile, &preview, "2026-01-01T00:00:00Z", true).expect("grant consent")
}

/// Strong-signal diff so `filter_relevant` actually dispatches the extractor.
const SCHEMA_DIFF: &str =
    "diff --git a/src/schema.rs b/src/schema.rs\n+CREATE TABLE t (id TEXT);\n";

fn evidence(content: &str) -> DecisionEvidence {
    DecisionEvidence {
        capture_id: "capture-1".to_string(),
        project_id: "project-1".to_string(),
        adapter: Some("opencode".to_string()),
        session_id: Some("session-1".to_string()),
        observed_at: Some("2026-01-01T00:00:00Z".to_string()),
        artifacts: vec![EvidenceArtifact {
            artifact_id: "artifact-1".to_string(),
            kind: "diff_hunk".to_string(),
            content: content.to_string(),
            metadata: "{}".to_string(),
        }],
    }
}

/// The strict JSON the model is asked to embed in `message.content`.
fn model_content(artifact_count: usize) -> String {
    serde_json::json!({
        "proposals": [{
            "question": "q",
            "choice": "c",
            "rationale": "r",
            "confidence": 0.7,
            "confidence_reason": "x",
            "evidence_refs": ["artifact-1"],
            "diff_summary": { "files": [], "artifacts": artifact_count }
        }]
    })
    .to_string()
}

/// A realistic OpenAI-compatible chat completion wrapping `content`.
fn chat_body(content: &str) -> Vec<u8> {
    serde_json::json!({
        "id": "chatcmpl-1",
        "object": "chat.completion",
        "created": 0,
        "model": "gpt-test",
        "choices": [{
            "index": 0,
            "message": { "role": "assistant", "content": content },
            "finish_reason": "stop"
        }]
    })
    .to_string()
    .into_bytes()
}

fn valid_body(artifact_count: usize) -> Vec<u8> {
    chat_body(&model_content(artifact_count))
}

/// Minimal in-memory extraction store.
struct TestStore {
    evidence: DecisionEvidence,
    records: Mutex<Vec<DecisionCandidateRecord>>,
}

impl TestStore {
    fn new(evidence: DecisionEvidence) -> Self {
        Self {
            evidence,
            records: Mutex::new(Vec::new()),
        }
    }

    fn record_count(&self) -> usize {
        self.records.lock().expect("lock").len()
    }
}

impl AssessmentStore for TestStore {
    fn record_assessment(&self, _row: &AssessmentRecord) -> Result<(), ExtractError> {
        Ok(())
    }
}

impl ExtractionStore for TestStore {
    fn load_evidence(&self, _capture_id: &str) -> Result<Option<DecisionEvidence>, ExtractError> {
        Ok(Some(self.evidence.clone()))
    }

    fn insert_candidates(
        &self,
        records: &[DecisionCandidateRecord],
    ) -> Result<usize, ExtractError> {
        let mut stored = self.records.lock().expect("lock");
        let mut inserted = 0;
        for record in records {
            if stored
                .iter()
                .any(|existing| existing.dedup_hash == record.dedup_hash)
            {
                continue;
            }
            stored.push(record.clone());
            inserted += 1;
        }
        Ok(inserted)
    }
}

#[test]
fn consent_disabled_makes_no_request() {
    let server = start_server("HTTP/1.1 200 OK", valid_body(1));
    let profile = base_profile(server.port, 64);
    let extractor =
        OpenAiCompatibleExtractor::new(&profile, "sk-synthetic".to_string()).expect("new");

    let result = extractor.extract(&evidence("content"), &[RelevanceSignal::PublicContract]);
    assert!(matches!(result, Err(ExtractError::Extractor(_))));
    assert_eq!(server.requests.load(Ordering::SeqCst), 0, "no network call");
}

#[test]
fn stale_consent_after_config_edit_makes_no_request() {
    let server = start_server("HTTP/1.1 200 OK", valid_body(1));
    let mut profile = granted_profile(server.port, 64);
    profile.model = "another-model".to_string();
    let extractor =
        OpenAiCompatibleExtractor::new(&profile, "sk-synthetic".to_string()).expect("new");

    let result = extractor.extract(&evidence(SCHEMA_DIFF), &[RelevanceSignal::PublicContract]);
    assert!(matches!(result, Err(ExtractError::Extractor(_))));
    assert_eq!(server.requests.load(Ordering::SeqCst), 0, "no network call");
}

#[test]
fn forged_consent_makes_no_request() {
    let server = start_server("HTTP/1.1 200 OK", valid_body(1));
    let mut profile = base_profile(server.port, 64);
    profile.external_calls_enabled = true;
    profile.consent = Some(application::profile::ConsentRecord {
        granted_at: "2026-01-01T00:00:00Z".to_string(),
        preview_hash: "forged".to_string(),
    });
    let extractor =
        OpenAiCompatibleExtractor::new(&profile, "sk-synthetic".to_string()).expect("new");

    let result = extractor.extract(&evidence(SCHEMA_DIFF), &[RelevanceSignal::PublicContract]);
    assert!(matches!(result, Err(ExtractError::Extractor(_))));
    assert_eq!(server.requests.load(Ordering::SeqCst), 0, "no network call");
}

#[test]
fn changed_endpoint_path_makes_no_request() {
    let server = start_server("HTTP/1.1 200 OK", valid_body(1));
    let mut profile = granted_profile(server.port, 64);
    profile.endpoint = Some(format!("http://127.0.0.1:{}/v2", server.port));
    let extractor = OpenAiCompatibleExtractor::new(&profile, "sk-synthetic".into()).expect("new");
    assert!(extractor
        .extract(&evidence(SCHEMA_DIFF), &[RelevanceSignal::PublicContract])
        .is_err());
    assert_eq!(server.requests.load(Ordering::SeqCst), 0);
}

#[test]
fn redirects_never_send_evidence_to_an_unapproved_destination() {
    let target = start_server("HTTP/1.1 200 OK", valid_body(1));
    let source = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
    let source_port = source.local_addr().expect("addr").port();
    let target_port = target.port;
    let server = std::thread::spawn(move || {
        let (mut stream, _) = source.accept().expect("request");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("timeout");
        let request = read_http_request(&mut stream);
        assert!(request.contains("sk-synthetic"));
        write!(stream, "HTTP/1.1 307 Temporary Redirect\r\nLocation: http://127.0.0.1:{target_port}/v1/chat/completions\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").expect("redirect");
    });
    let profile = granted_profile(source_port, 64);
    let extractor = OpenAiCompatibleExtractor::new(&profile, "sk-synthetic".into()).expect("new");
    assert!(extractor
        .extract(&evidence(SCHEMA_DIFF), &[RelevanceSignal::PublicContract])
        .is_err());
    server.join().expect("source server");
    assert_eq!(target.requests.load(Ordering::SeqCst), 0);
}

#[test]
fn happy_path_parses_persists_and_truncates() {
    let server = start_server("HTTP/1.1 200 OK", valid_body(1));
    let profile = granted_profile(server.port, 200);
    let extractor =
        OpenAiCompatibleExtractor::new(&profile, "sk-synthetic".to_string()).expect("new");

    let long_content = format!("{SCHEMA_DIFF}{}TAIL_MARKER", "A".repeat(150));
    let store = TestStore::new(evidence(&long_content));
    let report =
        run_extraction(&store, &extractor, "capture-1", &RunContext::for_tests()).expect("extract");

    assert_eq!(report.candidates, 1);
    assert_eq!(report.inserted, 1);
    assert_eq!(store.record_count(), 1);

    let request = server.last_request.lock().expect("lock").clone();
    let lower = request.to_lowercase();
    assert!(lower.contains("authorization: bearer sk-synthetic"));
    assert!(lower.contains("gpt-test"));
    assert!(lower.contains("json_object"));
    assert!(
        request.contains("schema.rs"),
        "the relevant content must be sent"
    );
    assert!(
        !request.contains("TAIL_MARKER"),
        "content must be truncated to max_input_chars"
    );
}

#[test]
fn hostile_output_is_rejected_and_writes_nothing() {
    let cases: Vec<(&str, String)> = vec![
        (
            "extra field inside content",
            r#"{"proposals":[{"question":"q","choice":"c","rationale":"r","confidence":0.7,"confidence_reason":"x","evidence_refs":["artifact-1"],"diff_summary":{"files":[],"artifacts":1},"extra":true}]}"#.to_string(),
        ),
        (
            "confidence out of range",
            r#"{"proposals":[{"question":"q","choice":"c","rationale":"r","confidence":1.5,"confidence_reason":"x","evidence_refs":["artifact-1"],"diff_summary":{"files":[],"artifacts":1}}]}"#.to_string(),
        ),
        (
            "empty file path",
            r#"{"proposals":[{"question":"q","choice":"c","rationale":"r","confidence":0.7,"confidence_reason":"x","evidence_refs":["artifact-1"],"diff_summary":{"files":[""],"artifacts":1}}]}"#.to_string(),
        ),
    ];

    for (name, inner) in cases {
        let body = chat_body(&inner);
        let server = start_server("HTTP/1.1 200 OK", body);
        let profile = granted_profile(server.port, 64);
        let extractor =
            OpenAiCompatibleExtractor::new(&profile, "sk-synthetic".to_string()).expect("new");
        let store = TestStore::new(evidence(SCHEMA_DIFF));

        let result = run_extraction(&store, &extractor, "capture-1", &RunContext::for_tests());
        assert!(result.is_err(), "{name}: must be rejected");
        assert_eq!(store.record_count(), 0, "{name}: no row may be written");
    }
}

#[test]
fn references_and_counts_are_reconciled_with_the_capture() {
    // The model often miscounts artifacts or invents ids; the app owns both.
    let cases = [
        r#"{"proposals":[{"question":"q","choice":"c","rationale":"r","confidence":0.7,"confidence_reason":"x","evidence_refs":["missing"],"diff_summary":{"files":[],"artifacts":1}}]}"#,
        r#"{"proposals":[{"question":"q","choice":"c","rationale":"r","confidence":0.7,"confidence_reason":"x","evidence_refs":["artifact-1"],"diff_summary":{"files":[],"artifacts":7}}]}"#,
    ];
    for inner in cases {
        let server = start_server("HTTP/1.1 200 OK", chat_body(inner));
        let profile = granted_profile(server.port, 64);
        let extractor =
            OpenAiCompatibleExtractor::new(&profile, "sk-synthetic".to_string()).expect("new");
        let store = TestStore::new(evidence(SCHEMA_DIFF));
        run_extraction(&store, &extractor, "capture-1", &RunContext::for_tests())
            .expect("reconciled");
        assert_eq!(store.record_count(), 1);
    }
}

#[test]
fn malformed_chat_envelopes_are_rejected() {
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("empty body object", b"{}".to_vec()),
        ("empty choices", br#"{"choices":[]}"#.to_vec()),
        (
            "missing content",
            br#"{"choices":[{"message":{"role":"assistant"}}]}"#.to_vec(),
        ),
        (
            "null content",
            br#"{"choices":[{"message":{"role":"assistant","content":null}}]}"#.to_vec(),
        ),
        (
            "content is not json",
            br#"{"choices":[{"message":{"role":"assistant","content":"not json"}}]}"#.to_vec(),
        ),
        (
            "content lacks the proposals contract",
            br#"{"choices":[{"message":{"role":"assistant","content":"{}"}}]}"#.to_vec(),
        ),
    ];

    for (name, body) in cases {
        let server = start_server("HTTP/1.1 200 OK", body);
        let profile = granted_profile(server.port, 64);
        let extractor =
            OpenAiCompatibleExtractor::new(&profile, "sk-synthetic".to_string()).expect("new");
        let store = TestStore::new(evidence(SCHEMA_DIFF));

        let result = run_extraction(&store, &extractor, "capture-1", &RunContext::for_tests());
        let error = result.expect_err("must be rejected");
        assert_eq!(
            store.record_count(),
            0,
            "{name}: no candidate may be written"
        );
        let rendered = error.to_string();
        assert!(
            !rendered.contains("content") && !rendered.contains("choices"),
            "{name}: error must be sanitized: {rendered}"
        );
    }
}

#[test]
fn transient_503s_are_retried_then_succeed() {
    let server = start_sequence_server(vec![
        ("HTTP/1.1 503 Service Unavailable", b"try later".to_vec()),
        ("HTTP/1.1 503 Service Unavailable", b"try later".to_vec()),
        ("HTTP/1.1 200 OK", valid_body(1)),
    ]);
    let profile = granted_profile(server.port, 64);
    let extractor = OpenAiCompatibleExtractor::new(&profile, "sk-synthetic".to_string())
        .expect("new")
        .with_sleep(no_sleep());

    let proposals = extractor
        .extract(&evidence(SCHEMA_DIFF), &[RelevanceSignal::PublicContract])
        .expect("the third attempt succeeds");
    assert_eq!(proposals.len(), 1);
    assert_eq!(
        server.requests.load(Ordering::SeqCst),
        3,
        "exactly three attempts"
    );
}

#[test]
fn transient_failures_exhaust_attempts_with_a_sanitized_error() {
    let server = start_sequence_server(vec![(
        "HTTP/1.1 503 Service Unavailable",
        b"provider secret body".to_vec(),
    )]);
    let profile = granted_profile(server.port, 64);
    let extractor = OpenAiCompatibleExtractor::new(&profile, "synthetic-key".to_string())
        .expect("new")
        .with_sleep(no_sleep());

    let error = extractor
        .extract(&evidence(SCHEMA_DIFF), &[RelevanceSignal::PublicContract])
        .expect_err("every attempt fails");
    assert_eq!(server.requests.load(Ordering::SeqCst), 3);
    let rendered = error.to_string();
    assert!(
        rendered.contains("tentativa"),
        "the attempt count must be reported: {rendered}"
    );
    assert!(!rendered.contains("provider secret body"));
    assert!(!rendered.contains("synthetic-key"));
}

#[test]
fn client_error_is_not_retried() {
    let server = start_sequence_server(vec![("HTTP/1.1 400 Bad Request", b"bad request".to_vec())]);
    let profile = granted_profile(server.port, 64);
    let extractor = OpenAiCompatibleExtractor::new(&profile, "sk-synthetic".to_string())
        .expect("new")
        .with_sleep(no_sleep());

    let result = extractor.extract(&evidence(SCHEMA_DIFF), &[RelevanceSignal::PublicContract]);
    assert!(result.is_err());
    assert_eq!(
        server.requests.load(Ordering::SeqCst),
        1,
        "a 4xx must fail on the first attempt"
    );
}

#[test]
fn backoff_delays_grow_exponentially_without_sleeping() {
    let server = start_sequence_server(vec![(
        "HTTP/1.1 503 Service Unavailable",
        b"try later".to_vec(),
    )]);
    let profile = granted_profile(server.port, 64);
    let (sleep, delays) = recording_sleep();
    let extractor = OpenAiCompatibleExtractor::new(&profile, "sk-synthetic".to_string())
        .expect("new")
        .with_sleep(sleep);

    let _ = extractor.extract(&evidence(SCHEMA_DIFF), &[RelevanceSignal::PublicContract]);
    assert_eq!(
        *delays.lock().expect("lock"),
        vec![Duration::from_millis(500), Duration::from_millis(1000)],
        "the backoff must be exponential and bounded"
    );
}

#[test]
fn refused_connection_is_transient_and_retried() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
    let port = listener.local_addr().expect("addr").port();
    drop(listener);

    let profile = granted_profile(port, 64);
    let (sleep, delays) = recording_sleep();
    let extractor = OpenAiCompatibleExtractor::new(&profile, "sk-synthetic".to_string())
        .expect("new")
        .with_sleep(sleep);

    let result = extractor.extract(&evidence(SCHEMA_DIFF), &[RelevanceSignal::PublicContract]);
    assert!(result.is_err());
    assert_eq!(
        delays.lock().expect("lock").len(),
        2,
        "a refused connection must exhaust the three attempts"
    );
}

#[test]
fn empty_secret_is_rejected_at_construction_unless_on_loopback() {
    let server = start_server("HTTP/1.1 200 OK", valid_body(1));
    let local = granted_profile(server.port, 64);
    assert!(
        OpenAiCompatibleExtractor::new(&local, String::new()).is_ok(),
        "a local model runs without a key (ADR-0004)"
    );
    let profile = AiProfile {
        endpoint: Some("https://api.example.test/v1".to_string()),
        ..local
    };
    assert!(matches!(
        OpenAiCompatibleExtractor::new(&profile, String::new()),
        Err(ExtractError::Extractor(_))
    ));
    assert!(matches!(
        OpenAiCompatibleExtractor::new(&profile, "   ".to_string()),
        Err(ExtractError::Extractor(_))
    ));
}

#[test]
fn oversized_response_and_http_errors_are_sanitized() {
    let oversized = vec![b'X'; 600 * 1024];
    let server = start_server("HTTP/1.1 200 OK", oversized);
    let profile = granted_profile(server.port, 64);
    let extractor = OpenAiCompatibleExtractor::new(&profile, "synthetic-key".to_string())
        .expect("new")
        .with_sleep(no_sleep());
    let error = extractor
        .extract(&evidence("content"), &[RelevanceSignal::PublicContract])
        .expect_err("oversized response must fail");
    let rendered = error.to_string();
    assert!(!rendered.contains('X'), "body must not leak: {rendered}");
    assert!(!rendered.contains("synthetic-key"));

    for status in [
        "HTTP/1.1 429 Too Many Requests",
        "HTTP/1.1 500 Internal Server Error",
    ] {
        let server = start_server(status, b"provider secret body".to_vec());
        let profile = granted_profile(server.port, 64);
        let extractor = OpenAiCompatibleExtractor::new(&profile, "synthetic-key".to_string())
            .expect("new")
            .with_sleep(no_sleep());
        let error = extractor
            .extract(&evidence("content"), &[RelevanceSignal::PublicContract])
            .expect_err("http error must fail");
        let rendered = error.to_string();
        assert!(
            !rendered.contains("provider secret body"),
            "error must not carry the body: {rendered}"
        );
        assert!(!rendered.contains("synthetic-key"));
    }
}

#[test]
#[ignore = "touches the real OS keychain; run explicitly as evidence"]
fn keyring_roundtrip_on_the_real_os_store() {
    use ai_provider::KeyringSecretStore;

    let store = KeyringSecretStore::new();
    let account = format!("xemnas-test-{}", uuid_like());
    store
        .set_secret(&account, "synthetic-key")
        .expect("set secret");
    let fetched = store.get_secret(&account).expect("get secret");
    assert_eq!(fetched.as_deref(), Some("synthetic-key"));
    store.delete_secret(&account).expect("delete secret");
    assert_eq!(store.get_secret(&account).expect("get after delete"), None);
}

/// Small unique suffix for the keyring test account, without a new dependency.
fn uuid_like() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("{}-{nanos}", std::process::id())
}

/// Strict model answer that cites the synthetic connection-test artifacts.
fn connection_test_body(evidence_ref: &str) -> Vec<u8> {
    chat_body(
        &serde_json::json!({
            "proposals": [{
                "question": "Onde persistir a fila de exemplo?",
                "choice": "SQLite",
                "rationale": "Aplicativo local.",
                "confidence": 0.8,
                "confidence_reason": "Troca explícita.",
                "evidence_refs": [evidence_ref],
                "diff_summary": { "files": [], "artifacts": 2 }
            }]
        })
        .to_string(),
    )
}

#[test]
fn connection_test_sends_only_synthetic_evidence_and_validates() {
    let server = start_server(
        "HTTP/1.1 200 OK",
        connection_test_body("00000000-0000-7000-8000-000000000001"),
    );
    let profile = granted_profile(server.port, 4_000);
    let report = test_connection(&profile, "sk-synthetic".to_string()).expect("connection test");
    assert_eq!(report.proposals, 1);
    assert_eq!(server.requests.load(Ordering::SeqCst), 1);
    let request = server.last_request.lock().expect("lock").clone();
    assert!(
        request.contains("fila de exemplo"),
        "the request carries the synthetic evidence"
    );
}

#[test]
fn connection_test_without_consent_makes_no_request() {
    let server = start_server(
        "HTTP/1.1 200 OK",
        connection_test_body("00000000-0000-7000-8000-000000000001"),
    );
    let profile = base_profile(server.port, 4_000);
    let result = test_connection(&profile, "sk-synthetic".to_string());
    assert!(matches!(result, Err(ExtractError::Extractor(_))));
    assert_eq!(server.requests.load(Ordering::SeqCst), 0, "no network call");
}

#[test]
fn connection_test_flags_an_answer_that_breaks_the_contract() {
    let server = start_server("HTTP/1.1 200 OK", connection_test_body("not-an-artifact"));
    let profile = granted_profile(server.port, 4_000);
    let result = test_connection(&profile, "sk-synthetic".to_string());
    assert_eq!(result.map_err(|error| error.code()), Err("validation"));
}

#[test]
fn a_429_is_a_pause_with_its_retry_after_and_is_not_retried_in_the_call() {
    let server = start_server(
        "HTTP/1.1 429 Too Many Requests\r\nRetry-After: 7",
        b"slow down".to_vec(),
    );
    let profile = granted_profile(server.port, 64);
    let (sleep, delays) = recording_sleep();
    let extractor = OpenAiCompatibleExtractor::new(&profile, "sk-synthetic".to_string())
        .expect("new")
        .with_sleep(sleep);

    let error = extractor
        .extract(&evidence(SCHEMA_DIFF), &[RelevanceSignal::PublicContract])
        .expect_err("a 429 does not succeed");
    assert_eq!(
        error,
        ExtractError::RateLimited {
            retry_after: Some(Duration::from_secs(7))
        }
    );
    assert!(error.is_deferred(), "the job goes back to the queue");
    assert_eq!(server.requests.load(Ordering::SeqCst), 1);
    assert!(
        delays.lock().expect("lock").is_empty(),
        "no sleep inside the call"
    );
}

#[test]
fn timeouts_and_5xx_end_as_unavailable_so_the_job_is_requeued() {
    let server = start_sequence_server(vec![(
        "HTTP/1.1 503 Service Unavailable",
        b"try later".to_vec(),
    )]);
    let profile = granted_profile(server.port, 64);
    let extractor = OpenAiCompatibleExtractor::new(&profile, "sk-synthetic".to_string())
        .expect("new")
        .with_sleep(no_sleep());
    let error = extractor
        .extract(&evidence(SCHEMA_DIFF), &[RelevanceSignal::PublicContract])
        .expect_err("every attempt fails");
    assert_eq!(error, ExtractError::Unavailable { attempts: 3 });
    assert!(error.is_deferred());
}

#[test]
fn a_429_pauses_every_call_behind_the_shared_limiter() {
    use ai_provider::{ChatGptSession, ProviderFactory};
    use application::analysis::ExtractorFactory;
    use application::limiter::ProviderLimiter;

    let server = start_server("HTTP/1.1 429 Too Many Requests", b"slow down".to_vec());
    let profile = granted_profile(server.port, 64);
    let limiter = ProviderLimiter::new(2);
    let factory =
        ProviderFactory::new(Arc::new(ChatGptSession::default())).with_limiter(limiter.clone());
    let first = factory
        .external(&profile, "sk-synthetic".to_string())
        .expect("extractor");
    let error = first
        .extract(&evidence(SCHEMA_DIFF), &[RelevanceSignal::PublicContract])
        .expect_err("rate limited");
    assert!(matches!(error, ExtractError::RateLimited { .. }));
    assert_eq!(limiter.active(), 0, "the slot is released");

    let second = factory
        .external(&profile, "sk-synthetic".to_string())
        .expect("extractor");
    match second.extract(&evidence(SCHEMA_DIFF), &[RelevanceSignal::PublicContract]) {
        Err(ExtractError::RateLimited {
            retry_after: Some(left),
        }) => assert!(left <= Duration::from_secs(20)),
        other => panic!("expected the shared pause, got {other:?}"),
    }
    assert_eq!(
        server.requests.load(Ordering::SeqCst),
        1,
        "nobody calls the provider during the pause"
    );
}
