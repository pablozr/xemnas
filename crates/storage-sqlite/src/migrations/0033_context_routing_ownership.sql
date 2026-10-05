ALTER TABLE context_routing_entries ADD COLUMN owner_job_id TEXT;
CREATE UNIQUE INDEX context_routing_owner ON context_routing_entries(owner_job_id)
 WHERE owner_job_id IS NOT NULL;

-- Original v28 had no exact ownership. Cancel only the deterministic job ID
-- attached to that entry, never another job in the same project queue.
UPDATE jobs SET state='cancelled',updated_at=strftime('%Y-%m-%dT%H:%M:%SZ','now')
 WHERE kind='context_routing' AND state IN ('queued','running')
 AND EXISTS (SELECT 1 FROM context_routing_entries e
             WHERE e.state IN ('pending','running')
             AND jobs.id='context-routing-' || e.key AND jobs.payload=e.project_id);
UPDATE context_routing_entries
 SET state='failed',request_json=NULL,result_json=NULL,
     expires_at=MIN(expires_at,CAST(strftime('%s','now') AS INTEGER)+60)
 WHERE state IN ('pending','running');
