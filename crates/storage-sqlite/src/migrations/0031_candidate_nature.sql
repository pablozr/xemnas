CREATE TABLE candidate_nature (
    candidate_id TEXT PRIMARY KEY REFERENCES decision_candidates(id) ON DELETE CASCADE,
    nature TEXT NOT NULL CHECK(nature IN ('description','inference','normative','unknown')),
    destination TEXT NOT NULL CHECK(destination IN ('review_required','inference_stored','description_verified')),
    observation_reference TEXT,
    actor TEXT NOT NULL DEFAULT 'unknown' CHECK(actor = 'unknown')
);
