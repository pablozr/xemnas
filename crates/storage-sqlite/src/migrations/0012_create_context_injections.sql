CREATE TABLE context_injections (
    injection_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    project_id TEXT NOT NULL REFERENCES projects(id),
    mode TEXT NOT NULL CHECK (mode IN ('shadow', 'inject')),
    tokens INTEGER NOT NULL CHECK (tokens >= 0),
    omitted INTEGER NOT NULL CHECK (omitted >= 0),
    created_at TEXT NOT NULL
);

CREATE INDEX idx_context_injections_session ON context_injections(session_id, mode);

CREATE TABLE context_injection_items (
    injection_id TEXT NOT NULL REFERENCES context_injections(injection_id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    item_kind TEXT NOT NULL CHECK (item_kind IN ('decision', 'claim')),
    item_id TEXT NOT NULL,
    item_version INTEGER NOT NULL,
    PRIMARY KEY (injection_id, position)
);
