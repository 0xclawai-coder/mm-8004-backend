-- Backfill empty agent owners from Registered activity events.
-- URIUpdated/MetadataSet upserts previously overwrote owner with empty string.
UPDATE agents a
SET owner = (al.event_data->>'owner')
FROM activity_log al
WHERE al.agent_id = a.agent_id
  AND al.chain_id = a.chain_id
  AND al.event_type = 'Registered'
  AND (a.owner IS NULL OR a.owner = '')
  AND al.event_data->>'owner' IS NOT NULL
  AND al.event_data->>'owner' != '';
