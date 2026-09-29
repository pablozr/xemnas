CREATE TABLE IF NOT EXISTS decision_candidates (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    capture_id TEXT NOT NULL REFERENCES capture_receipts(capture_id) ON DELETE CASCADE,
    status TEXT NOT NULL CHECK (status IN ('pending','accepted','edited_and_accepted','dismissed','snoozed')),
    question TEXT NOT NULL,
    choice TEXT NOT NULL,
    rationale TEXT NOT NULL,
    signals TEXT NOT NULL,
    confidence REAL NOT NULL CHECK (confidence >= 0.0 AND confidence <= 1.0),
    confidence_reason TEXT NOT NULL,
    evidence_refs TEXT NOT NULL,
    diff_summary TEXT NOT NULL,
    dedup_hash TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_decision_candidates_capture ON decision_candidates(capture_id);
CREATE INDEX IF NOT EXISTS idx_decision_candidates_status ON decision_candidates(status);
