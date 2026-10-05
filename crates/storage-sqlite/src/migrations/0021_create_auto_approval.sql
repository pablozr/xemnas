-- Approval by bands (docs/pesquisas/exibicao-e-aprovacao.md). The mode is one
-- global setting, manual unless a person turned the automatic mode on, which
-- the application allows only after the confidence of the extractor was
-- measured against what the person accepted.
CREATE TABLE approval_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    mode TEXT NOT NULL CHECK (mode IN ('manual', 'automatic')),
    updated_at TEXT NOT NULL
);

-- What the policy did with a candidate: held until `due_at` and then accepted
-- (`resolved_at` and `decision_id` set), or left for a blind check.
CREATE TABLE auto_approvals (
    candidate_id TEXT PRIMARY KEY REFERENCES decision_candidates(id) ON DELETE CASCADE,
    project_id TEXT NOT NULL REFERENCES projects(id),
    lane TEXT NOT NULL CHECK (lane IN ('held', 'audit')),
    created_at TEXT NOT NULL,
    due_at TEXT,
    resolved_at TEXT,
    decision_id TEXT
);

CREATE INDEX idx_auto_approvals_project ON auto_approvals(project_id, lane, resolved_at);
