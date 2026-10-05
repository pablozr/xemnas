CREATE TABLE review_targets (
    process_id TEXT PRIMARY KEY,
    fingerprint TEXT NOT NULL REFERENCES review_groups(fingerprint) ON DELETE CASCADE,
    representative_id TEXT NOT NULL REFERENCES decision_candidates(id) ON DELETE CASCADE,
    target_kind TEXT NOT NULL CHECK(target_kind IN ('decision','claim')),
    target_id TEXT NOT NULL,
    target_version TEXT NOT NULL,
    representative_source TEXT NOT NULL,
    actor TEXT NOT NULL DEFAULT 'unknown',
    outcome TEXT NOT NULL DEFAULT 'already_represented'
);
INSERT INTO review_targets
SELECT process_id,fingerprint,representative_id,'decision',decision_id,
       CAST(decision_version AS TEXT),representative_source,actor,outcome FROM review_journal;
CREATE TABLE review_target_resolutions (
    candidate_id TEXT PRIMARY KEY REFERENCES decision_candidates(id) ON DELETE CASCADE,
    process_id TEXT NOT NULL REFERENCES review_targets(process_id) ON DELETE CASCADE,
    source_fingerprint TEXT NOT NULL
);
INSERT INTO review_target_resolutions SELECT * FROM review_resolutions;
