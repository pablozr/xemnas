-- A rule adopted from Revisão keeps the candidate it came from, so its
-- evidence (files, components) scopes it on the map.
ALTER TABLE context_claims ADD COLUMN source_candidate_id TEXT;
