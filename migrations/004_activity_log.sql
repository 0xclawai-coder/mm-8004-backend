CREATE TABLE IF NOT EXISTS activity_log (
    id SERIAL PRIMARY KEY,
    agent_id BIGINT NOT NULL,
    chain_id INT NOT NULL,
    event_type TEXT NOT NULL,
    event_data JSONB DEFAULT '{}',
    block_number BIGINT NOT NULL,
    tx_hash TEXT NOT NULL,
    log_index INT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);
CREATE INDEX idx_activity_agent ON activity_log(agent_id, chain_id);
CREATE INDEX idx_activity_type ON activity_log(event_type);
CREATE INDEX idx_activity_block ON activity_log(block_number);
