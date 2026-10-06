-- 0027 added claim_suggestions.source_version without filling the rows that
-- already existed, and confirming a suggestion requires the version to match
-- its source decision, so every pre-0027 pending suggestion was stuck.
-- Backfill only what is provably safe: a decision never revised (version 1)
-- is the only version the suggestion can have come from, and a quote still
-- found verbatim in the current text still rests on it. The rest stays NULL.
UPDATE claim_suggestions
SET source_version = (
    SELECT d.version FROM engineering_decisions d
    WHERE d.decision_id = claim_suggestions.decision_id
)
WHERE outcome IS NULL
  AND source_version IS NULL
  AND EXISTS (
    SELECT 1 FROM engineering_decisions d
    WHERE d.decision_id = claim_suggestions.decision_id
      AND (
        d.version = 1
        OR instr(d.question, claim_suggestions.quote) > 0
        OR instr(d.choice, claim_suggestions.quote) > 0
        OR instr(d.rationale, claim_suggestions.quote) > 0
      )
  );
