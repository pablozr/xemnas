-- Adapter version reported by the envelope that last advanced each session
-- checkpoint. Nullable: rows written before this migration never recorded it,
-- and Settings → OpenCode reports those as an unknown version.
ALTER TABLE adapter_checkpoints ADD COLUMN adapter_version TEXT;
