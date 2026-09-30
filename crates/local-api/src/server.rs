//! Axum router, authentication middleware, timeouts and the four MVP endpoints.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use application::captures::{CaptureApi, IngestError, Receipt};
use axum::extract::rejection::JsonRejection;
use axum::extract::{DefaultBodyLimit, Path, Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use integration_contracts::capture::{CaptureEnvelope, CAPTURE_ENVELOPE_SCHEMA_VERSION};
use serde::Serialize;
use serde_json::{json, Value};

use crate::auth::constant_time_eq;
use crate::discovery::{
    generate_instance_id, remove_runtime_files, write_runtime_files, DiscoveryInfo,
    PROTOCOL_VERSION,
};
use crate::error::ApiError;

/// Default request body limit: 4 MiB (ticket 09 decision 6).
pub const DEFAULT_MAX_BODY_BYTES: usize = 4 * 1024 * 1024;

/// Default whole-request processing timeout (ticket 09 decision 6).
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// Runtime configuration for the local API.
#[derive(Debug, Clone)]
pub struct ApiConfig {
    /// Protocol version advertised to adapters.
    pub protocol_version: u32,
    /// Per-session instance identifier.
    pub instance_id: String,
    /// Bearer token required on every endpoint.
    pub token: String,
    /// Maximum accepted body size in bytes.
    pub max_body_bytes: usize,
    /// Whole-request timeout (headers, body read and handler).
    pub request_timeout: Duration,
}

impl ApiConfig {
    /// Builds a config with the ticket defaults for limits and timeout.
    pub fn new(token: String, instance_id: String) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            instance_id,
            token,
            max_body_bytes: DEFAULT_MAX_BODY_BYTES,
            request_timeout: DEFAULT_REQUEST_TIMEOUT,
        }
    }
}

/// Shared router state.
#[derive(Clone)]
struct AppState {
    api: Arc<dyn CaptureApi>,
    config: ApiConfig,
}

/// Builds the router with the four MVP endpoints.
pub fn router(api: Arc<dyn CaptureApi>, config: ApiConfig) -> Router {
    let state = AppState {
        api,
        config: config.clone(),
    };
    Router::new()
        .route("/v1/health", get(health))
        .route("/v1/capabilities", get(capabilities))
        .route("/v1/captures", post(create_capture))
        .route("/v1/captures/{id}", get(get_capture))
        .layer(middleware::from_fn_with_state(state.clone(), authorize))
        .layer(DefaultBodyLimit::max(config.max_body_bytes))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            with_request_timeout,
        ))
        .with_state(state)
}

/// Binds an ephemeral port on IPv4 loopback only.
pub async fn bind_loopback() -> std::io::Result<tokio::net::TcpListener> {
    tokio::net::TcpListener::bind(("127.0.0.1", 0)).await
}

/// Serves the router until the task is dropped or aborted.
pub async fn serve(listener: tokio::net::TcpListener, router: Router) -> std::io::Result<()> {
    axum::serve(listener, router).await
}

/// Configuration for [`ApiServer::start`].
#[derive(Clone)]
pub struct ApiServerConfig {
    /// The use case the API exposes.
    pub api: Arc<dyn CaptureApi>,
    /// Directory receiving `discovery.json` and `api-token`.
    pub runtime_dir: PathBuf,
    /// Protocol version advertised to adapters.
    pub protocol_version: u32,
    /// Maximum accepted body size in bytes.
    pub max_body_bytes: usize,
    /// Whole-request timeout (headers, body read and handler).
    pub request_timeout: Duration,
}

impl ApiServerConfig {
    /// Builds a config with the ticket defaults for limits and timeout.
    pub fn new(api: Arc<dyn CaptureApi>, runtime_dir: impl Into<PathBuf>) -> Self {
        Self {
            api,
            runtime_dir: runtime_dir.into(),
            protocol_version: PROTOCOL_VERSION,
            max_body_bytes: DEFAULT_MAX_BODY_BYTES,
            request_timeout: DEFAULT_REQUEST_TIMEOUT,
        }
    }
}

/// Failure while starting the local API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiServerError {
    /// Binding, file writing or thread setup failed.
    Io(String),
    /// The system RNG could not provide token entropy.
    Entropy(String),
}

impl std::fmt::Display for ApiServerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(message) => write!(formatter, "local API setup failed: {message}"),
            Self::Entropy(message) => write!(formatter, "local API token failed: {message}"),
        }
    }
}

impl std::error::Error for ApiServerError {}

/// A running local API: its address, discovery data, token and shutdown handle.
pub struct RunningApi {
    address: SocketAddr,
    discovery: DiscoveryInfo,
    token: String,
    runtime_dir: PathBuf,
    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl RunningApi {
    /// Actual bound loopback address.
    pub fn address(&self) -> SocketAddr {
        self.address
    }

    /// Discovery data written for the adapter.
    pub fn discovery(&self) -> &DiscoveryInfo {
        &self.discovery
    }

    /// Bearer token required by every endpoint.
    pub fn token(&self) -> &str {
        &self.token
    }

    /// Stops the server, joins its runtime thread and removes the session files.
    pub fn shutdown(mut self) {
        self.stop();
    }

    /// Idempotent stop used by [`RunningApi::shutdown`] and `Drop`.
    fn stop(&mut self) {
        if let Some(sender) = self.shutdown.take() {
            let _ = sender.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        remove_runtime_files(&self.runtime_dir);
    }
}

impl Drop for RunningApi {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Starts the loopback API on its own Tokio runtime thread.
pub struct ApiServer;

impl ApiServer {
    /// Binds loopback, writes discovery/token and serves until shutdown.
    pub fn start(config: ApiServerConfig) -> Result<RunningApi, ApiServerError> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|error| ApiServerError::Io(error.to_string()))?;
        let listener = runtime
            .block_on(bind_loopback())
            .map_err(|error| ApiServerError::Io(error.to_string()))?;
        let address = listener
            .local_addr()
            .map_err(|error| ApiServerError::Io(error.to_string()))?;
        let token = crate::auth::generate_token()
            .map_err(|error| ApiServerError::Entropy(error.to_string()))?;
        let instance_id = generate_instance_id();
        let discovery = DiscoveryInfo {
            protocol_version: config.protocol_version,
            port: address.port(),
            instance_id: instance_id.clone(),
        };
        write_runtime_files(&config.runtime_dir, &discovery, &token)
            .map_err(|error| ApiServerError::Io(error.to_string()))?;

        let (shutdown, shutdown_rx) = tokio::sync::oneshot::channel();
        let router = router(
            config.api,
            ApiConfig {
                protocol_version: config.protocol_version,
                instance_id,
                token: token.clone(),
                max_body_bytes: config.max_body_bytes,
                request_timeout: config.request_timeout,
            },
        );

        let runtime_dir = config.runtime_dir.clone();
        let thread = match std::thread::Builder::new()
            .name("xemnas-local-api".to_string())
            .spawn(move || {
                let _ = runtime.block_on(async move {
                    axum::serve(listener, router)
                        .with_graceful_shutdown(async {
                            let _ = shutdown_rx.await;
                        })
                        .await
                });
            }) {
            Ok(thread) => thread,
            Err(error) => {
                remove_runtime_files(&runtime_dir);
                return Err(ApiServerError::Io(error.to_string()));
            }
        };

        Ok(RunningApi {
            address,
            discovery,
            token,
            runtime_dir,
            shutdown: Some(shutdown),
            thread: Some(thread),
        })
    }
}

/// Rejects browser origins and enforces the bearer token.
async fn authorize(State(state): State<AppState>, request: Request, next: Next) -> Response {
    if request.headers().contains_key(axum::http::header::ORIGIN) {
        return ApiError::Forbidden.into_response();
    }
    let provided = request
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));
    match provided {
        Some(token) if constant_time_eq(token.as_bytes(), state.config.token.as_bytes()) => {
            next.run(request).await
        }
        _ => ApiError::Unauthorized.into_response(),
    }
}

/// Applies the whole-request timeout around the inner service.
async fn with_request_timeout(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    match tokio::time::timeout(state.config.request_timeout, next.run(request)).await {
        Ok(response) => response,
        Err(_elapsed) => ApiError::GatewayTimeout.into_response(),
    }
}

/// Health probe.
async fn health(State(state): State<AppState>) -> Json<Value> {
    Json(json!({
        "status": "ok",
        "protocol_version": state.config.protocol_version,
        "instance_id": state.config.instance_id,
    }))
}

/// Adapter-facing capabilities.
async fn capabilities(State(state): State<AppState>) -> Json<Value> {
    Json(json!({
        "protocol_version": state.config.protocol_version,
        "capture_schema_versions": [CAPTURE_ENVELOPE_SCHEMA_VERSION],
        "max_body_bytes": state.config.max_body_bytes,
    }))
}

/// `POST /v1/captures`: the eight ingest steps of MVP-SPEC §7.3.
async fn create_capture(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Response {
    let value = match payload {
        Ok(Json(value)) => value,
        Err(rejection) => return json_rejection_to_api_error(rejection).into_response(),
    };

    let Some(idempotency_key) = headers
        .get("Idempotency-Key")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
    else {
        return ApiError::BadRequest.into_response();
    };

    if let Err(error) = integration_contracts::capture::validate_envelope(&value) {
        tracing::warn!(
            operation = "validate_capture",
            error_count = error.error_count(),
            keyword = error.keywords().first().map(String::as_str).unwrap_or(""),
            "capture envelope failed schema validation"
        );
        return ApiError::Unprocessable.into_response();
    }

    let envelope: CaptureEnvelope = match serde_json::from_value(value) {
        Ok(envelope) => envelope,
        Err(error) => {
            tracing::warn!(
                operation = "deserialize_capture",
                line = error.line(),
                column = error.column(),
                "capture envelope failed deserialization"
            );
            return ApiError::Unprocessable.into_response();
        }
    };

    if envelope.idempotency_key != idempotency_key {
        return ApiError::BadRequest.into_response();
    }

    let api = state.api.clone();
    let key_for_call = idempotency_key.clone();
    let outcome = run_blocking(move || api.ingest(&envelope, &key_for_call)).await;

    match outcome {
        Ok(Ok(outcome)) => {
            let status = if outcome.replayed {
                StatusCode::OK
            } else {
                StatusCode::CREATED
            };
            (status, Json(ReceiptBody::from(outcome.receipt))).into_response()
        }
        Ok(Err(error)) => ingest_error_response(error),
        Err(error) => error.into_response(),
    }
}

/// `GET /v1/captures/{id}`: the stored receipt or 404.
async fn get_capture(State(state): State<AppState>, Path(capture_id): Path<String>) -> Response {
    let api = state.api.clone();
    let result = run_blocking(move || api.receipt(&capture_id)).await;
    match result {
        Ok(Ok(receipt)) => Json(ReceiptBody::from(receipt)).into_response(),
        Ok(Err(error)) => ingest_error_response(error),
        Err(error) => error.into_response(),
    }
}

/// Maps a use-case failure onto the HTTP surface, logging storage detail only.
fn ingest_error_response(error: IngestError) -> Response {
    match error {
        IngestError::Forbidden => ApiError::Forbidden.into_response(),
        IngestError::Conflict => ApiError::Conflict.into_response(),
        IngestError::InvalidFingerprint => ApiError::Unprocessable.into_response(),
        IngestError::NotFound => ApiError::NotFound.into_response(),
        IngestError::Storage(message) => {
            tracing::error!(
                error = %message,
                operation = "capture_ingest",
                "capture persistence failed"
            );
            ApiError::Internal.into_response()
        }
    }
}

/// Maps an extractor rejection to our error surface.
fn json_rejection_to_api_error(rejection: JsonRejection) -> ApiError {
    match rejection.status() {
        StatusCode::PAYLOAD_TOO_LARGE => ApiError::PayloadTooLarge,
        StatusCode::UNPROCESSABLE_ENTITY => ApiError::Unprocessable,
        _ => ApiError::BadRequest,
    }
}

/// Runs a blocking use-case call on the blocking pool.
async fn run_blocking<T, F>(task: F) -> Result<T, ApiError>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    match tokio::task::spawn_blocking(task).await {
        Ok(value) => Ok(value),
        Err(_join_error) => Err(ApiError::Internal),
    }
}

/// Receipt wire shape returned to the adapter.
#[derive(Debug, Serialize)]
struct ReceiptBody {
    capture_id: String,
    idempotency_key: String,
    received_at: String,
    artifact_count: i64,
}

impl From<Receipt> for ReceiptBody {
    fn from(receipt: Receipt) -> Self {
        Self {
            capture_id: receipt.capture_id,
            idempotency_key: receipt.idempotency_key,
            received_at: receipt.received_at,
            artifact_count: receipt.artifact_count,
        }
    }
}
