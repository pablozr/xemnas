CREATE TABLE decision_relations (
    from_decision_id TEXT NOT NULL REFERENCES engineering_decisions(decision_id),
    to_decision_id TEXT NOT NULL REFERENCES engineering_decisions(decision_id),
    kind TEXT NOT NULL CHECK (kind IN ('supersedes', 'depends_on', 'conflicts_with')),
    created_at TEXT NOT NULL,
    PRIMARY KEY (from_decision_id, to_decision_id, kind),
    CHECK (from_decision_id <> to_decision_id)
);

CREATE INDEX idx_decision_relations_to ON decision_relations(to_decision_id);

CREATE UNIQUE INDEX idx_decision_relations_single_successor
    ON decision_relations(to_decision_id) WHERE kind = 'supersedes';
