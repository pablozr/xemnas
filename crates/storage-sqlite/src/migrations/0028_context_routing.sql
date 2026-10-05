CREATE TABLE context_routing_entries (
 key TEXT PRIMARY KEY,
 project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
 snapshot TEXT NOT NULL,
 generation INTEGER NOT NULL,
 profile_hash TEXT NOT NULL,
 request_json TEXT,
 result_json TEXT,
 state TEXT NOT NULL CHECK(state IN ('pending','running','completed','failed')),
 created_at INTEGER NOT NULL,
 expires_at INTEGER NOT NULL,
 logical_invocations INTEGER NOT NULL DEFAULT 0 CHECK(logical_invocations BETWEEN 0 AND 1),
 input_chars INTEGER NOT NULL DEFAULT 0,
 estimated_tokens INTEGER NOT NULL DEFAULT 0,
 actual_content_attempts INTEGER
);
CREATE INDEX context_routing_project ON context_routing_entries(project_id, created_at);
