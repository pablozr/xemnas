-- Absolute file of a document imported from outside the project folder (a
-- sibling git worktree); NULL for documents found by indexing the folder.
ALTER TABLE project_documents ADD COLUMN source TEXT;
