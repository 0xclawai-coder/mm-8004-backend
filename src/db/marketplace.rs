use bigdecimal::BigDecimal;
use sqlx::PgPool;

use crate::types::{
    MarketplaceAuction, MarketplaceAuctionBid, MarketplaceBundle, MarketplaceCollectionOffer,
    MarketplaceDutchAuction, MarketplaceListing, MarketplaceOffer, MarketplaceStatsResponse,
    MarketplaceUserPortfolioResponse, NewMarketplaceAuction, NewMarketplaceBundle,
    NewMarketplaceCollectionOffer, NewMarketplaceDutchAuction, NewMarketplaceListing,
    NewMarketplaceOffer,
};

// ─── Listings ───────────────────────────────────────────────────────────

pub async fn upsert_listing(pool: &PgPool, l: &NewMarketplaceListing) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO marketplace_listings
            (listing_id, chain_id, seller, nft_contract, token_id, payment_token, price, expiry, block_number, block_timestamp, tx_hash)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        ON CONFLICT (listing_id, chain_id) DO UPDATE SET
            seller = EXCLUDED.seller,
            nft_contract = EXCLUDED.nft_contract,
            token_id = EXCLUDED.token_id,
            payment_token = EXCLUDED.payment_token,
            price = EXCLUDED.price,
            expiry = EXCLUDED.expiry,
            block_number = EXCLUDED.block_number,
            block_timestamp = EXCLUDED.block_timestamp,
            tx_hash = EXCLUDED.tx_hash,
            updated_at = NOW()
        "#,
    )
    .bind(l.listing_id)
    .bind(l.chain_id)
    .bind(&l.seller)
    .bind(&l.nft_contract)
    .bind(&l.token_id)
    .bind(&l.payment_token)
    .bind(&l.price)
    .bind(l.expiry)
    .bind(l.block_number)
    .bind(l.block_timestamp)
    .bind(&l.tx_hash)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_listing_status(
    pool: &PgPool,
    listing_id: i64,
    chain_id: i32,
    status: &str,
    buyer: Option<&str>,
    sold_price: Option<&BigDecimal>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE marketplace_listings
        SET status = $3, buyer = COALESCE($4, buyer), sold_price = COALESCE($5, sold_price), updated_at = NOW()
        WHERE listing_id = $1 AND chain_id = $2
        "#,
    )
    .bind(listing_id)
    .bind(chain_id)
    .bind(status)
    .bind(buyer)
    .bind(sold_price)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_listing_price(
    pool: &PgPool,
    listing_id: i64,
    chain_id: i32,
    new_price: &BigDecimal,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE marketplace_listings
        SET price = $3, updated_at = NOW()
        WHERE listing_id = $1 AND chain_id = $2
        "#,
    )
    .bind(listing_id)
    .bind(chain_id)
    .bind(new_price)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_listings(
    pool: &PgPool,
    chain_id: Option<i32>,
    nft_contract: Option<&str>,
    seller: Option<&str>,
    status: &str,
    sort: &str,
    offset: i64,
    limit: i64,
) -> Result<(Vec<MarketplaceListing>, i64), sqlx::Error> {
    let order_clause = match sort {
        "price_asc" => "l.price ASC",
        "price_desc" => "l.price DESC",
        _ => "l.block_number DESC",
    };

    let query = format!(
        r#"
        SELECT l.*, a.name AS agent_name, a.image AS agent_image
        FROM marketplace_listings l
        LEFT JOIN agents a ON a.agent_id = l.token_id::BIGINT AND a.chain_id = l.chain_id
        WHERE l.status = $1
          AND ($2::INT IS NULL OR l.chain_id = $2)
          AND ($3::TEXT IS NULL OR l.nft_contract = $3)
          AND ($4::TEXT IS NULL OR l.seller = $4)
        ORDER BY {}
        LIMIT $5 OFFSET $6
        "#,
        order_clause
    );

    let listings: Vec<MarketplaceListing> = sqlx::query_as(&query)
        .bind(status)
        .bind(chain_id)
        .bind(nft_contract)
        .bind(seller)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

    let (total,): (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM marketplace_listings
        WHERE status = $1
          AND ($2::INT IS NULL OR chain_id = $2)
          AND ($3::TEXT IS NULL OR nft_contract = $3)
          AND ($4::TEXT IS NULL OR seller = $4)
        "#,
    )
    .bind(status)
    .bind(chain_id)
    .bind(nft_contract)
    .bind(seller)
    .fetch_one(pool)
    .await?;

    Ok((listings, total))
}

pub async fn get_listing_by_id(
    pool: &PgPool,
    listing_id: i64,
    chain_id: i32,
) -> Result<Option<MarketplaceListing>, sqlx::Error> {
    sqlx::query_as(
        "SELECT * FROM marketplace_listings WHERE listing_id = $1 AND chain_id = $2",
    )
    .bind(listing_id)
    .bind(chain_id)
    .fetch_optional(pool)
    .await
}

// ─── Offers ─────────────────────────────────────────────────────────────

pub async fn upsert_offer(pool: &PgPool, o: &NewMarketplaceOffer) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO marketplace_offers
            (offer_id, chain_id, offerer, nft_contract, token_id, payment_token, amount, expiry, block_number, block_timestamp, tx_hash)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        ON CONFLICT (offer_id, chain_id) DO UPDATE SET
            offerer = EXCLUDED.offerer,
            nft_contract = EXCLUDED.nft_contract,
            token_id = EXCLUDED.token_id,
            payment_token = EXCLUDED.payment_token,
            amount = EXCLUDED.amount,
            expiry = EXCLUDED.expiry,
            updated_at = NOW()
        "#,
    )
    .bind(o.offer_id)
    .bind(o.chain_id)
    .bind(&o.offerer)
    .bind(&o.nft_contract)
    .bind(&o.token_id)
    .bind(&o.payment_token)
    .bind(&o.amount)
    .bind(o.expiry)
    .bind(o.block_number)
    .bind(o.block_timestamp)
    .bind(&o.tx_hash)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_offer_status(
    pool: &PgPool,
    offer_id: i64,
    chain_id: i32,
    status: &str,
    accepted_by: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE marketplace_offers
        SET status = $3, accepted_by = COALESCE($4, accepted_by), updated_at = NOW()
        WHERE offer_id = $1 AND chain_id = $2
        "#,
    )
    .bind(offer_id)
    .bind(chain_id)
    .bind(status)
    .bind(accepted_by)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_offers(
    pool: &PgPool,
    chain_id: Option<i32>,
    nft_contract: Option<&str>,
    token_id: Option<&str>,
    offerer: Option<&str>,
    status: Option<&str>,
    offset: i64,
    limit: i64,
) -> Result<(Vec<MarketplaceOffer>, i64), sqlx::Error> {
    let token_id_bd: Option<BigDecimal> = token_id.and_then(|t| t.parse().ok());

    let offers: Vec<MarketplaceOffer> = sqlx::query_as(
        r#"
        SELECT * FROM marketplace_offers
        WHERE ($1::INT IS NULL OR chain_id = $1)
          AND ($2::TEXT IS NULL OR nft_contract = $2)
          AND ($3::NUMERIC IS NULL OR token_id = $3)
          AND ($4::TEXT IS NULL OR offerer = $4)
          AND ($5::TEXT IS NULL OR status = $5)
        ORDER BY block_number DESC
        LIMIT $6 OFFSET $7
        "#,
    )
    .bind(chain_id)
    .bind(nft_contract)
    .bind(&token_id_bd)
    .bind(offerer)
    .bind(status)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let (total,): (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM marketplace_offers
        WHERE ($1::INT IS NULL OR chain_id = $1)
          AND ($2::TEXT IS NULL OR nft_contract = $2)
          AND ($3::NUMERIC IS NULL OR token_id = $3)
          AND ($4::TEXT IS NULL OR offerer = $4)
          AND ($5::TEXT IS NULL OR status = $5)
        "#,
    )
    .bind(chain_id)
    .bind(nft_contract)
    .bind(&token_id_bd)
    .bind(offerer)
    .bind(status)
    .fetch_one(pool)
    .await?;

    Ok((offers, total))
}

// ─── Collection Offers ──────────────────────────────────────────────────

pub async fn upsert_collection_offer(
    pool: &PgPool,
    o: &NewMarketplaceCollectionOffer,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO marketplace_collection_offers
            (offer_id, chain_id, offerer, nft_contract, payment_token, amount, expiry, block_number, block_timestamp, tx_hash)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        ON CONFLICT (offer_id, chain_id) DO UPDATE SET
            offerer = EXCLUDED.offerer,
            nft_contract = EXCLUDED.nft_contract,
            payment_token = EXCLUDED.payment_token,
            amount = EXCLUDED.amount,
            expiry = EXCLUDED.expiry,
            updated_at = NOW()
        "#,
    )
    .bind(o.offer_id)
    .bind(o.chain_id)
    .bind(&o.offerer)
    .bind(&o.nft_contract)
    .bind(&o.payment_token)
    .bind(&o.amount)
    .bind(o.expiry)
    .bind(o.block_number)
    .bind(o.block_timestamp)
    .bind(&o.tx_hash)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_collection_offer_status(
    pool: &PgPool,
    offer_id: i64,
    chain_id: i32,
    status: &str,
    accepted_by: Option<&str>,
    accepted_token_id: Option<&BigDecimal>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE marketplace_collection_offers
        SET status = $3,
            accepted_by = COALESCE($4, accepted_by),
            accepted_token_id = COALESCE($5, accepted_token_id),
            updated_at = NOW()
        WHERE offer_id = $1 AND chain_id = $2
        "#,
    )
    .bind(offer_id)
    .bind(chain_id)
    .bind(status)
    .bind(accepted_by)
    .bind(accepted_token_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_collection_offers(
    pool: &PgPool,
    chain_id: Option<i32>,
    nft_contract: Option<&str>,
    offerer: Option<&str>,
    status: Option<&str>,
    offset: i64,
    limit: i64,
) -> Result<(Vec<MarketplaceCollectionOffer>, i64), sqlx::Error> {
    let offers: Vec<MarketplaceCollectionOffer> = sqlx::query_as(
        r#"
        SELECT * FROM marketplace_collection_offers
        WHERE ($1::INT IS NULL OR chain_id = $1)
          AND ($2::TEXT IS NULL OR nft_contract = $2)
          AND ($3::TEXT IS NULL OR offerer = $3)
          AND ($4::TEXT IS NULL OR status = $4)
        ORDER BY block_number DESC
        LIMIT $5 OFFSET $6
        "#,
    )
    .bind(chain_id)
    .bind(nft_contract)
    .bind(offerer)
    .bind(status)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let (total,): (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM marketplace_collection_offers
        WHERE ($1::INT IS NULL OR chain_id = $1)
          AND ($2::TEXT IS NULL OR nft_contract = $2)
          AND ($3::TEXT IS NULL OR offerer = $3)
          AND ($4::TEXT IS NULL OR status = $4)
        "#,
    )
    .bind(chain_id)
    .bind(nft_contract)
    .bind(offerer)
    .bind(status)
    .fetch_one(pool)
    .await?;

    Ok((offers, total))
}

// ─── Auctions ───────────────────────────────────────────────────────────

pub async fn upsert_auction(
    pool: &PgPool,
    a: &NewMarketplaceAuction,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO marketplace_auctions
            (auction_id, chain_id, seller, nft_contract, token_id, payment_token,
             start_price, reserve_price, buy_now_price, start_time, end_time,
             block_number, block_timestamp, tx_hash)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
        ON CONFLICT (auction_id, chain_id) DO UPDATE SET
            seller = EXCLUDED.seller,
            nft_contract = EXCLUDED.nft_contract,
            token_id = EXCLUDED.token_id,
            payment_token = EXCLUDED.payment_token,
            start_price = EXCLUDED.start_price,
            reserve_price = EXCLUDED.reserve_price,
            buy_now_price = EXCLUDED.buy_now_price,
            start_time = EXCLUDED.start_time,
            end_time = EXCLUDED.end_time,
            updated_at = NOW()
        "#,
    )
    .bind(a.auction_id)
    .bind(a.chain_id)
    .bind(&a.seller)
    .bind(&a.nft_contract)
    .bind(&a.token_id)
    .bind(&a.payment_token)
    .bind(&a.start_price)
    .bind(&a.reserve_price)
    .bind(&a.buy_now_price)
    .bind(a.start_time)
    .bind(a.end_time)
    .bind(a.block_number)
    .bind(a.block_timestamp)
    .bind(&a.tx_hash)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_auction_bid(
    pool: &PgPool,
    auction_id: i64,
    chain_id: i32,
    highest_bid: &BigDecimal,
    highest_bidder: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE marketplace_auctions
        SET highest_bid = $3, highest_bidder = $4, bid_count = COALESCE(bid_count, 0) + 1, updated_at = NOW()
        WHERE auction_id = $1 AND chain_id = $2
        "#,
    )
    .bind(auction_id)
    .bind(chain_id)
    .bind(highest_bid)
    .bind(highest_bidder)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_auction_end_time(
    pool: &PgPool,
    auction_id: i64,
    chain_id: i32,
    new_end_time: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE marketplace_auctions
        SET end_time = $3, updated_at = NOW()
        WHERE auction_id = $1 AND chain_id = $2
        "#,
    )
    .bind(auction_id)
    .bind(chain_id)
    .bind(new_end_time)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_auction_status(
    pool: &PgPool,
    auction_id: i64,
    chain_id: i32,
    status: &str,
    winner: Option<&str>,
    settled_price: Option<&BigDecimal>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE marketplace_auctions
        SET status = $3, winner = COALESCE($4, winner), settled_price = COALESCE($5, settled_price), updated_at = NOW()
        WHERE auction_id = $1 AND chain_id = $2
        "#,
    )
    .bind(auction_id)
    .bind(chain_id)
    .bind(status)
    .bind(winner)
    .bind(settled_price)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn insert_auction_bid(
    pool: &PgPool,
    auction_id: i64,
    chain_id: i32,
    bidder: &str,
    amount: &BigDecimal,
    block_number: i64,
    block_timestamp: Option<chrono::DateTime<chrono::Utc>>,
    tx_hash: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO marketplace_auction_bids
            (auction_id, chain_id, bidder, amount, block_number, block_timestamp, tx_hash)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(auction_id)
    .bind(chain_id)
    .bind(bidder)
    .bind(amount)
    .bind(block_number)
    .bind(block_timestamp)
    .bind(tx_hash)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_auctions(
    pool: &PgPool,
    chain_id: Option<i32>,
    nft_contract: Option<&str>,
    seller: Option<&str>,
    status: Option<&str>,
    sort: &str,
    offset: i64,
    limit: i64,
) -> Result<(Vec<MarketplaceAuction>, i64), sqlx::Error> {
    let order_clause = match sort {
        "ending_soon" => "CASE WHEN a.status = 'Active' THEN 0 ELSE 1 END ASC, a.end_time ASC",
        "highest_bid" => "a.highest_bid DESC NULLS LAST",
        _ => "a.block_number DESC",
    };

    let query = format!(
        r#"
        SELECT a.*, ag.name AS agent_name, ag.image AS agent_image
        FROM marketplace_auctions a
        LEFT JOIN agents ag ON ag.agent_id = a.token_id::BIGINT AND ag.chain_id = a.chain_id
        WHERE ($1::INT IS NULL OR a.chain_id = $1)
          AND ($2::TEXT IS NULL OR a.nft_contract = $2)
          AND ($3::TEXT IS NULL OR a.seller = $3)
          AND ($4::TEXT IS NULL OR a.status = $4)
        ORDER BY {}
        LIMIT $5 OFFSET $6
        "#,
        order_clause
    );

    let auctions: Vec<MarketplaceAuction> = sqlx::query_as(&query)
        .bind(chain_id)
        .bind(nft_contract)
        .bind(seller)
        .bind(status)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

    let (total,): (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM marketplace_auctions
        WHERE ($1::INT IS NULL OR chain_id = $1)
          AND ($2::TEXT IS NULL OR nft_contract = $2)
          AND ($3::TEXT IS NULL OR seller = $3)
          AND ($4::TEXT IS NULL OR status = $4)
        "#,
    )
    .bind(chain_id)
    .bind(nft_contract)
    .bind(seller)
    .bind(status)
    .fetch_one(pool)
    .await?;

    Ok((auctions, total))
}

pub async fn get_auction_with_bids(
    pool: &PgPool,
    auction_id: i64,
    chain_id: i32,
) -> Result<Option<(MarketplaceAuction, Vec<MarketplaceAuctionBid>)>, sqlx::Error> {
    let auction: Option<MarketplaceAuction> = sqlx::query_as(
        "SELECT * FROM marketplace_auctions WHERE auction_id = $1 AND chain_id = $2",
    )
    .bind(auction_id)
    .bind(chain_id)
    .fetch_optional(pool)
    .await?;

    match auction {
        Some(a) => {
            let bids: Vec<MarketplaceAuctionBid> = sqlx::query_as(
                "SELECT * FROM marketplace_auction_bids WHERE auction_id = $1 AND chain_id = $2 ORDER BY amount DESC",
            )
            .bind(auction_id)
            .bind(chain_id)
            .fetch_all(pool)
            .await?;
            Ok(Some((a, bids)))
        }
        None => Ok(None),
    }
}

// ─── Dutch Auctions ────────────────────────────────────────────────────

pub async fn upsert_dutch_auction(
    pool: &PgPool,
    a: &NewMarketplaceDutchAuction,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO marketplace_dutch_auctions
            (auction_id, chain_id, seller, nft_contract, token_id, payment_token,
             start_price, end_price, start_time, end_time, block_number, block_timestamp, tx_hash)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
        ON CONFLICT (auction_id, chain_id) DO UPDATE SET
            seller = EXCLUDED.seller,
            nft_contract = EXCLUDED.nft_contract,
            token_id = EXCLUDED.token_id,
            payment_token = EXCLUDED.payment_token,
            start_price = EXCLUDED.start_price,
            end_price = EXCLUDED.end_price,
            start_time = EXCLUDED.start_time,
            end_time = EXCLUDED.end_time,
            updated_at = NOW()
        "#,
    )
    .bind(a.auction_id)
    .bind(a.chain_id)
    .bind(&a.seller)
    .bind(&a.nft_contract)
    .bind(&a.token_id)
    .bind(&a.payment_token)
    .bind(&a.start_price)
    .bind(&a.end_price)
    .bind(a.start_time)
    .bind(a.end_time)
    .bind(a.block_number)
    .bind(a.block_timestamp)
    .bind(&a.tx_hash)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_dutch_auction_status(
    pool: &PgPool,
    auction_id: i64,
    chain_id: i32,
    status: &str,
    buyer: Option<&str>,
    sold_price: Option<&BigDecimal>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE marketplace_dutch_auctions
        SET status = $3, buyer = COALESCE($4, buyer), sold_price = COALESCE($5, sold_price), updated_at = NOW()
        WHERE auction_id = $1 AND chain_id = $2
        "#,
    )
    .bind(auction_id)
    .bind(chain_id)
    .bind(status)
    .bind(buyer)
    .bind(sold_price)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_dutch_auctions(
    pool: &PgPool,
    chain_id: Option<i32>,
    nft_contract: Option<&str>,
    status: Option<&str>,
    offset: i64,
    limit: i64,
) -> Result<(Vec<MarketplaceDutchAuction>, i64), sqlx::Error> {
    let auctions: Vec<MarketplaceDutchAuction> = sqlx::query_as(
        r#"
        SELECT * FROM marketplace_dutch_auctions
        WHERE ($1::INT IS NULL OR chain_id = $1)
          AND ($2::TEXT IS NULL OR nft_contract = $2)
          AND ($3::TEXT IS NULL OR status = $3)
        ORDER BY block_number DESC
        LIMIT $4 OFFSET $5
        "#,
    )
    .bind(chain_id)
    .bind(nft_contract)
    .bind(status)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let (total,): (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM marketplace_dutch_auctions
        WHERE ($1::INT IS NULL OR chain_id = $1)
          AND ($2::TEXT IS NULL OR nft_contract = $2)
          AND ($3::TEXT IS NULL OR status = $3)
        "#,
    )
    .bind(chain_id)
    .bind(nft_contract)
    .bind(status)
    .fetch_one(pool)
    .await?;

    Ok((auctions, total))
}

// ─── Bundles ────────────────────────────────────────────────────────────

pub async fn upsert_bundle(pool: &PgPool, b: &NewMarketplaceBundle) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO marketplace_bundles
            (bundle_id, chain_id, seller, nft_contracts, token_ids, payment_token, price, expiry, item_count, block_number, block_timestamp, tx_hash)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        ON CONFLICT (bundle_id, chain_id) DO UPDATE SET
            seller = EXCLUDED.seller,
            nft_contracts = EXCLUDED.nft_contracts,
            token_ids = EXCLUDED.token_ids,
            payment_token = EXCLUDED.payment_token,
            price = EXCLUDED.price,
            expiry = EXCLUDED.expiry,
            item_count = EXCLUDED.item_count,
            updated_at = NOW()
        "#,
    )
    .bind(b.bundle_id)
    .bind(b.chain_id)
    .bind(&b.seller)
    .bind(&b.nft_contracts)
    .bind(&b.token_ids)
    .bind(&b.payment_token)
    .bind(&b.price)
    .bind(b.expiry)
    .bind(b.item_count)
    .bind(b.block_number)
    .bind(b.block_timestamp)
    .bind(&b.tx_hash)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_bundle_status(
    pool: &PgPool,
    bundle_id: i64,
    chain_id: i32,
    status: &str,
    buyer: Option<&str>,
    sold_price: Option<&BigDecimal>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE marketplace_bundles
        SET status = $3, buyer = COALESCE($4, buyer), sold_price = COALESCE($5, sold_price), updated_at = NOW()
        WHERE bundle_id = $1 AND chain_id = $2
        "#,
    )
    .bind(bundle_id)
    .bind(chain_id)
    .bind(status)
    .bind(buyer)
    .bind(sold_price)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_bundles(
    pool: &PgPool,
    chain_id: Option<i32>,
    seller: Option<&str>,
    status: Option<&str>,
    offset: i64,
    limit: i64,
) -> Result<(Vec<MarketplaceBundle>, i64), sqlx::Error> {
    let bundles: Vec<MarketplaceBundle> = sqlx::query_as(
        r#"
        SELECT * FROM marketplace_bundles
        WHERE ($1::INT IS NULL OR chain_id = $1)
          AND ($2::TEXT IS NULL OR seller = $2)
          AND ($3::TEXT IS NULL OR status = $3)
        ORDER BY block_number DESC
        LIMIT $4 OFFSET $5
        "#,
    )
    .bind(chain_id)
    .bind(seller)
    .bind(status)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let (total,): (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM marketplace_bundles
        WHERE ($1::INT IS NULL OR chain_id = $1)
          AND ($2::TEXT IS NULL OR seller = $2)
          AND ($3::TEXT IS NULL OR status = $3)
        "#,
    )
    .bind(chain_id)
    .bind(seller)
    .bind(status)
    .fetch_one(pool)
    .await?;

    Ok((bundles, total))
}

// ─── Config ─────────────────────────────────────────────────────────────

pub async fn upsert_marketplace_config(
    pool: &PgPool,
    chain_id: i32,
    fee_bps: Option<i32>,
    fee_recipient: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO marketplace_config (chain_id, platform_fee_bps, fee_recipient, updated_at)
        VALUES ($1, $2, $3, NOW())
        ON CONFLICT (chain_id) DO UPDATE SET
            platform_fee_bps = COALESCE($2, marketplace_config.platform_fee_bps),
            fee_recipient = COALESCE($3, marketplace_config.fee_recipient),
            updated_at = NOW()
        "#,
    )
    .bind(chain_id)
    .bind(fee_bps)
    .bind(fee_recipient)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn upsert_payment_token(
    pool: &PgPool,
    chain_id: i32,
    token_address: &str,
    active: bool,
    block_number: Option<i64>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO marketplace_payment_tokens (chain_id, token_address, active, block_number, updated_at)
        VALUES ($1, $2, $3, $4, NOW())
        ON CONFLICT (chain_id, token_address) DO UPDATE SET
            active = $3,
            block_number = COALESCE($4, marketplace_payment_tokens.block_number),
            updated_at = NOW()
        "#,
    )
    .bind(chain_id)
    .bind(token_address)
    .bind(active)
    .bind(block_number)
    .execute(pool)
    .await?;
    Ok(())
}

// ─── User Portfolio ─────────────────────────────────────────────────────

pub async fn get_user_portfolio(
    pool: &PgPool,
    address: &str,
    chain_id: Option<i32>,
) -> Result<MarketplaceUserPortfolioResponse, sqlx::Error> {
    let listings: Vec<MarketplaceListing> = sqlx::query_as(
        r#"
        SELECT * FROM marketplace_listings
        WHERE seller = $1 AND ($2::INT IS NULL OR chain_id = $2)
        ORDER BY block_number DESC
        LIMIT 50
        "#,
    )
    .bind(address)
    .bind(chain_id)
    .fetch_all(pool)
    .await?;

    let offers: Vec<MarketplaceOffer> = sqlx::query_as(
        r#"
        SELECT * FROM marketplace_offers
        WHERE offerer = $1 AND ($2::INT IS NULL OR chain_id = $2)
        ORDER BY block_number DESC
        LIMIT 50
        "#,
    )
    .bind(address)
    .bind(chain_id)
    .fetch_all(pool)
    .await?;

    let bids: Vec<MarketplaceAuctionBid> = sqlx::query_as(
        r#"
        SELECT * FROM marketplace_auction_bids
        WHERE bidder = $1 AND ($2::INT IS NULL OR chain_id = $2)
        ORDER BY block_number DESC
        LIMIT 50
        "#,
    )
    .bind(address)
    .bind(chain_id)
    .fetch_all(pool)
    .await?;

    Ok(MarketplaceUserPortfolioResponse {
        listings,
        offers,
        bids,
    })
}

// ─── Marketplace Stats ──────────────────────────────────────────────────

pub async fn get_marketplace_stats(pool: &PgPool) -> Result<MarketplaceStatsResponse, sqlx::Error> {
    // Combined marketplace listing stats in a single query (was 4 separate queries)
    let (total_listings, active_listings, total_sales, total_volume): (i64, i64, i64, BigDecimal) =
        sqlx::query_as(
            r#"
            SELECT
                COUNT(*),
                COUNT(*) FILTER (WHERE status = 'Active'),
                COUNT(*) FILTER (WHERE status = 'Sold'),
                COALESCE(SUM(sold_price) FILTER (WHERE status = 'Sold'), 0)
            FROM marketplace_listings
            "#,
        )
        .fetch_one(pool)
        .await?;

    let (active_auctions,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM marketplace_auctions WHERE status = 'Active'")
            .fetch_one(pool)
            .await?;

    Ok(MarketplaceStatsResponse {
        total_listings,
        active_listings,
        total_sales,
        total_volume,
        active_auctions,
    })
}
