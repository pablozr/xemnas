-- The reason a failed analysis gives, kept next to its short error_code: the
-- error's own message only (sanitized, at most 200 characters), never prompt
-- or document content.
ALTER TABLE assessments ADD COLUMN error_detail TEXT;
