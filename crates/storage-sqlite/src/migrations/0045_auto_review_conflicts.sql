-- The side a needs_human verdict conflicts with, so Review can show the two
-- items together: another candidate of the same pass, or a decision in force.
ALTER TABLE auto_reviews ADD COLUMN conflicts_kind TEXT
    CHECK (conflicts_kind IN ('candidate', 'decision'));
ALTER TABLE auto_reviews ADD COLUMN conflicts_id TEXT;
