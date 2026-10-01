-- Relations between decisions proposed after an adoption, with the quote
-- that shows them; a person confirms or rejects each, and a (from, to, kind)
-- is never suggested twice.
CREATE TABLE relation_suggestions (
    suggestion_id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    from_id TEXT NOT NULL,
    to_id TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('depends_on', 'conflicts_with', 'supersedes')),
    quote TEXT NOT NULL,
    reason TEXT NOT NULL,
    created_at TEXT NOT NULL,
    outcome TEXT CHECK (outcome IN ('confirmed', 'rejected')),
    resolved_at TEXT,
    UNIQUE (from_id, to_id, kind)
);

CREATE INDEX idx_relation_suggestions_project ON relation_suggestions(project_id, outcome);
