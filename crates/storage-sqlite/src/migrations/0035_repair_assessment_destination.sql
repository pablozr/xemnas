-- The typed migration-35 hook inspects PRAGMA table_info(assessments) and adds
-- only missing v22 destination columns in this migration's transaction.
-- Historical attempt remains NULL (unknown), reason 'unknown', and destination
-- counters 0, exactly as in v22; no historical attempts/counts are inferred.
-- Existing columns and rows are untouched. The version marker is written only
-- after the hook succeeds; unrelated missing base columns fail closed.
SELECT 1;
