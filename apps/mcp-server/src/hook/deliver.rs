//! Delivery of capture envelopes: the local API first, the outbox as fallback.
//!
//! Follows `adapters/opencode/src/client.ts` and `outbox.ts`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use application::CaptureEnvelope;
use sha2::{Digest, Sha256};

use crate::backend::endpoint;

/// Timeout of one `POST /v1/captures`.
const POST_TIMEOUT: Duration = Duration::from_secs(2);

/// What happened to one envelope.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The app accepted it (2xx).
    Sent,
    /// The app was unavailable; it waits in `outbox/pending`.
    Queued,
    /// The app refused it (4xx): retrying the same envelope cannot help.
    Rejected(u16),
    /// Neither the app nor the outbox took it.
    Failed,
}

/// Sends envelopes to the running app, falling back to `outbox/pending`.
pub struct Sender {
    /// Port and token; cleared on the first unavailable answer so the rest of
    /// the run goes straight to the outbox instead of waiting on timeouts.
    endpoint: Option<(u16, String)>,
    client: Option<reqwest::blocking::Client>,
    pending_dir: PathBuf,
}

impl Sender {
    /// Sender for the app whose runtime files are in `runtime_dir`.
    pub fn new(runtime_dir: &Path, outbox_dir: &Path) -> Self {
        let endpoint = endpoint(runtime_dir);
        let client = endpoint.as_ref().and_then(|_| {
            reqwest::blocking::Client::builder()
                .timeout(POST_TIMEOUT)
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .ok()
        });
        Self {
            endpoint,
            client,
            pending_dir: outbox_dir.join("pending"),
        }
    }

    /// Delivers `envelope`: 2xx advances, 4xx stops, anything else is queued.
    pub fn send(&mut self, envelope: &CaptureEnvelope) -> Outcome {
        if let (Some((port, token)), Some(client)) = (&self.endpoint, &self.client) {
            let response = client
                .post(format!("http://127.0.0.1:{port}/v1/captures"))
                .bearer_auth(token)
                .header("Idempotency-Key", &envelope.idempotency_key)
                .json(envelope)
                .send();
            match response.map(|response| response.status().as_u16()) {
                Ok(200..=299) => return Outcome::Sent,
                Ok(status @ 400..=499) => return Outcome::Rejected(status),
                _ => self.endpoint = None,
            }
        }
        match write_pending(&self.pending_dir, envelope) {
            Ok(()) => Outcome::Queued,
            Err(_) => Outcome::Failed,
        }
    }
}

/// Writes `pending/<sha256hex(idempotency_key)>.json` through a temporary file
/// and a rename, so the app never sees a partial file and the same key
/// overwrites the same file.
fn write_pending(pending_dir: &Path, envelope: &CaptureEnvelope) -> std::io::Result<()> {
    std::fs::create_dir_all(pending_dir)?;
    let name: String = Sha256::digest(envelope.idempotency_key.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let temporary = pending_dir.join(format!("{name}.json.{}.tmp", std::process::id()));
    let result = serde_json::to_vec(envelope)
        .map_err(std::io::Error::other)
        .and_then(|mut bytes| {
            bytes.push(b'\n');
            std::fs::write(&temporary, bytes)
        })
        .and_then(|()| std::fs::rename(&temporary, pending_dir.join(format!("{name}.json"))));
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}
