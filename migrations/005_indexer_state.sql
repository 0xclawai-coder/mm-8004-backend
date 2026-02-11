CREATE TABLE IF NOT EXISTS indexer_state (
    chain_id INT NOT NULL,
    contract_address TEXT NOT NULL,
    last_block BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY(chain_id, contract_address)
);
