CREATE TABLE project_context_settings (
    project_id TEXT PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
    mode TEXT NOT NULL CHECK (mode IN ('off', 'shadow', 'inject')),
    budget_tokens INTEGER CHECK (budget_tokens IS NULL OR budget_tokens BETWEEN 50 AND 2000),
    updated_at TEXT NOT NULL
);
