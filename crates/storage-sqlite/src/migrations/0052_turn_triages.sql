-- One row per call of the turn triage (a minimal AI question asked when the
-- user spoke and nothing structural happened). It keeps the verdict, the
-- characters sent and the error code, so the dogfood and the replay can count
-- how many calls run and what they cost. It never stores conversation text.
CREATE TABLE turn_triages (
    id TEXT PRIMARY KEY,
    capture_id TEXT NOT NULL,
    job_id TEXT,
    model TEXT,
    called_at TEXT NOT NULL,
    kind TEXT,
    passed INTEGER NOT NULL CHECK (passed IN (0, 1)),
    input_chars INTEGER NOT NULL,
    error_code TEXT
);

CREATE INDEX idx_turn_triages_called_at ON turn_triages(called_at);
