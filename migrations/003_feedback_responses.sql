CREATE TABLE IF NOT EXISTS feedback_responses (
    id SERIAL PRIMARY KEY,
    feedback_id BIGINT NOT NULL,
    agent_id BIGINT NOT NULL,
    chain_id INT NOT NULL,
    response_uri TEXT,
    block_number BIGINT NOT NULL,
    tx_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);
CREATE INDEX idx_feedback_responses_feedback ON feedback_responses(feedback_id);
