CREATE TABLE observation_deliveries (
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    session_id TEXT NOT NULL,
    mode TEXT NOT NULL CHECK (mode IN ('inject', 'shadow')),
    observation_id TEXT NOT NULL,
    version INTEGER NOT NULL CHECK (version > 0),
    delivered_json TEXT NOT NULL,
    correction_json TEXT,
    PRIMARY KEY (project_id, session_id, mode, observation_id, version)
);
