-- Project documentation indexed from the project folder (docs/, specs/,
-- ADRs, README...): outline and fingerprint only, replaced per indexing.
CREATE TABLE project_documents (
    project_id TEXT NOT NULL REFERENCES projects(id),
    path TEXT NOT NULL,
    kind TEXT NOT NULL,
    title TEXT NOT NULL,
    headings TEXT NOT NULL,
    excerpt TEXT NOT NULL,
    bytes INTEGER NOT NULL,
    fingerprint TEXT NOT NULL,
    indexed_at TEXT NOT NULL,
    PRIMARY KEY (project_id, path)
);
