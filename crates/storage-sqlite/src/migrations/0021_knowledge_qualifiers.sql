ALTER TABLE decision_candidates ADD COLUMN qualifiers TEXT NOT NULL DEFAULT '[]';
ALTER TABLE engineering_decisions ADD COLUMN qualifiers TEXT NOT NULL DEFAULT '[]';
ALTER TABLE decision_revisions ADD COLUMN qualifiers TEXT NOT NULL DEFAULT '[]';
ALTER TABLE context_claims ADD COLUMN qualifiers TEXT NOT NULL DEFAULT '[]';
ALTER TABLE claim_suggestions ADD COLUMN qualifiers TEXT NOT NULL DEFAULT '[]';
