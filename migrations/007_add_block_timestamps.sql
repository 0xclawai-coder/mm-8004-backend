-- Add block_timestamp to all event-sourced tables.
-- This stores the actual on-chain block timestamp (from the block header),
-- as opposed to created_at which is the DB insertion time.

ALTER TABLE agents ADD COLUMN IF NOT EXISTS block_timestamp TIMESTAMPTZ;
ALTER TABLE feedbacks ADD COLUMN IF NOT EXISTS block_timestamp TIMESTAMPTZ;
ALTER TABLE feedback_responses ADD COLUMN IF NOT EXISTS block_timestamp TIMESTAMPTZ;
ALTER TABLE activity_log ADD COLUMN IF NOT EXISTS block_timestamp TIMESTAMPTZ;
