-- Add missing indexes for common query patterns
CREATE INDEX IF NOT EXISTS idx_agents_owner ON agents(owner);
CREATE INDEX IF NOT EXISTS idx_agents_chain_id_active ON agents(chain_id, active);
CREATE INDEX IF NOT EXISTS idx_feedbacks_created_at ON feedbacks(created_at);
CREATE INDEX IF NOT EXISTS idx_feedbacks_agent_chain_revoked ON feedbacks(agent_id, chain_id, revoked);
CREATE INDEX IF NOT EXISTS idx_marketplace_listings_status ON marketplace_listings(status);
CREATE INDEX IF NOT EXISTS idx_marketplace_listings_seller ON marketplace_listings(seller);
CREATE INDEX IF NOT EXISTS idx_marketplace_listings_nft_token ON marketplace_listings(nft_contract, token_id);
CREATE INDEX IF NOT EXISTS idx_marketplace_auctions_status ON marketplace_auctions(status);
CREATE INDEX IF NOT EXISTS idx_activity_log_chain_created ON activity_log(chain_id, created_at DESC);
