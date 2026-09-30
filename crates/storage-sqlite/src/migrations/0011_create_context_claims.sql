CREATE TABLE context_claims (
    claim_id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    kind TEXT NOT NULL CHECK (kind IN ('assumption', 'constraint', 'goal', 'convention')),
    statement TEXT NOT NULL,
    valid_from TEXT NOT NULL,
    valid_until TEXT,
    source_decision_id TEXT REFERENCES engineering_decisions(decision_id),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (valid_until IS NULL OR valid_until >= valid_from)
);

CREATE INDEX idx_context_claims_project ON context_claims(project_id, valid_from, claim_id);

CREATE VIRTUAL TABLE claims_fts USING fts5(claim_id UNINDEXED, statement);
