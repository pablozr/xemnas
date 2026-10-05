ALTER TABLE context_claims ADD COLUMN inherited_scope TEXT NOT NULL DEFAULT '[]';
ALTER TABLE claim_suggestions ADD COLUMN inherited_scope TEXT NOT NULL DEFAULT '[]';
ALTER TABLE claim_suggestions ADD COLUMN source_version INTEGER;
