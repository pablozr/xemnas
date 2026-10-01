-- Context (assumptions, constraints, conventions, goals) derived from an
-- adopted decision, with the sentence it rests on; a person confirms or
-- rejects each, and a decision never gets the same statement twice.
CREATE TABLE claim_suggestions (
    suggestion_id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    decision_id TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('assumption', 'constraint', 'goal', 'convention')),
    statement TEXT NOT NULL,
    quote TEXT NOT NULL,
    created_at TEXT NOT NULL,
    outcome TEXT CHECK (outcome IN ('confirmed', 'rejected')),
    resolved_at TEXT,
    UNIQUE (decision_id, statement)
);

CREATE INDEX idx_claim_suggestions_project ON claim_suggestions(project_id, outcome);
