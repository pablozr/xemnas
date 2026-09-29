CREATE TABLE IF NOT EXISTS adapter_checkpoints (
    adapter TEXT NOT NULL,
    session_id TEXT NOT NULL,
    message_id TEXT NOT NULL,
    capture_id TEXT NOT NULL,
    observed_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (adapter, session_id),
    FOREIGN KEY (capture_id) REFERENCES capture_receipts (capture_id) ON DELETE CASCADE
);
