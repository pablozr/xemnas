-- Global job settings, one row like approval_settings: how many AI-provider
-- calls the job lanes may run at once ("análises em paralelo", 1 to 4).
-- Absent row means the default (2). Read at startup.
CREATE TABLE job_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    parallel_analyses INTEGER NOT NULL CHECK (parallel_analyses BETWEEN 1 AND 4),
    updated_at TEXT NOT NULL
);
