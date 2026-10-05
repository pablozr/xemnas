CREATE TABLE review_groups (
    fingerprint TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE
);
CREATE TABLE review_group_members (
    candidate_id TEXT PRIMARY KEY REFERENCES decision_candidates(id) ON DELETE CASCADE,
    fingerprint TEXT NOT NULL REFERENCES review_groups(fingerprint) ON DELETE CASCADE,
    source_fingerprint TEXT NOT NULL
);
CREATE INDEX review_members_group ON review_group_members(fingerprint);
CREATE TABLE review_journal (
    process_id TEXT PRIMARY KEY,
    fingerprint TEXT NOT NULL REFERENCES review_groups(fingerprint) ON DELETE CASCADE,
    representative_id TEXT NOT NULL REFERENCES decision_candidates(id) ON DELETE CASCADE,
    decision_id TEXT NOT NULL REFERENCES engineering_decisions(decision_id) ON DELETE CASCADE,
    decision_version INTEGER NOT NULL,
    representative_source TEXT NOT NULL,
    actor TEXT NOT NULL CHECK(actor IN ('human','automation','lab','unknown')),
    outcome TEXT NOT NULL CHECK(outcome = 'already_represented')
);
CREATE TABLE review_resolutions (
    candidate_id TEXT PRIMARY KEY REFERENCES decision_candidates(id) ON DELETE CASCADE,
    process_id TEXT NOT NULL REFERENCES review_journal(process_id) ON DELETE CASCADE,
    source_fingerprint TEXT NOT NULL
);
