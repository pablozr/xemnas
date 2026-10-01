-- Generated project overview (summary and main flows), one per project,
-- replaced on each regeneration.
CREATE TABLE project_overviews (
    project_id TEXT PRIMARY KEY REFERENCES projects(id),
    generated_at TEXT NOT NULL,
    content TEXT NOT NULL
);
