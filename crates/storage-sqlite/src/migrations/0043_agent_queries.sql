CREATE TABLE agent_queries (
  query_id   INTEGER PRIMARY KEY AUTOINCREMENT,
  project_id TEXT NOT NULL,
  tool       TEXT NOT NULL CHECK (tool IN ('decision','search','file')),
  outcome    TEXT NOT NULL CHECK (outcome IN ('answered','empty','not_found','ambiguous')),
  chars      INTEGER NOT NULL,
  created_at TEXT NOT NULL
);
CREATE INDEX agent_queries_project_time ON agent_queries(project_id, created_at);
