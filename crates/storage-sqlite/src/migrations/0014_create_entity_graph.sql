-- Knowledge graph of a project (ADR-0005). Rows are never deleted by the
-- app: entities are retired, edges are invalidated.
CREATE TABLE entities (
    entity_id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    kind TEXT NOT NULL CHECK (kind IN ('component', 'technology')),
    name TEXT NOT NULL,
    key TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL,
    retired_at TEXT
);

CREATE INDEX idx_entities_project ON entities(project_id, kind, key);

CREATE TABLE entity_patterns (
    entity_id TEXT NOT NULL REFERENCES entities(entity_id),
    position INTEGER NOT NULL,
    pattern TEXT NOT NULL,
    PRIMARY KEY (entity_id, position)
);

CREATE TABLE entity_aliases (
    entity_id TEXT NOT NULL REFERENCES entities(entity_id),
    position INTEGER NOT NULL,
    alias TEXT NOT NULL,
    PRIMARY KEY (entity_id, position)
);

CREATE TABLE entity_edges (
    edge_id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    kind TEXT NOT NULL CHECK (kind IN ('affects', 'uses', 'applies_to', 'part_of')),
    source_kind TEXT NOT NULL CHECK (source_kind IN ('decision', 'claim', 'entity')),
    source_id TEXT NOT NULL,
    entity_id TEXT NOT NULL REFERENCES entities(entity_id),
    origin TEXT NOT NULL CHECK (origin IN ('human', 'derived')),
    reason TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL,
    confirmed_at TEXT,
    invalidated_at TEXT
);

CREATE INDEX idx_entity_edges_project ON entity_edges(project_id, created_at);
CREATE INDEX idx_entity_edges_entity ON entity_edges(entity_id);

-- One live edge per (kind, source, entity); invalidated rows stay as history.
CREATE UNIQUE INDEX idx_entity_edges_live
    ON entity_edges(kind, source_kind, source_id, entity_id) WHERE invalidated_at IS NULL;
