CREATE TABLE engineering_decisions (
    decision_id TEXT PRIMARY KEY,
    candidate_id TEXT NOT NULL UNIQUE REFERENCES decision_candidates(id),
    project_id TEXT NOT NULL REFERENCES projects(id),
    capture_id TEXT,
    status TEXT NOT NULL CHECK (status IN ('accepted','superseded')),
    question TEXT NOT NULL,
    choice TEXT NOT NULL,
    rationale TEXT NOT NULL,
    assumptions TEXT NOT NULL DEFAULT '[]',
    reconsider_when TEXT NOT NULL DEFAULT '[]',
    scope TEXT NOT NULL DEFAULT '[]',
    consequences TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL,
    confirmed_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE decision_revisions (
    decision_id TEXT NOT NULL REFERENCES engineering_decisions(decision_id),
    version INTEGER NOT NULL,
    question TEXT NOT NULL,
    choice TEXT NOT NULL,
    rationale TEXT NOT NULL,
    assumptions TEXT NOT NULL DEFAULT '[]',
    reconsider_when TEXT NOT NULL DEFAULT '[]',
    scope TEXT NOT NULL DEFAULT '[]',
    consequences TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL,
    UNIQUE (decision_id, version)
);

CREATE TABLE evidence_links (
    decision_id TEXT NOT NULL REFERENCES engineering_decisions(decision_id) ON DELETE CASCADE,
    artifact_id TEXT NOT NULL,
    position INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE (decision_id, artifact_id)
);

CREATE VIRTUAL TABLE decisions_fts USING fts5(decision_id UNINDEXED, question, choice, rationale);

CREATE INDEX idx_engineering_decisions_project ON engineering_decisions(project_id);
CREATE INDEX idx_engineering_decisions_confirmed ON engineering_decisions(confirmed_at, decision_id);
