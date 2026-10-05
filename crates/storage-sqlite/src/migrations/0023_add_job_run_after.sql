-- A job the AI provider asked to defer (HTTP 429, timeouts) goes back to
-- `queued` with the earliest moment it may be claimed again. NULL means now.
ALTER TABLE jobs ADD COLUMN run_after TEXT;
