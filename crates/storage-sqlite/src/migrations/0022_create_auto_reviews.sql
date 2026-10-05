-- Automatic review (docs/pesquisas/exibicao-e-aprovacao.md): one switch that
-- lets the AI approve what waits for a person. The mode lives in
-- approval_settings (0021); what the review decided goes into a ledger, and
-- every call to the AI is logged so the calls can be spaced and capped.
DROP TABLE IF EXISTS auto_approvals;

CREATE TABLE auto_reviews (
    item_kind TEXT NOT NULL CHECK (item_kind IN ('candidate', 'relation', 'claim', 'link')),
    item_id TEXT NOT NULL,
    project_id TEXT NOT NULL REFERENCES projects(id),
    verdict TEXT NOT NULL CHECK (verdict IN ('accepted', 'discarded', 'needs_human')),
    decided_by TEXT NOT NULL CHECK (decided_by IN ('rules', 'ai')),
    reason TEXT NOT NULL,
    title TEXT NOT NULL,
    result_id TEXT,
    created_at TEXT NOT NULL,
    undone_at TEXT,
    PRIMARY KEY (item_kind, item_id)
);

CREATE INDEX idx_auto_reviews_project ON auto_reviews(project_id, created_at);

CREATE TABLE auto_review_calls (
    call_id INTEGER PRIMARY KEY AUTOINCREMENT,
    called_at TEXT NOT NULL,
    items INTEGER NOT NULL,
    succeeded INTEGER NOT NULL CHECK (succeeded IN (0, 1))
);

CREATE INDEX idx_auto_review_calls_at ON auto_review_calls(called_at);
