CREATE TABLE IF NOT EXISTS feedbacks (
    id SERIAL PRIMARY KEY,
    agent_id BIGINT NOT NULL,
    chain_id INT NOT NULL,
    client_address TEXT NOT NULL,
    feedback_index BIGINT NOT NULL,
    value NUMERIC NOT NULL,
    value_decimals INT DEFAULT 0,
    tag1 TEXT,
    tag2 TEXT,
    endpoint TEXT,
    feedback_uri TEXT,
    feedback_hash TEXT,
    revoked BOOLEAN DEFAULT false,
    block_number BIGINT NOT NULL,
    tx_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);
CREATE INDEX idx_feedbacks_agent ON feedbacks(agent_id, chain_id);
CREATE INDEX idx_feedbacks_client ON feedbacks(client_address);
