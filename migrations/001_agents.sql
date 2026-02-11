CREATE TABLE IF NOT EXISTS agents (
    id SERIAL PRIMARY KEY,
    agent_id BIGINT NOT NULL,
    chain_id INT NOT NULL,
    owner TEXT NOT NULL,
    uri TEXT,
    metadata JSONB DEFAULT '{}',
    name TEXT,
    description TEXT,
    image TEXT,
    categories TEXT[] DEFAULT '{}',
    x402_support BOOLEAN DEFAULT false,
    active BOOLEAN DEFAULT true,
    block_number BIGINT,
    tx_hash TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(agent_id, chain_id)
);
CREATE INDEX idx_agents_chain ON agents(chain_id);
CREATE INDEX idx_agents_active ON agents(active);
CREATE INDEX idx_agents_name ON agents(name);
