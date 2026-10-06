//! Outbox drain: import captures the adapter could not deliver while the app was
//! closed.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

use integration_contracts::capture::{validate_envelope, CaptureEnvelope, EnvelopeValidationError};
use serde_json::{json, Value};

use crate::captures::{CaptureApi, IngestError};
use crate::clock::now_rfc3339;

/// Subdirectory with items waiting to be imported.
const PENDING: &str = "pending";
/// Subdirectory with an item claimed by the current drain.
const SENDING: &str = "sending";
/// Subdirectory with successfully imported items (bounded retention).
const ACCEPTED: &str = "accepted";
/// Subdirectory with safe diagnostics for items that can never be imported.
const REJECTED: &str = "rejected";
/// Subdirectory with intact items parked by older versions after repeated refusals;
/// `retry_stalled` restores them. Nothing parks new items here anymore.
const STALLED: &str = "stalled";

/// Default retention of `rejected/` diagnostics: long enough to outlive a
/// dogfood week, short enough that diagnostics cannot pile up forever.
pub const DEFAULT_REJECTED_RETENTION: Duration = Duration::from_secs(30 * 24 * 60 * 60);

/// Retention and retry bounds for one drain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrainPolicy {
    /// Age after which `accepted/` items are removed.
    pub accepted_retention: Duration,
    /// Age after which `rejected/` diagnostics are removed.
    pub rejected_retention: Duration,
}

impl DrainPolicy {
    /// Policy with the given accepted retention and the default bounds.
    pub fn new(accepted_retention: Duration) -> Self {
        Self {
            accepted_retention,
            rejected_retention: DEFAULT_REJECTED_RETENTION,
        }
    }
}

/// Failure setting up or listing the outbox.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutboxError {
    /// Filesystem access failed for a directory-level operation.
    Io(String),
}

impl std::fmt::Display for OutboxError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(message) => write!(formatter, "outbox error: {message}"),
        }
    }
}

impl std::error::Error for OutboxError {}

impl From<io::Error> for OutboxError {
    fn from(error: io::Error) -> Self {
        Self::Io(error.to_string())
    }
}

/// Outcome of one drain pass, with counts only (never capture content).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DrainReport {
    /// Items imported (fresh inserts and idempotent replays).
    pub accepted: usize,
    /// Rejected items by code (`invalid_json`, `schema_invalid`, ...).
    pub rejected: BTreeMap<String, usize>,
    /// `.json` items still waiting in `pending/` (including transient returns).
    pub pending_remaining: usize,
    /// Items moved from `sending/` back to `pending/` at the start.
    pub sending_recovered: usize,
    /// Accepted items removed by retention.
    pub pruned: usize,
    /// Rejected diagnostics removed by retention.
    pub pruned_rejected: usize,
}

/// Drains the outbox under `root` into `api` with [`DrainPolicy::new`].
pub fn drain(
    root: &Path,
    api: &dyn CaptureApi,
    accepted_retention: Duration,
) -> Result<DrainReport, OutboxError> {
    drain_with(root, api, &DrainPolicy::new(accepted_retention))
}

/// Drains the outbox under `root` into `api` with an explicit policy.
pub fn drain_with(
    root: &Path,
    api: &dyn CaptureApi,
    policy: &DrainPolicy,
) -> Result<DrainReport, OutboxError> {
    let pending_dir = root.join(PENDING);
    let sending_dir = root.join(SENDING);
    let accepted_dir = root.join(ACCEPTED);
    let rejected_dir = root.join(REJECTED);
    for directory in [&pending_dir, &sending_dir, &accepted_dir, &rejected_dir] {
        fs::create_dir_all(directory)?;
    }

    let mut report = DrainReport::default();

    for path in json_files(&sending_dir)? {
        if let Some(filename) = file_name(&path) {
            if restore_to_pending(root, &filename, &path) {
                report.sending_recovered += 1;
            }
        }
    }

    remove_orphan_tmp(&pending_dir)?;

    let mut accepted_now: BTreeSet<String> = BTreeSet::new();

    for path in json_files(&pending_dir)? {
        let Some(filename) = file_name(&path) else {
            continue;
        };

        let claimed = sending_dir.join(&filename);
        match move_no_replace(&path, &claimed) {
            Ok(true) => {}
            Ok(false) => continue,
            Err(_) => continue,
        }

        let text = match fs::read_to_string(&claimed) {
            Ok(text) => text,
            Err(_) => {
                let _ = restore_to_pending(root, &filename, &claimed);
                continue;
            }
        };

        let value: Value = match serde_json::from_str(&text) {
            Ok(value) => value,
            Err(_) => {
                reject(
                    root,
                    &filename,
                    &claimed,
                    "invalid_json",
                    "malformed JSON",
                    &mut report,
                );
                continue;
            }
        };

        if let Err(error) = validate_envelope(&value) {
            let detail = validation_detail(&error);
            reject(
                root,
                &filename,
                &claimed,
                "schema_invalid",
                &detail,
                &mut report,
            );
            continue;
        }

        let envelope: CaptureEnvelope = match serde_json::from_value(value) {
            Ok(envelope) => envelope,
            Err(error) => {
                let detail = format!("line {}, column {}", error.line(), error.column());
                reject(
                    root,
                    &filename,
                    &claimed,
                    "deserialize_failed",
                    &detail,
                    &mut report,
                );
                continue;
            }
        };

        match api.ingest(&envelope, &envelope.idempotency_key) {
            Ok(_) => {
                if accept_item(&filename, &claimed, &accepted_dir).is_ok() {
                    accepted_now.insert(filename);
                    report.accepted += 1;
                }
            }
            // `Forbidden` means the folder is not a registered project (or its path
            // cannot be resolved). The ingest has no other cause for it, so the
            // refusal is final: retrying every drain only piles up envelopes that
            // the Claude Code hook writes for unregistered folders.
            Err(IngestError::Forbidden) => reject(
                root,
                &filename,
                &claimed,
                "project_not_registered",
                "the project is not registered or its path cannot be resolved",
                &mut report,
            ),
            Err(IngestError::Storage(_)) => {
                let _ = restore_to_pending(root, &filename, &claimed);
            }
            Err(IngestError::InvalidFingerprint) => reject(
                root,
                &filename,
                &claimed,
                "invalid_fingerprint",
                "declared fingerprint does not match the content",
                &mut report,
            ),
            Err(IngestError::Conflict) => reject(
                root,
                &filename,
                &claimed,
                "artifact_id_conflict",
                "payload repeats an artifact id",
                &mut report,
            ),
            Err(IngestError::NotFound) => reject(
                root,
                &filename,
                &claimed,
                "ingest_not_found",
                "capture could not be located after ingest",
                &mut report,
            ),
        }
    }

    report.pending_remaining = json_files(&pending_dir)?.len();
    report.pruned = prune_older(&accepted_dir, policy.accepted_retention, &accepted_now)?;
    report.pruned_rejected =
        prune_older(&rejected_dir, policy.rejected_retention, &BTreeSet::new())?;
    Ok(report)
}

/// Moves every item parked in `stalled/` back to `pending/`, resetting its
/// attempt counter, so the next drain retries it (an explicit user action,
/// for example after registering the project).
pub fn retry_stalled(root: &Path) -> Result<usize, OutboxError> {
    let stalled_dir = root.join(STALLED);
    if !stalled_dir.is_dir() {
        return Ok(0);
    }
    fs::create_dir_all(root.join(PENDING))?;
    let mut restored = 0;
    for path in json_files(&stalled_dir)? {
        let Some(filename) = file_name(&path) else {
            continue;
        };
        match move_no_replace(&path, &root.join(PENDING).join(&filename)) {
            Ok(true) => restored += 1,
            Ok(false) => {
                let _ = fs::remove_file(&path);
            }
            Err(_) => {}
        }
    }
    Ok(restored)
}

/// Moves an accepted item and stamps the acceptance time on it.
fn accept_item(filename: &str, claimed: &Path, accepted_dir: &Path) -> io::Result<()> {
    let accepted = accepted_dir.join(filename);
    match move_no_replace(claimed, &accepted) {
        Ok(true) => {
            let _ = stamp_modified(&accepted, SystemTime::now());
            Ok(())
        }
        Ok(false) => fs::remove_file(claimed),
        Err(error) => Err(error),
    }
}

/// Sets a file's modification time, used as its acceptance timestamp.
fn stamp_modified(path: &Path, time: SystemTime) -> io::Result<()> {
    fs::File::options()
        .write(true)
        .open(path)?
        .set_modified(time)
}

/// Builds a unique temporary sibling name for `filename` inside `directory`.
fn unique_temporary(directory: &Path, filename: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    directory.join(format!("{filename}.{}.{}.tmp", std::process::id(), unique))
}

/// Writes the safe diagnostic for a rejected item and removes the claimed file.
fn reject(
    root: &Path,
    filename: &str,
    claimed: &Path,
    code: &str,
    detail: &str,
    report: &mut DrainReport,
) {
    let rejected_file = root.join(REJECTED).join(filename);
    let temporary = unique_temporary(&root.join(REJECTED), filename);
    let diagnostic = json!({
        "filename": filename,
        "code": code,
        "detail": detail,
        "rejected_at": now_rfc3339(),
    });
    let Ok(text) = serde_json::to_string_pretty(&diagnostic) else {
        let _ = restore_to_pending(root, filename, claimed);
        return;
    };
    if fs::write(&temporary, text).is_err() {
        let _ = restore_to_pending(root, filename, claimed);
        return;
    }
    match move_no_replace(&temporary, &rejected_file) {
        Ok(true) => {
            let _ = fs::remove_file(claimed);
            *report.rejected.entry(code.to_string()).or_insert(0) += 1;
        }
        Ok(false) => {
            let _ = fs::remove_file(&temporary);
            let _ = fs::remove_file(claimed);
            *report.rejected.entry(code.to_string()).or_insert(0) += 1;
        }
        Err(_) => {
            let _ = fs::remove_file(&temporary);
            let _ = restore_to_pending(root, filename, claimed);
        }
    }
}

/// Moves a claimed file back to `pending/`.
fn restore_to_pending(root: &Path, filename: &str, source: &Path) -> bool {
    let pending = root.join(PENDING).join(filename);
    match move_no_replace(source, &pending) {
        Ok(true) => true,
        Ok(false) => fs::remove_file(source).is_ok(),
        Err(_) => false,
    }
}

/// Returns a safe validation detail: schema keywords and counts only.
fn validation_detail(error: &EnvelopeValidationError) -> String {
    if let Some(schema_detail) = error.schema_detail() {
        return format!("schema: {schema_detail}");
    }
    format!(
        "keywords: {}; errors: {}",
        error.keywords().join(","),
        error.error_count()
    )
}

/// Lists `*.json` files in `dir`, sorted for deterministic processing.
fn json_files(dir: &Path) -> Result<Vec<PathBuf>, OutboxError> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let is_json = path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("json"));
        if path.is_file() && is_json {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

/// Removes `*.tmp` files left by an interrupted writer.
fn remove_orphan_tmp(dir: &Path) -> Result<(), OutboxError> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let is_tmp = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.to_ascii_lowercase().ends_with(".tmp"));
        if path.is_file() && is_tmp {
            let _ = fs::remove_file(&path);
        }
    }
    Ok(())
}

/// Removes `*.json` items older than `retention`, skipping `exempt` names (for
/// `accepted/`, the items accepted in this pass).
fn prune_older(
    dir: &Path,
    retention: Duration,
    exempt: &BTreeSet<String>,
) -> Result<usize, OutboxError> {
    let mut pruned = 0;
    let now = SystemTime::now();
    for path in json_files(dir)? {
        let Some(filename) = file_name(&path) else {
            continue;
        };
        if exempt.contains(&filename) {
            continue;
        }
        let Ok(modified) = fs::metadata(&path).and_then(|metadata| metadata.modified()) else {
            continue;
        };
        let Ok(age) = now.duration_since(modified) else {
            continue;
        };
        if age > retention && fs::remove_file(&path).is_ok() {
            pruned += 1;
        }
    }
    Ok(pruned)
}

/// Returns the file name of `path` as a `String`, if it is valid UTF-8.
fn file_name(path: &Path) -> Option<String> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
}

/// Renames `from` to `to` without deleting an existing destination.
fn move_no_replace(from: &Path, to: &Path) -> io::Result<bool> {
    match fs::metadata(to) {
        Ok(metadata) if metadata.is_file() => return Ok(false),
        Ok(_) => {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("destination {} is not a regular file", to.display()),
            ))
        }
        Err(_) => {}
    }
    fs::rename(from, to)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::{drain, retry_stalled, unique_temporary, DrainReport, OutboxError};
    use crate::captures::{CaptureApi, IngestError, IngestOutcome, Receipt};
    use integration_contracts::capture::{artifact_fingerprint, CaptureEnvelope};
    use std::collections::HashMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Mutex;
    use std::time::{Duration, SystemTime};

    const RETENTION: Duration = Duration::from_secs(7 * 24 * 60 * 60);

    fn temporary_directory(tag: &str) -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let directory = std::env::temp_dir().join(format!(
            "xemnas-outbox-{tag}-{}-{nanos}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).expect("create temporary directory");
        directory
    }

    fn filename() -> String {
        format!("{}.json", "a".repeat(64))
    }

    fn valid_envelope_json(idempotency_key: &str, canonical_path: &str) -> String {
        let content = "synthetic content";
        serde_json::json!({
            "schema_version": 1,
            "capture_id": "018f2d3c-4b5a-7c6d-8e9f-000000000001",
            "idempotency_key": idempotency_key,
            "source": {
                "adapter": "opencode",
                "adapter_version": "0.0.0",
                "session_id": "session-1",
                "message_id": "message-1"
            },
            "project": { "canonical_path": canonical_path },
            "observed_at": "2026-01-01T00:00:00Z",
            "artifacts": [{
                "artifact_id": "018f2d3c-4b5a-7c6d-8e9f-000000000101",
                "kind": "user_text",
                "content": content,
                "metadata": {},
                "fingerprint": artifact_fingerprint(content)
            }]
        })
        .to_string()
    }

    fn write_pending(root: &Path, filename: &str, content: &str) -> PathBuf {
        let path = root.join("pending").join(filename);
        fs::create_dir_all(path.parent().expect("parent")).expect("create pending");
        fs::write(&path, content).expect("write pending");
        path
    }

    fn set_mtime(path: &Path, time: SystemTime) {
        fs::File::options()
            .write(true)
            .open(path)
            .expect("open for mtime")
            .set_modified(time)
            .expect("set mtime");
    }

    #[derive(Default)]
    struct AcceptingApi {
        seen: Mutex<HashMap<String, Receipt>>,
    }

    impl AcceptingApi {
        fn seen_count(&self) -> usize {
            self.seen
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .len()
        }
    }

    impl CaptureApi for AcceptingApi {
        fn ingest(
            &self,
            envelope: &CaptureEnvelope,
            idempotency_key: &str,
        ) -> Result<IngestOutcome, IngestError> {
            let mut seen = self
                .seen
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(receipt) = seen.get(idempotency_key) {
                return Ok(IngestOutcome {
                    receipt: receipt.clone(),
                    replayed: true,
                });
            }
            let receipt = Receipt {
                capture_id: envelope.capture_id.clone(),
                idempotency_key: idempotency_key.to_string(),
                received_at: "2026-01-01T00:00:00Z".to_string(),
                artifact_count: envelope.artifacts.len() as i64,
            };
            seen.insert(idempotency_key.to_string(), receipt.clone());
            Ok(IngestOutcome {
                receipt,
                replayed: false,
            })
        }

        fn receipt(&self, _capture_id: &str) -> Result<Receipt, IngestError> {
            Err(IngestError::NotFound)
        }
    }

    struct FailingApi(IngestError);

    impl CaptureApi for FailingApi {
        fn ingest(
            &self,
            _envelope: &CaptureEnvelope,
            _idempotency_key: &str,
        ) -> Result<IngestOutcome, IngestError> {
            Err(self.0.clone())
        }

        fn receipt(&self, _capture_id: &str) -> Result<Receipt, IngestError> {
            Err(IngestError::NotFound)
        }
    }

    #[test]
    fn accepts_valid_envelope_and_dedupes_on_the_second_drain() {
        let root = temporary_directory("accept");
        let api = AcceptingApi::default();
        let name = filename();
        write_pending(&root, &name, &valid_envelope_json("key-1", "C:/proj"));

        let first = drain(&root, &api, RETENTION).expect("drain");
        assert_eq!(first.accepted, 1);
        assert_eq!(first.pending_remaining, 0);
        assert!(root.join("accepted").join(&name).exists());
        assert_eq!(api.seen_count(), 1);

        fs::copy(
            root.join("accepted").join(&name),
            root.join("pending").join(&name),
        )
        .expect("copy back");
        let second = drain(&root, &api, RETENTION).expect("drain");
        assert_eq!(second.accepted, 1, "replay is still an accepted item");
        assert_eq!(second.pending_remaining, 0);
        assert_eq!(api.seen_count(), 1, "deduplicated by idempotency key");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn invalid_json_is_rejected_without_envelope_content() {
        let root = temporary_directory("invalid-json");
        let name = filename();
        write_pending(&root, &name, "{ \"secret\": \"SECRET_MARKER_XYZ\", broken ");

        let report = drain(&root, &AcceptingApi::default(), RETENTION).expect("drain");
        assert_eq!(report.rejected.get("invalid_json"), Some(&1));
        let diagnostic = fs::read_to_string(root.join("rejected").join(&name)).expect("diagnostic");
        assert!(diagnostic.contains("invalid_json"));
        assert!(
            !diagnostic.contains("SECRET_MARKER_XYZ"),
            "diagnostic must not carry the envelope content: {diagnostic}"
        );
        assert!(!root.join("pending").join(&name).exists());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn schema_invalid_is_rejected_with_keywords_and_without_instance() {
        let root = temporary_directory("schema-invalid");
        let name = filename();
        let mut value: serde_json::Value =
            serde_json::from_str(&valid_envelope_json("key-1", "C:/proj")).expect("parse");
        value["observed_at"] = serde_json::json!("SECRET_MARKER_XYZ");
        write_pending(&root, &name, &value.to_string());

        let report = drain(&root, &AcceptingApi::default(), RETENTION).expect("drain");
        assert_eq!(report.rejected.get("schema_invalid"), Some(&1));
        let diagnostic = fs::read_to_string(root.join("rejected").join(&name)).expect("diagnostic");
        assert!(diagnostic.contains("keywords"));
        assert!(
            !diagnostic.contains("SECRET_MARKER_XYZ"),
            "diagnostic must not carry the instance: {diagnostic}"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn transient_failure_returns_the_item_to_pending() {
        let root = temporary_directory("transient");
        let name = filename();
        write_pending(&root, &name, &valid_envelope_json("key-1", "C:/proj"));

        let error = IngestError::Storage("busy".to_string());
        let report = drain(&root, &FailingApi(error), RETENTION).expect("drain");
        assert_eq!(report.accepted, 0);
        assert_eq!(report.pending_remaining, 1);
        assert!(root.join("pending").join(&name).exists());
        assert!(!root.join("sending").join(&name).exists());
        assert!(!root.join("rejected").join(&name).exists());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn rejected_items_are_recorded_with_the_right_code() {
        for (error, code) in [
            (IngestError::Conflict, "artifact_id_conflict"),
            (IngestError::NotFound, "ingest_not_found"),
            (IngestError::InvalidFingerprint, "invalid_fingerprint"),
        ] {
            let root = temporary_directory("reject-code");
            let name = filename();
            write_pending(&root, &name, &valid_envelope_json("key-1", "C:/proj"));

            let report = drain(&root, &FailingApi(error), RETENTION).expect("drain");
            assert_eq!(report.rejected.get(code), Some(&1), "code {code}");
            assert_eq!(report.accepted, 0);
            let diagnostic =
                fs::read_to_string(root.join("rejected").join(&name)).expect("diagnostic");
            assert!(
                diagnostic.contains(code),
                "diagnostic for {code}: {diagnostic}"
            );
            assert!(!root.join("pending").join(&name).exists());
            assert!(!root.join("sending").join(&name).exists());
            let leftovers: Vec<_> = fs::read_dir(root.join("rejected"))
                .expect("rejected dir")
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
                .collect();
            assert!(leftovers.is_empty(), "no temporary left behind");

            let _ = fs::remove_dir_all(&root);
        }
    }

    #[test]
    fn rejected_diagnostics_use_unique_temporary_names() {
        let root = temporary_directory("tmp-name");
        let directory = root.join("rejected");
        fs::create_dir_all(&directory).expect("rejected dir");
        let name = filename();

        let first = unique_temporary(&directory, &name);
        let second = unique_temporary(&directory, &name);

        let first_name = first
            .file_name()
            .expect("file name")
            .to_string_lossy()
            .into_owned();
        let second_name = second
            .file_name()
            .expect("file name")
            .to_string_lossy()
            .into_owned();
        assert!(first_name.starts_with(&format!("{name}.")));
        assert!(first_name.ends_with(".tmp"));
        assert!(first_name.contains(&format!(".{}.", std::process::id())));
        assert_ne!(first_name, second_name);
        assert_eq!(first.parent(), Some(directory.as_path()));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn sending_is_recovered_and_imported() {
        let root = temporary_directory("recover");
        let api = AcceptingApi::default();
        let name = filename();
        let sending = root.join("sending");
        fs::create_dir_all(&sending).expect("create sending");
        fs::write(sending.join(&name), valid_envelope_json("key-1", "C:/proj"))
            .expect("write sending");

        let report = drain(&root, &api, RETENTION).expect("drain");
        assert_eq!(report.sending_recovered, 1);
        assert_eq!(report.accepted, 1);
        assert!(root.join("accepted").join(&name).exists());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn orphan_tmp_is_removed_and_non_json_is_kept() {
        let root = temporary_directory("tmp");
        fs::create_dir_all(root.join("pending")).expect("pending");
        fs::write(root.join("pending").join("orphan.tmp"), "leftover").expect("tmp");
        fs::write(root.join("pending").join("notes.txt"), "keep me").expect("notes");

        let report = drain(&root, &AcceptingApi::default(), RETENTION).expect("drain");
        assert_eq!(report.pending_remaining, 0);
        assert!(!root.join("pending").join("orphan.tmp").exists());
        assert!(root.join("pending").join("notes.txt").exists());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn retention_prunes_old_accepted_only() {
        let root = temporary_directory("retention");
        for directory in ["accepted", "rejected"] {
            fs::create_dir_all(root.join(directory)).expect("directory");
        }
        let old = root.join("accepted").join("old.json");
        let recent = root.join("accepted").join("recent.json");
        let rejected = root.join("rejected").join("keep.json");
        for path in [&old, &recent, &rejected] {
            fs::write(path, "{}").expect("write");
        }
        let ten_days_ago = SystemTime::now() - Duration::from_secs(10 * 24 * 60 * 60);
        for path in [&old, &rejected] {
            set_mtime(path, ten_days_ago);
        }

        let report = drain(&root, &AcceptingApi::default(), RETENTION).expect("drain");
        assert_eq!(report.pruned, 1);
        assert!(!old.exists(), "old accepted item must be pruned");
        assert!(recent.exists(), "recent accepted item stays");
        assert!(rejected.exists(), "rejected diagnostics are never pruned");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn an_old_pending_item_is_not_pruned_right_after_acceptance() {
        let root = temporary_directory("accept-age");
        let name = filename();
        let pending = write_pending(&root, &name, &valid_envelope_json("key-1", "C:/proj"));
        set_mtime(
            &pending,
            SystemTime::now() - Duration::from_secs(30 * 24 * 60 * 60),
        );

        let report = drain(&root, &AcceptingApi::default(), RETENTION).expect("drain");
        assert_eq!(report.accepted, 1);
        assert_eq!(
            report.pruned, 0,
            "a freshly accepted item must not be pruned"
        );
        assert!(root.join("accepted").join(&name).exists());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn rejected_write_failure_leaves_the_item_pending() {
        let root = temporary_directory("reject-failure");
        let name = filename();
        write_pending(&root, &name, &valid_envelope_json("key-1", "C:/proj"));
        fs::create_dir_all(root.join("rejected").join(&name)).expect("blocking directory");

        let report = drain(
            &root,
            &FailingApi(IngestError::InvalidFingerprint),
            RETENTION,
        )
        .expect("drain");
        assert!(report.rejected.is_empty());
        assert!(
            root.join("pending").join(&name).exists(),
            "item is not lost"
        );
        assert!(!root.join("sending").join(&name).exists());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn outbox_error_display_is_stable() {
        let error = OutboxError::Io("boom".to_string());
        assert!(error.to_string().contains("boom"));
        let _: &dyn std::error::Error = &error;
        let _ = DrainReport::default();
    }

    #[test]
    fn unregistered_project_is_rejected_on_the_first_drain() {
        let root = temporary_directory("unregistered");
        let name = filename();
        write_pending(&root, &name, &valid_envelope_json("key-1", "C:/proj"));

        let report = drain(&root, &FailingApi(IngestError::Forbidden), RETENTION).expect("drain");
        assert_eq!(report.rejected.get("project_not_registered"), Some(&1));
        assert_eq!((report.accepted, report.pending_remaining), (0, 0));
        let diagnostic = fs::read_to_string(root.join("rejected").join(&name)).expect("diagnostic");
        assert!(diagnostic.contains("project_not_registered"));
        assert!(!root.join("pending").join(&name).exists());
        assert!(!root.join("stalled").join(&name).exists());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn storage_failures_keep_retrying_without_parking() {
        let root = temporary_directory("storage-retry");
        let name = filename();
        write_pending(&root, &name, &valid_envelope_json("key-1", "C:/proj"));
        let failing = FailingApi(IngestError::Storage("busy".to_string()));
        for _ in 0..8 {
            let report = drain(&root, &failing, RETENTION).expect("drain");
            assert_eq!(report.pending_remaining, 1);
        }
        assert!(!root.join("stalled").join(&name).exists());
        let _ = fs::remove_dir_all(&root);
    }

    fn park_legacy_stalled(root: &Path, name: &str, content: &str) {
        let stalled = root.join("stalled");
        fs::create_dir_all(&stalled).expect("stalled dir");
        fs::write(stalled.join(name), content).expect("stalled item");
    }

    #[test]
    fn retry_restores_items_parked_by_older_versions() {
        let root = temporary_directory("stall-legacy");
        let name = filename();
        park_legacy_stalled(&root, &name, &valid_envelope_json("key-1", "C:/proj"));

        assert_eq!(retry_stalled(&root).expect("retry"), 1);
        assert!(root.join("pending").join(&name).exists());
        assert!(!root.join("stalled").join(&name).exists());
        let accepted = drain(&root, &AcceptingApi::default(), RETENTION).expect("drain");
        assert_eq!(accepted.accepted, 1);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_newer_pending_item_supersedes_its_stalled_copy_on_retry() {
        let root = temporary_directory("stall-superseded");
        let name = filename();
        park_legacy_stalled(&root, &name, &valid_envelope_json("key-1", "C:/proj"));
        let newer = valid_envelope_json("key-1", "C:/newer");
        write_pending(&root, &name, &newer);

        assert_eq!(retry_stalled(&root).expect("retry"), 0);
        assert!(!root.join("stalled").join(&name).exists());
        assert_eq!(
            fs::read_to_string(root.join("pending").join(&name)).expect("pending"),
            newer
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn retry_without_stalled_directory_is_a_no_op() {
        let root = temporary_directory("no-stalled");
        assert_eq!(retry_stalled(&root).expect("retry"), 0);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn rejected_diagnostics_older_than_the_retention_are_pruned() {
        let root = temporary_directory("rejected-retention");
        let rejected = root.join("rejected");
        fs::create_dir_all(&rejected).expect("rejected dir");
        let old = rejected.join("old.json");
        let recent = rejected.join("recent.json");
        fs::write(&old, "{}").expect("old");
        fs::write(&recent, "{}").expect("recent");
        set_mtime(
            &old,
            SystemTime::now() - Duration::from_secs(31 * 24 * 60 * 60),
        );

        let report = drain(&root, &AcceptingApi::default(), RETENTION).expect("drain");
        assert_eq!(report.pruned_rejected, 1);
        assert!(!old.exists());
        assert!(recent.exists());
        let _ = fs::remove_dir_all(&root);
    }
}
