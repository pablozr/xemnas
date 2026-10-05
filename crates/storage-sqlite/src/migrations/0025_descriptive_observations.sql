CREATE TABLE observation_refresh (
 project_id TEXT PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
 generation INTEGER NOT NULL, dirty INTEGER NOT NULL,
 capture_trigger TEXT NOT NULL, requested_at TEXT NOT NULL,
 coverage TEXT NOT NULL DEFAULT '{"partial":false,"unknown":true,"verified_sources":0,"missing_sources":0,"failed_sources":0,"quota_exceeded_sources":0}'
);
CREATE TABLE observation_sources (
 source_id TEXT PRIMARY KEY,
 project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
 project_relative_path TEXT NOT NULL, manifest_kind TEXT NOT NULL,
 sha256 TEXT, parser_policy_version TEXT NOT NULL,
 last_checked_at TEXT, last_check_status TEXT NOT NULL,
 UNIQUE(project_id, project_relative_path)
);
CREATE TABLE observation_records (
 observation_id TEXT NOT NULL, version INTEGER NOT NULL,
 project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
 source_id TEXT NOT NULL REFERENCES observation_sources(source_id) ON DELETE CASCADE,
 status TEXT NOT NULL, record_json TEXT NOT NULL,
 PRIMARY KEY(observation_id, version)
);
CREATE INDEX observation_records_current ON observation_records(project_id, status);
