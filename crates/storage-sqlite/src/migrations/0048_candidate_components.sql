-- The map components a candidate decision applies to, as the extractor named
-- them, each with a quote copied from the candidate's own text. Only moves
-- forward: candidates extracted before have none.
--
-- `entity_id` has no foreign key to `entities`: the component is checked
-- again when the candidate is adopted, so one retired or removed in between
-- is skipped there instead of blocking the extraction or the adoption.
CREATE TABLE candidate_components (
    candidate_id TEXT NOT NULL REFERENCES decision_candidates(id) ON DELETE CASCADE,
    entity_id TEXT NOT NULL,
    quote TEXT NOT NULL,
    PRIMARY KEY (candidate_id, entity_id)
);
