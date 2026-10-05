//! [`Backend`] over the loopback local API of the running app.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{json, Value};

use crate::protocol::{Backend, BackendError};

/// Timeout for one local API call.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// Talks to `POST /v1/agent/*`, reading discovery and token on every call
/// because the app writes new ones on each start.
pub struct HttpBackend {
    runtime_dir: PathBuf,
    directory: String,
    client: reqwest::blocking::Client,
}

impl HttpBackend {
    /// Backend for the project at `directory`, with the app's runtime files in `runtime_dir`.
    pub fn new(runtime_dir: impl Into<PathBuf>, directory: impl Into<String>) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap_or_else(|_| reqwest::blocking::Client::new());
        Self {
            runtime_dir: runtime_dir.into(),
            directory: directory.into(),
            client,
        }
    }

    fn post(&self, path: &str, body: Value) -> Result<Value, BackendError> {
        let (port, token) = endpoint(&self.runtime_dir).ok_or(BackendError::AppClosed)?;
        let response = self
            .client
            .post(format!("http://127.0.0.1:{port}{path}"))
            .bearer_auth(token)
            .json(&body)
            .send()
            .map_err(|_| BackendError::AppClosed)?;
        match response.status().as_u16() {
            200 => response.json::<Value>().map_err(|_| BackendError::Failed),
            400 | 422 => Err(BackendError::Invalid),
            403 => Err(BackendError::ProjectNotFound),
            404 => Err(BackendError::NotFound),
            409 => Err(BackendError::Ambiguous),
            _ => Err(BackendError::Failed),
        }
    }
}

impl Backend for HttpBackend {
    fn decision(&self, reference: &str) -> Result<String, BackendError> {
        let body = self.post(
            "/v1/agent/decision",
            json!({ "canonical_path": self.directory, "reference": reference }),
        )?;
        body["text"]
            .as_str()
            .map(str::to_string)
            .ok_or(BackendError::Failed)
    }

    fn search(&self, query: &str, path: Option<&str>) -> Result<Option<String>, BackendError> {
        let mut request = json!({ "canonical_path": self.directory, "query": query });
        if let Some(path) = path {
            request["path"] = json!(path);
        }
        let body = self.post("/v1/agent/search", request)?;
        Ok(body["text"].as_str().map(str::to_string))
    }

    fn file(&self, path: &str) -> Result<Option<String>, BackendError> {
        let body = self.post(
            "/v1/agent/file",
            json!({ "canonical_path": self.directory, "path": path }),
        )?;
        Ok(body["text"].as_str().map(str::to_string))
    }
}

/// Port and token of the running app, or `None` when it is closed.
pub(crate) fn endpoint(runtime_dir: &Path) -> Option<(u16, String)> {
    let discovery: Value =
        serde_json::from_str(&std::fs::read_to_string(runtime_dir.join("discovery.json")).ok()?)
            .ok()?;
    if discovery["protocol_version"].as_u64() != Some(1) {
        return None;
    }
    let port = u16::try_from(discovery["port"].as_u64()?).ok()?;
    let token = std::fs::read_to_string(runtime_dir.join("api-token")).ok()?;
    let token = token.trim();
    (!token.is_empty()).then(|| (port, token.to_string()))
}
