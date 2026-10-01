-- Decision, rule or detail, and how much a candidate matters (noise
-- reduction). Existing candidates read as decisions that matter.
ALTER TABLE decision_candidates ADD COLUMN kind TEXT NOT NULL DEFAULT 'decision'
    CHECK (kind IN ('decision', 'rule'));
ALTER TABLE decision_candidates ADD COLUMN significance REAL NOT NULL DEFAULT 1.0
    CHECK (significance >= 0.0 AND significance <= 1.0);
ALTER TABLE decision_candidates ADD COLUMN criteria TEXT NOT NULL DEFAULT '[]';
