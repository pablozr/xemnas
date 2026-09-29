CREATE TABLE IF NOT EXISTS capture_receipts (
    capture_id TEXT PRIMARY KEY,
    idempotency_key TEXT UNIQUE NOT NULL,
    canonical_path TEXT NOT NULL,
    received_at TEXT NOT NULL,
    artifact_count INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS capture_artifacts (
    capture_id TEXT NOT NULL,
    artifact_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    content TEXT NOT NULL,
    metadata TEXT NOT NULL,
    fingerprint TEXT NOT NULL,
    PRIMARY KEY (capture_id, artifact_id),
    UNIQUE (capture_id, kind, fingerprint),
    FOREIGN KEY (capture_id) REFERENCES capture_receipts (capture_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_capture_artifacts_capture_id ON capture_artifacts (capture_id);
