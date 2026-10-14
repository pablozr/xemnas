-- Why a routing judgment failed: a short code (`provider_extractor`,
-- `answer_unparsable`, `snapshot_changed`...) and, for a provider error only,
-- a sanitized detail of at most 200 characters. Never request or answer
-- content. NULL for completed entries and for failures before this column.
ALTER TABLE context_routing_entries ADD COLUMN failure_code TEXT;
ALTER TABLE context_routing_entries ADD COLUMN failure_detail TEXT;
