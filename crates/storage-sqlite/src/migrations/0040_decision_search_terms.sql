-- Search terms of adopted decisions (application::search_terms): words a
-- search would use that the decision's text does not. A row with an empty
-- array means the decision was processed and got no term.
CREATE TABLE decision_search_terms (
    decision_id TEXT PRIMARY KEY REFERENCES engineering_decisions(decision_id) ON DELETE CASCADE,
    terms TEXT NOT NULL,
    model TEXT NOT NULL,
    created_at TEXT NOT NULL
);

-- FTS5 cannot add a column: rebuild the index with one for the terms.
CREATE TABLE decisions_fts_rebuild AS
    SELECT decision_id, question, choice, rationale FROM decisions_fts;
DROP TABLE decisions_fts;
CREATE VIRTUAL TABLE decisions_fts USING fts5(
    decision_id UNINDEXED, question, choice, rationale, search_terms
);
INSERT INTO decisions_fts (decision_id, question, choice, rationale)
    SELECT decision_id, question, choice, rationale FROM decisions_fts_rebuild;
DROP TABLE decisions_fts_rebuild;
