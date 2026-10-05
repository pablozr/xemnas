CREATE TABLE capture_episode_sources (
    capture_id TEXT PRIMARY KEY REFERENCES capture_receipts(capture_id) ON DELETE CASCADE,
    provenance TEXT NOT NULL
);
