//! Settings → OpenCode: integration status and connection test (MVP-SPEC §8).
//!
//! [`Integration::status`] reports what the adapter needs to reach the desktop
//! and what it has delivered so far: the local API endpoint, the adapters and
//! versions seen in accepted captures, the last checkpoint, the outbox
//! location and counts, and the contract version this build accepts.
//! [`Integration::check`] is the connection test: a fixed list of local checks,
//! each with a stable kind, an outcome and an actionable product message.
//!
//! Everything here is local and read-only. The checks inspect file metadata
//! and the discovery file only; the bearer token is **never read**, and no
//! session or message identifier, capture content or project path leaves this
//! module (PRIV-001). Both methods touch SQLite and the filesystem, so callers
//! on the UI must run them off the UI thread (ASYNC-001).

use std::fs;
use std::path::{Path, PathBuf};

use integration_contracts::capture::CAPTURE_ENVELOPE_SCHEMA_VERSION;
use serde::{Deserialize, Serialize};

/// Name of the discovery file the local API writes into the runtime directory.
pub const DISCOVERY_FILE: &str = "discovery.json";

/// Name of the per-session token file the local API writes beside it.
pub const TOKEN_FILE: &str = "api-token";

/// Failure modes of the integration use case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntegrationError {
    /// A storage query failed; the message is diagnostic only.
    Storage(String),
}

impl IntegrationError {
    /// Returns a short, stable code.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Storage(_) => "storage",
        }
    }
}

impl std::fmt::Display for IntegrationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Storage(message) => {
                write!(formatter, "falha ao consultar a integração: {message}")
            }
        }
    }
}

impl std::error::Error for IntegrationError {}

/// Per-adapter activity aggregated from the adapter checkpoints.
///
/// Carries no session or message identifier: only counts, the most recent
/// adapter version and timestamps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterActivityRow {
    /// Adapter name, for example `opencode`.
    pub adapter: String,
    /// Version reported by the most recently updated checkpoint, when recorded.
    pub adapter_version: Option<String>,
    /// Distinct sessions with at least one accepted capture.
    pub sessions: i64,
    /// RFC 3339 observation time of the most recent accepted capture.
    pub last_observed_at: String,
    /// RFC 3339 time the most recent capture was accepted.
    pub last_received_at: String,
}

/// Persistence port the integration use case needs.
pub trait IntegrationStore {
    /// Activity per adapter, most recently updated first.
    fn adapter_activity(&self) -> Result<Vec<AdapterActivityRow>, IntegrationError>;
}

/// The loopback endpoint the composition root started, if any.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalApiEndpoint {
    /// Bound loopback port.
    pub port: u16,
    /// Protocol version advertised in the discovery file.
    pub protocol_version: u32,
}

/// Local paths and runtime facts only the composition root knows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationEnvironment {
    /// Data directory (`XEMNAS_DATA_DIR` or `%LOCALAPPDATA%\xemnas`).
    pub data_dir: PathBuf,
    /// Outbox root the adapter writes to while the desktop is closed.
    pub outbox_dir: PathBuf,
    /// Directory holding the discovery and token files.
    pub runtime_dir: PathBuf,
    /// The running local API, or `None` when it failed to start.
    pub api: Option<LocalApiEndpoint>,
}

/// Overall state of the integration, from the desktop's point of view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntegrationState {
    /// The local API is not running: the adapter falls back to the outbox.
    ApiUnavailable,
    /// The API is running but no capture was ever accepted.
    AwaitingFirstCapture,
    /// The API is running and captures have been accepted.
    Receiving,
}

impl IntegrationState {
    /// Returns the persisted/serialized literal.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ApiUnavailable => "api_unavailable",
            Self::AwaitingFirstCapture => "awaiting_first_capture",
            Self::Receiving => "receiving",
        }
    }
}

/// Whether an adapter's captures match the contract this build accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Compatibility {
    /// Its captures were accepted under the current contract version.
    Compatible,
    /// Captures were accepted before versions were recorded; nothing is known
    /// about the adapter build.
    Unknown,
}

/// One adapter as shown in Settings → OpenCode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterStatus {
    /// Adapter name.
    pub adapter: String,
    /// Most recent adapter version, when recorded.
    pub adapter_version: Option<String>,
    /// Compatibility with [`IntegrationStatus::contract_version`].
    pub compatibility: Compatibility,
    /// Distinct sessions captured.
    pub sessions: i64,
    /// RFC 3339 observation time of the last checkpoint.
    pub last_observed_at: String,
    /// RFC 3339 time the last checkpoint was accepted.
    pub last_received_at: String,
}

/// Outbox location and file counts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboxStatus {
    /// Outbox root.
    pub path: PathBuf,
    /// Whether the root exists yet (the adapter creates it on first use).
    pub exists: bool,
    /// Items waiting in `pending/` (imported on the next start).
    pub pending: i64,
    /// Items archived in `accepted/`.
    pub accepted: i64,
    /// Safe diagnostics in `rejected/`.
    pub rejected: i64,
}

/// Settings → OpenCode snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntegrationStatus {
    /// Overall state.
    pub state: IntegrationState,
    /// Capture Envelope contract version this build accepts.
    pub contract_version: u32,
    /// The local API endpoint, when running.
    pub api: Option<LocalApiEndpoint>,
    /// Adapters seen in accepted captures, most recent first.
    pub adapters: Vec<AdapterStatus>,
    /// Outbox location and counts.
    pub outbox: OutboxStatus,
    /// Data directory, for "open folder" actions.
    pub data_dir: PathBuf,
}

/// Which local condition a connection check verifies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckKind {
    /// The loopback API is running.
    LocalApi,
    /// The discovery file points at this running instance.
    Discovery,
    /// The token file exists and is not empty.
    Token,
    /// The outbox has nothing stuck or rejected.
    Outbox,
    /// At least one capture has been accepted.
    Captures,
}

/// Outcome of one connection check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckOutcome {
    /// Condition satisfied.
    Ok,
    /// Works, but needs attention.
    Warning,
    /// The adapter cannot reach the desktop through this path.
    Failed,
    /// Not applicable because a prerequisite failed.
    Skipped,
}

/// One connection check with an actionable, content-free product message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntegrationCheck {
    /// What was checked.
    pub kind: CheckKind,
    /// Result.
    pub outcome: CheckOutcome,
    /// Product-language message built from fixed text, counts and timestamps.
    pub message: String,
}

/// Settings → OpenCode use case over an [`IntegrationStore`].
#[derive(Debug, Clone)]
pub struct Integration<S> {
    store: S,
    environment: IntegrationEnvironment,
}

impl<S: IntegrationStore> Integration<S> {
    /// Wraps the store and the composition-root environment.
    pub fn new(store: S, environment: IntegrationEnvironment) -> Self {
        Self { store, environment }
    }

    /// Builds the integration snapshot.
    ///
    /// # Errors
    ///
    /// [`IntegrationError::Storage`] when the checkpoint query fails. Missing
    /// directories are reported as zero counts, never as errors.
    pub fn status(&self) -> Result<IntegrationStatus, IntegrationError> {
        let adapters: Vec<AdapterStatus> = self
            .store
            .adapter_activity()?
            .into_iter()
            .map(|row| AdapterStatus {
                compatibility: if row.adapter_version.is_some() {
                    Compatibility::Compatible
                } else {
                    Compatibility::Unknown
                },
                adapter: row.adapter,
                adapter_version: row.adapter_version,
                sessions: row.sessions,
                last_observed_at: row.last_observed_at,
                last_received_at: row.last_received_at,
            })
            .collect();
        let state = match (self.environment.api, adapters.is_empty()) {
            (None, _) => IntegrationState::ApiUnavailable,
            (Some(_), true) => IntegrationState::AwaitingFirstCapture,
            (Some(_), false) => IntegrationState::Receiving,
        };
        Ok(IntegrationStatus {
            state,
            contract_version: CAPTURE_ENVELOPE_SCHEMA_VERSION,
            api: self.environment.api,
            adapters,
            outbox: outbox_status(&self.environment.outbox_dir),
            data_dir: self.environment.data_dir.clone(),
        })
    }

    /// Runs the connection test: one [`IntegrationCheck`] per [`CheckKind`], in
    /// declaration order.
    ///
    /// # Errors
    ///
    /// [`IntegrationError::Storage`] when the checkpoint query fails.
    pub fn check(&self) -> Result<Vec<IntegrationCheck>, IntegrationError> {
        let status = self.status()?;
        let runtime = &self.environment.runtime_dir;
        let mut checks = Vec::with_capacity(5);

        checks.push(match status.api {
            Some(api) => check(
                CheckKind::LocalApi,
                CheckOutcome::Ok,
                format!("API local ativa em 127.0.0.1:{}.", api.port),
            ),
            None => check(
                CheckKind::LocalApi,
                CheckOutcome::Failed,
                "A API local não está ativa. As capturas vão para a outbox e serão \
                 importadas na próxima abertura; reinicie o aplicativo para reativá-la."
                    .to_string(),
            ),
        });

        checks.push(match status.api {
            None => check(
                CheckKind::Discovery,
                CheckOutcome::Skipped,
                "Sem API local ativa, o arquivo de descoberta não se aplica.".to_string(),
            ),
            Some(api) => discovery_check(&runtime.join(DISCOVERY_FILE), api),
        });

        checks.push(match status.api {
            None => check(
                CheckKind::Token,
                CheckOutcome::Skipped,
                "Sem API local ativa, o token de sessão não se aplica.".to_string(),
            ),
            Some(_) => token_check(&runtime.join(TOKEN_FILE)),
        });

        checks.push(outbox_check(&status.outbox));

        checks.push(match status.adapters.first() {
            Some(latest) => check(
                CheckKind::Captures,
                CheckOutcome::Ok,
                format!("Última captura recebida em {}.", latest.last_received_at),
            ),
            None => check(
                CheckKind::Captures,
                CheckOutcome::Warning,
                "Nenhuma captura recebida ainda. Confirme que o plugin do xemnas está \
                 ativo no OpenCode e conclua um turno em um projeto cadastrado."
                    .to_string(),
            ),
        });

        Ok(checks)
    }
}

fn check(kind: CheckKind, outcome: CheckOutcome, message: String) -> IntegrationCheck {
    IntegrationCheck {
        kind,
        outcome,
        message,
    }
}

/// Discovery fields the adapter relies on; unknown fields are tolerated.
#[derive(Deserialize)]
struct DiscoveryProbe {
    protocol_version: u32,
    port: u16,
}

fn discovery_check(path: &Path, api: LocalApiEndpoint) -> IntegrationCheck {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(_) => {
            return check(
                CheckKind::Discovery,
                CheckOutcome::Failed,
                "O arquivo de descoberta não foi encontrado; o adapter não localiza o \
                 aplicativo. Reinicie o aplicativo para recriá-lo."
                    .to_string(),
            )
        }
    };
    match serde_json::from_str::<DiscoveryProbe>(&text) {
        Ok(probe) if probe.port == api.port && probe.protocol_version == api.protocol_version => {
            check(
                CheckKind::Discovery,
                CheckOutcome::Ok,
                "O arquivo de descoberta aponta para esta instância.".to_string(),
            )
        }
        Ok(_) => check(
            CheckKind::Discovery,
            CheckOutcome::Failed,
            "O arquivo de descoberta aponta para outra instância. Feche outras cópias \
             do aplicativo e reinicie esta."
                .to_string(),
        ),
        Err(_) => check(
            CheckKind::Discovery,
            CheckOutcome::Failed,
            "O arquivo de descoberta está corrompido. Reinicie o aplicativo para recriá-lo."
                .to_string(),
        ),
    }
}

/// Checks only the token file's metadata; its content is never read.
fn token_check(path: &Path) -> IntegrationCheck {
    match fs::metadata(path) {
        Ok(metadata) if metadata.is_file() && metadata.len() > 0 => check(
            CheckKind::Token,
            CheckOutcome::Ok,
            "O token de sessão está disponível para o adapter.".to_string(),
        ),
        _ => check(
            CheckKind::Token,
            CheckOutcome::Failed,
            "O token de sessão está ausente ou vazio; o adapter será recusado. \
             Reinicie o aplicativo para gerá-lo."
                .to_string(),
        ),
    }
}

fn outbox_check(outbox: &OutboxStatus) -> IntegrationCheck {
    if outbox.rejected > 0 {
        return check(
            CheckKind::Outbox,
            CheckOutcome::Warning,
            format!(
                "{} captura(s) da outbox foram rejeitadas e não serão importadas. \
                 Consulte Diagnostics.",
                outbox.rejected
            ),
        );
    }
    if outbox.pending > 0 {
        return check(
            CheckKind::Outbox,
            CheckOutcome::Warning,
            format!(
                "{} captura(s) aguardam na outbox e serão importadas na próxima abertura.",
                outbox.pending
            ),
        );
    }
    let message = if outbox.exists {
        "A outbox está vazia.".to_string()
    } else {
        "A outbox ainda não existe; o adapter a cria quando o aplicativo estiver fechado."
            .to_string()
    };
    check(CheckKind::Outbox, CheckOutcome::Ok, message)
}

/// Counts outbox files; a missing directory counts as zero.
pub fn outbox_status(root: &Path) -> OutboxStatus {
    OutboxStatus {
        path: root.to_path_buf(),
        exists: root.is_dir(),
        pending: count_json(&root.join("pending")),
        accepted: count_json(&root.join("accepted")),
        rejected: count_json(&root.join("rejected")),
    }
}

fn count_json(directory: &Path) -> i64 {
    fs::read_dir(directory)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|entry| {
                    entry
                        .path()
                        .extension()
                        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
                })
                .count() as i64
        })
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct FakeStore(Result<Vec<AdapterActivityRow>, IntegrationError>);

    impl IntegrationStore for FakeStore {
        fn adapter_activity(&self) -> Result<Vec<AdapterActivityRow>, IntegrationError> {
            self.0.clone()
        }
    }

    fn temp_dir(tag: &str) -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let directory = std::env::temp_dir().join(format!(
            "xemnas-integration-{tag}-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).expect("create temp dir");
        directory
    }

    fn activity(version: Option<&str>) -> AdapterActivityRow {
        AdapterActivityRow {
            adapter: "opencode".to_string(),
            adapter_version: version.map(str::to_string),
            sessions: 2,
            last_observed_at: "2026-01-01T00:00:00Z".to_string(),
            last_received_at: "2026-01-01T00:00:01Z".to_string(),
        }
    }

    fn environment(root: &Path, api: Option<LocalApiEndpoint>) -> IntegrationEnvironment {
        IntegrationEnvironment {
            data_dir: root.to_path_buf(),
            outbox_dir: root.join("outbox"),
            runtime_dir: root.join("state"),
            api,
        }
    }

    const API: LocalApiEndpoint = LocalApiEndpoint {
        port: 4321,
        protocol_version: 1,
    };

    fn write_runtime(root: &Path, port: u16, token: &str) {
        let runtime = root.join("state");
        fs::create_dir_all(&runtime).expect("runtime dir");
        fs::write(
            runtime.join(DISCOVERY_FILE),
            format!(r#"{{"protocol_version":1,"port":{port},"instance_id":"i"}}"#),
        )
        .expect("discovery");
        fs::write(runtime.join(TOKEN_FILE), token).expect("token");
    }

    fn outcome(checks: &[IntegrationCheck], kind: CheckKind) -> CheckOutcome {
        checks
            .iter()
            .find(|check| check.kind == kind)
            .map(|check| check.outcome)
            .expect("check present")
    }

    #[test]
    fn status_without_api_or_captures() {
        let root = temp_dir("no-api");
        let integration = Integration::new(FakeStore(Ok(vec![])), environment(&root, None));
        let status = integration.status().expect("status");
        assert_eq!(status.state, IntegrationState::ApiUnavailable);
        assert_eq!(status.contract_version, CAPTURE_ENVELOPE_SCHEMA_VERSION);
        assert!(status.adapters.is_empty());
        assert!(!status.outbox.exists);
        assert_eq!(status.outbox.pending, 0);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn status_maps_versions_to_compatibility() {
        let root = temp_dir("versions");
        let integration = Integration::new(
            FakeStore(Ok(vec![activity(Some("0.2.0")), activity(None)])),
            environment(&root, Some(API)),
        );
        let status = integration.status().expect("status");
        assert_eq!(status.state, IntegrationState::Receiving);
        assert_eq!(status.adapters[0].compatibility, Compatibility::Compatible);
        assert_eq!(status.adapters[0].adapter_version.as_deref(), Some("0.2.0"));
        assert_eq!(status.adapters[1].compatibility, Compatibility::Unknown);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn status_counts_outbox_json_files_only() {
        let root = temp_dir("outbox");
        for (directory, files) in [("pending", 2), ("accepted", 1), ("rejected", 3)] {
            let path = root.join("outbox").join(directory);
            fs::create_dir_all(&path).expect("outbox dir");
            for index in 0..files {
                fs::write(path.join(format!("{index}.json")), "{}").expect("item");
            }
            fs::write(path.join("ignored.tmp"), "x").expect("tmp");
        }
        let integration = Integration::new(FakeStore(Ok(vec![])), environment(&root, Some(API)));
        let status = integration.status().expect("status");
        assert_eq!(status.state, IntegrationState::AwaitingFirstCapture);
        assert!(status.outbox.exists);
        assert_eq!(
            (
                status.outbox.pending,
                status.outbox.accepted,
                status.outbox.rejected
            ),
            (2, 1, 3)
        );
        let checks = integration.check().expect("checks");
        assert_eq!(outcome(&checks, CheckKind::Outbox), CheckOutcome::Warning);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn check_passes_with_matching_runtime_files() {
        let root = temp_dir("healthy");
        write_runtime(&root, API.port, "token-value");
        let integration = Integration::new(
            FakeStore(Ok(vec![activity(Some("0.1.0"))])),
            environment(&root, Some(API)),
        );
        let checks = integration.check().expect("checks");
        let kinds: Vec<CheckKind> = checks.iter().map(|check| check.kind).collect();
        assert_eq!(
            kinds,
            vec![
                CheckKind::LocalApi,
                CheckKind::Discovery,
                CheckKind::Token,
                CheckKind::Outbox,
                CheckKind::Captures
            ]
        );
        assert!(checks.iter().all(|check| check.outcome == CheckOutcome::Ok));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn check_flags_stale_discovery_and_empty_token() {
        let root = temp_dir("stale");
        write_runtime(&root, 9999, "");
        let integration = Integration::new(FakeStore(Ok(vec![])), environment(&root, Some(API)));
        let checks = integration.check().expect("checks");
        assert_eq!(outcome(&checks, CheckKind::Discovery), CheckOutcome::Failed);
        assert_eq!(outcome(&checks, CheckKind::Token), CheckOutcome::Failed);
        assert_eq!(outcome(&checks, CheckKind::Captures), CheckOutcome::Warning);

        fs::write(root.join("state").join(DISCOVERY_FILE), "not json").expect("corrupt");
        let checks = integration.check().expect("checks");
        assert_eq!(outcome(&checks, CheckKind::Discovery), CheckOutcome::Failed);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn check_skips_runtime_files_without_api() {
        let root = temp_dir("skipped");
        let integration = Integration::new(FakeStore(Ok(vec![])), environment(&root, None));
        let checks = integration.check().expect("checks");
        assert_eq!(outcome(&checks, CheckKind::LocalApi), CheckOutcome::Failed);
        assert_eq!(
            outcome(&checks, CheckKind::Discovery),
            CheckOutcome::Skipped
        );
        assert_eq!(outcome(&checks, CheckKind::Token), CheckOutcome::Skipped);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn check_never_echoes_the_token() {
        let root = temp_dir("token-secret");
        write_runtime(&root, API.port, "SECRET-MARKER-TOKEN");
        let integration = Integration::new(FakeStore(Ok(vec![])), environment(&root, Some(API)));
        let checks = integration.check().expect("checks");
        let json = serde_json::to_string(&checks).expect("serialize");
        let status = serde_json::to_string(&integration.status().expect("status"))
            .expect("serialize status");
        assert!(!json.contains("SECRET-MARKER"));
        assert!(!status.contains("SECRET-MARKER"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn storage_failure_is_reported() {
        let root = temp_dir("storage");
        let integration = Integration::new(
            FakeStore(Err(IntegrationError::Storage("boom".to_string()))),
            environment(&root, Some(API)),
        );
        assert_eq!(
            integration.status().map_err(|error| error.code()),
            Err("storage")
        );
        assert!(integration.check().is_err());
        let _ = fs::remove_dir_all(&root);
    }
}
