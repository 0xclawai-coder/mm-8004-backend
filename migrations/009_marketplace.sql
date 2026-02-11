-- MoltMarketplace tables: listings, offers, auctions, bundles, config

-- Fixed-price listings
CREATE TABLE marketplace_listings (
    id SERIAL PRIMARY KEY,
    listing_id BIGINT NOT NULL,
    chain_id INT NOT NULL,
    seller TEXT NOT NULL,
    nft_contract TEXT NOT NULL,
    token_id NUMERIC NOT NULL,
    payment_token TEXT NOT NULL,
    price NUMERIC NOT NULL,
    expiry BIGINT NOT NULL,
    status TEXT NOT NULL DEFAULT 'Active',
    buyer TEXT,
    sold_price NUMERIC,
    block_number BIGINT NOT NULL,
    block_timestamp TIMESTAMPTZ,
    tx_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(listing_id, chain_id)
);

CREATE INDEX idx_ml_chain ON marketplace_listings(chain_id);
CREATE INDEX idx_ml_seller ON marketplace_listings(seller);
CREATE INDEX idx_ml_nft ON marketplace_listings(nft_contract, token_id);
CREATE INDEX idx_ml_status ON marketplace_listings(status);
CREATE INDEX idx_ml_block ON marketplace_listings(block_number);

-- Offers (ERC-20 only, lazy escrow)
CREATE TABLE marketplace_offers (
    id SERIAL PRIMARY KEY,
    offer_id BIGINT NOT NULL,
    chain_id INT NOT NULL,
    offerer TEXT NOT NULL,
    nft_contract TEXT NOT NULL,
    token_id NUMERIC NOT NULL,
    payment_token TEXT NOT NULL,
    amount NUMERIC NOT NULL,
    expiry BIGINT NOT NULL,
    status TEXT NOT NULL DEFAULT 'Active',
    accepted_by TEXT,
    block_number BIGINT NOT NULL,
    block_timestamp TIMESTAMPTZ,
    tx_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(offer_id, chain_id)
);

CREATE INDEX idx_mo_chain ON marketplace_offers(chain_id);
CREATE INDEX idx_mo_nft ON marketplace_offers(nft_contract, token_id);
CREATE INDEX idx_mo_offerer ON marketplace_offers(offerer);
CREATE INDEX idx_mo_status ON marketplace_offers(status);

-- Collection offers (any tokenId in a collection)
CREATE TABLE marketplace_collection_offers (
    id SERIAL PRIMARY KEY,
    offer_id BIGINT NOT NULL,
    chain_id INT NOT NULL,
    offerer TEXT NOT NULL,
    nft_contract TEXT NOT NULL,
    payment_token TEXT NOT NULL,
    amount NUMERIC NOT NULL,
    expiry BIGINT NOT NULL,
    status TEXT NOT NULL DEFAULT 'Active',
    accepted_by TEXT,
    accepted_token_id NUMERIC,
    block_number BIGINT NOT NULL,
    block_timestamp TIMESTAMPTZ,
    tx_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(offer_id, chain_id)
);

CREATE INDEX idx_mco_chain ON marketplace_collection_offers(chain_id);
CREATE INDEX idx_mco_nft ON marketplace_collection_offers(nft_contract);
CREATE INDEX idx_mco_offerer ON marketplace_collection_offers(offerer);
CREATE INDEX idx_mco_status ON marketplace_collection_offers(status);

-- English auctions
CREATE TABLE marketplace_auctions (
    id SERIAL PRIMARY KEY,
    auction_id BIGINT NOT NULL,
    chain_id INT NOT NULL,
    seller TEXT NOT NULL,
    nft_contract TEXT NOT NULL,
    token_id NUMERIC NOT NULL,
    payment_token TEXT NOT NULL,
    start_price NUMERIC NOT NULL,
    reserve_price NUMERIC NOT NULL,
    buy_now_price NUMERIC NOT NULL,
    highest_bid NUMERIC DEFAULT 0,
    highest_bidder TEXT,
    start_time BIGINT NOT NULL,
    end_time BIGINT NOT NULL,
    bid_count INT DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'Active',
    winner TEXT,
    settled_price NUMERIC,
    block_number BIGINT NOT NULL,
    block_timestamp TIMESTAMPTZ,
    tx_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(auction_id, chain_id)
);

CREATE INDEX idx_ma_chain ON marketplace_auctions(chain_id);
CREATE INDEX idx_ma_seller ON marketplace_auctions(seller);
CREATE INDEX idx_ma_nft ON marketplace_auctions(nft_contract, token_id);
CREATE INDEX idx_ma_status ON marketplace_auctions(status);

-- Bid history for English auctions
CREATE TABLE marketplace_auction_bids (
    id SERIAL PRIMARY KEY,
    auction_id BIGINT NOT NULL,
    chain_id INT NOT NULL,
    bidder TEXT NOT NULL,
    amount NUMERIC NOT NULL,
    block_number BIGINT NOT NULL,
    block_timestamp TIMESTAMPTZ,
    tx_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_mab_auction ON marketplace_auction_bids(auction_id, chain_id);

-- Dutch auctions (linear price decay)
CREATE TABLE marketplace_dutch_auctions (
    id SERIAL PRIMARY KEY,
    auction_id BIGINT NOT NULL,
    chain_id INT NOT NULL,
    seller TEXT NOT NULL,
    nft_contract TEXT NOT NULL,
    token_id NUMERIC NOT NULL,
    payment_token TEXT NOT NULL,
    start_price NUMERIC NOT NULL,
    end_price NUMERIC NOT NULL,
    start_time BIGINT NOT NULL,
    end_time BIGINT NOT NULL,
    status TEXT NOT NULL DEFAULT 'Active',
    buyer TEXT,
    sold_price NUMERIC,
    block_number BIGINT NOT NULL,
    block_timestamp TIMESTAMPTZ,
    tx_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(auction_id, chain_id)
);

CREATE INDEX idx_mda_chain ON marketplace_dutch_auctions(chain_id);
CREATE INDEX idx_mda_nft ON marketplace_dutch_auctions(nft_contract, token_id);
CREATE INDEX idx_mda_status ON marketplace_dutch_auctions(status);

-- Bundle listings (up to 20 NFTs)
CREATE TABLE marketplace_bundles (
    id SERIAL PRIMARY KEY,
    bundle_id BIGINT NOT NULL,
    chain_id INT NOT NULL,
    seller TEXT NOT NULL,
    nft_contracts TEXT[] NOT NULL,
    token_ids NUMERIC[] NOT NULL,
    payment_token TEXT NOT NULL,
    price NUMERIC NOT NULL,
    expiry BIGINT NOT NULL,
    item_count INT NOT NULL,
    status TEXT NOT NULL DEFAULT 'Active',
    buyer TEXT,
    sold_price NUMERIC,
    block_number BIGINT NOT NULL,
    block_timestamp TIMESTAMPTZ,
    tx_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(bundle_id, chain_id)
);

CREATE INDEX idx_mb_chain ON marketplace_bundles(chain_id);
CREATE INDEX idx_mb_seller ON marketplace_bundles(seller);
CREATE INDEX idx_mb_status ON marketplace_bundles(status);

-- Platform config (fee, recipient)
CREATE TABLE marketplace_config (
    id SERIAL PRIMARY KEY,
    chain_id INT NOT NULL,
    platform_fee_bps INT,
    fee_recipient TEXT,
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(chain_id)
);

-- Allowed payment tokens
CREATE TABLE marketplace_payment_tokens (
    id SERIAL PRIMARY KEY,
    chain_id INT NOT NULL,
    token_address TEXT NOT NULL,
    active BOOLEAN NOT NULL DEFAULT true,
    block_number BIGINT,
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(chain_id, token_address)
);
