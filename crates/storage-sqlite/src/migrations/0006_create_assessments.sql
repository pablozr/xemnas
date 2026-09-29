CREATE TABLE IF NOT EXISTS assessments (
    id TEXT PRIMARY KEY,
    capture_id TEXT NOT NULL REFERENCES capture_receipts(capture_id) ON DELETE CASCADE,
    job_id TEXT REFERENCES jobs(id) ON DELETE SET NULL,
    profile_id TEXT NOT NULL,
    adapter TEXT NOT NULL,
    model TEXT,
    policy TEXT NOT NULL,
    consent_preview_hash TEXT,
    input_hash TEXT NOT NULL,
    started_at TEXT NOT NULL,
    finished_at TEXT NOT NULL,
    outcome TEXT NOT NULL CHECK (outcome IN ('ok','empty','failed','skipped')),
    candidates INTEGER NOT NULL DEFAULT 0,
    inserted INTEGER NOT NULL DEFAULT 0,
    error_code TEXT
);
CREATE INDEX IF NOT EXISTS idx_assessments_capture_id ON assessments(capture_id);
CREATE INDEX IF NOT EXISTS idx_assessments_started_at ON assessments(started_at);
