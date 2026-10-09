-- Who confirmed and who invalidated each edge of the map. Until now a link the
-- automatic review accepted looked like one a person made (origin 'human'), and
-- a rule that inherited its tie from a decision was labelled the same way.
-- confirmed_by / invalidated_by tell the machine's guesses from a person's
-- word: a derivation may revise the first and never the second.
ALTER TABLE entity_edges ADD COLUMN confirmed_by TEXT
    CHECK (confirmed_by IN ('person', 'rules', 'ai', 'inherited'));
ALTER TABLE entity_edges ADD COLUMN invalidated_by TEXT
    CHECK (invalidated_by IN ('person', 'rules', 'ai', 'inherited'));

-- (a) A link the automatic review settled: its ledger entry names the actor.
UPDATE entity_edges SET confirmed_by = (
        SELECT r.decided_by FROM auto_reviews r
        WHERE r.item_kind = 'link' AND r.item_id = entity_edges.edge_id
          AND r.verdict = 'accepted' AND r.undone_at IS NULL)
WHERE confirmed_at IS NOT NULL AND EXISTS (
        SELECT 1 FROM auto_reviews r
        WHERE r.item_kind = 'link' AND r.item_id = entity_edges.edge_id
          AND r.verdict = 'accepted' AND r.undone_at IS NULL);

UPDATE entity_edges SET invalidated_by = (
        SELECT r.decided_by FROM auto_reviews r
        WHERE r.item_kind = 'link' AND r.item_id = entity_edges.edge_id
          AND r.verdict = 'discarded' AND r.undone_at IS NULL)
WHERE invalidated_at IS NOT NULL AND EXISTS (
        SELECT 1 FROM auto_reviews r
        WHERE r.item_kind = 'link' AND r.item_id = entity_edges.edge_id
          AND r.verdict = 'discarded' AND r.undone_at IS NULL);

-- (b) A rule the review adopted took the ties of its decision as "human"
-- edges with no reason: they were inherited.
UPDATE entity_edges
SET origin = 'derived', reason = 'herdado da decisão de origem', confirmed_by = 'inherited'
WHERE kind = 'applies_to' AND source_kind = 'claim' AND origin = 'human' AND reason = ''
  AND confirmed_at IS NOT NULL AND confirmed_by IS NULL
  AND source_id IN (
        SELECT r.result_id FROM auto_reviews r
        WHERE r.item_kind = 'claim' AND r.verdict = 'accepted' AND r.undone_at IS NULL
          AND r.result_id IS NOT NULL);

-- (c) The ties an adoption by the review linked, made within five minutes of
-- the review's entry for the candidate. The window can relabel an edge a
-- person made just after an adoption; only the label is affected.
UPDATE entity_edges
SET origin = 'derived', confirmed_by = (
        SELECT r.decided_by FROM auto_reviews r
        WHERE r.item_kind = 'candidate' AND r.verdict = 'accepted' AND r.undone_at IS NULL
          AND r.result_id = entity_edges.source_id
          AND abs(julianday(entity_edges.created_at) - julianday(r.created_at)) * 86400 <= 300
        LIMIT 1)
WHERE origin = 'human' AND reason = '' AND confirmed_at IS NOT NULL AND confirmed_by IS NULL
  AND EXISTS (
        SELECT 1 FROM auto_reviews r
        WHERE r.item_kind = 'candidate' AND r.verdict = 'accepted' AND r.undone_at IS NULL
          AND r.result_id = entity_edges.source_id
          AND abs(julianday(entity_edges.created_at) - julianday(r.created_at)) * 86400 <= 300);

-- (d) Ties a rule inherited, without an actor.
UPDATE entity_edges SET confirmed_by = 'inherited'
WHERE reason = 'herdado da decisão de origem' AND confirmed_at IS NOT NULL
  AND confirmed_by IS NULL;

-- (e) Everything else was confirmed or removed by hand.
UPDATE entity_edges SET confirmed_by = 'person'
WHERE confirmed_at IS NOT NULL AND confirmed_by IS NULL;
UPDATE entity_edges SET invalidated_by = 'person'
WHERE invalidated_at IS NOT NULL AND invalidated_by IS NULL;
