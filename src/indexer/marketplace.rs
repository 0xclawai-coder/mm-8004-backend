use std::collections::HashMap;
use std::str::FromStr;

use alloy::providers::Provider;
use alloy::rpc::types::Filter;
use alloy::sol;
use alloy::sol_types::SolEvent;
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::provider::{self, ChainConfig, HttpProvider};
use crate::db;
use crate::types::{
    NewActivity, NewMarketplaceAuction, NewMarketplaceBundle, NewMarketplaceCollectionOffer,
    NewMarketplaceDutchAuction, NewMarketplaceListing, NewMarketplaceOffer,
};

// Load MoltMarketplace ABI
sol!(MoltMarketplace, "abi/MoltMarketplace.json");

use MoltMarketplace::{
    AuctionBuyNow, AuctionCancelled, AuctionCreated, AuctionExtended, AuctionReserveNotMet,
    AuctionSettled, BidPlaced, Bought, BundleBought, BundleListed, BundleListingCancelled,
    CollectionOfferAccepted, CollectionOfferCancelled, CollectionOfferMade, DutchAuctionBought,
    DutchAuctionCancelled, DutchAuctionCreated, FeeRecipientUpdated, Listed, ListingCancelled,
    ListingPriceUpdated, OfferAccepted, OfferCancelled, OfferMade, PaymentTokenAdded,
    PaymentTokenRemoved, PlatformFeeUpdated,
};

/// Index marketplace events for a block range.
pub async fn index_marketplace_events(
    pool: &PgPool,
    provider: &HttpProvider,
    chain: &ChainConfig,
    from_block: u64,
    to_block: u64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let marketplace_address = match chain.marketplace_address {
        Some(addr) => addr,
        None => return Ok(()),
    };

    let filter = Filter::new()
        .address(marketplace_address)
        .from_block(from_block)
        .to_block(to_block)
        .event_signature(vec![
            Listed::SIGNATURE_HASH,
            Bought::SIGNATURE_HASH,
            ListingCancelled::SIGNATURE_HASH,
            ListingPriceUpdated::SIGNATURE_HASH,
            OfferMade::SIGNATURE_HASH,
            OfferAccepted::SIGNATURE_HASH,
            OfferCancelled::SIGNATURE_HASH,
            CollectionOfferMade::SIGNATURE_HASH,
            CollectionOfferAccepted::SIGNATURE_HASH,
            CollectionOfferCancelled::SIGNATURE_HASH,
            AuctionCreated::SIGNATURE_HASH,
            BidPlaced::SIGNATURE_HASH,
            AuctionSettled::SIGNATURE_HASH,
            AuctionCancelled::SIGNATURE_HASH,
            AuctionExtended::SIGNATURE_HASH,
            AuctionBuyNow::SIGNATURE_HASH,
            AuctionReserveNotMet::SIGNATURE_HASH,
            DutchAuctionCreated::SIGNATURE_HASH,
            DutchAuctionBought::SIGNATURE_HASH,
            DutchAuctionCancelled::SIGNATURE_HASH,
            BundleListed::SIGNATURE_HASH,
            BundleBought::SIGNATURE_HASH,
            BundleListingCancelled::SIGNATURE_HASH,
            PlatformFeeUpdated::SIGNATURE_HASH,
            FeeRecipientUpdated::SIGNATURE_HASH,
            PaymentTokenAdded::SIGNATURE_HASH,
            PaymentTokenRemoved::SIGNATURE_HASH,
        ]);

    let logs = provider.get_logs(&filter).await?;

    tracing::debug!(
        chain_id = chain.chain_id,
        "Fetched {} marketplace logs for blocks {} - {}",
        logs.len(),
        from_block,
        to_block
    );

    let mut block_ts_cache: HashMap<u64, DateTime<Utc>> = HashMap::new();

    for log in logs {
        let block_num_raw = log.block_number.unwrap_or(0);
        let block_number = block_num_raw as i64;
        let tx_hash = log
            .transaction_hash
            .map(|h| format!("{:#x}", h))
            .unwrap_or_default();
        let log_index = log.log_index.unwrap_or(0) as i32;

        let block_timestamp = if let Some(ts) = block_ts_cache.get(&block_num_raw) {
            Some(*ts)
        } else {
            match provider::get_block_timestamp(provider, block_num_raw).await {
                Ok(ts) => {
                    block_ts_cache.insert(block_num_raw, ts);
                    Some(ts)
                }
                Err(e) => {
                    tracing::warn!("Failed to fetch timestamp for block {}: {:?}", block_num_raw, e);
                    None
                }
            }
        };

        let topic0 = match log.topic0() {
            Some(t) => *t,
            None => continue,
        };

        // ─── Listings ───────────────────────────────────────────────
        if topic0 == Listed::SIGNATURE_HASH {
            if let Ok(decoded) = log.log_decode::<Listed>() {
                let e = &decoded.inner.data;
                let listing_id = e.listingId.to::<u64>() as i64;
                let seller = format!("{:#x}", e.seller);
                let nft_contract = format!("{:#x}", e.nftContract);
                let token_id = BigDecimal::from_str(&e.tokenId.to_string()).unwrap_or_default();
                let payment_token = format!("{:#x}", e.paymentToken);
                let price = BigDecimal::from_str(&e.price.to_string()).unwrap_or_default();
                let expiry = e.expiry.to::<u64>() as i64;

                tracing::info!(chain_id = chain.chain_id, "Listed #{}", listing_id);

                let new_listing = NewMarketplaceListing {
                    listing_id,
                    chain_id: chain.chain_id,
                    seller: seller.clone(),
                    nft_contract: nft_contract.clone(),
                    token_id: token_id.clone(),
                    payment_token: payment_token.clone(),
                    price: price.clone(),
                    expiry,
                    block_number,
                    block_timestamp,
                    tx_hash: tx_hash.clone(),
                };
                if let Err(err) = db::marketplace::upsert_listing(pool, &new_listing).await {
                    tracing::error!("Failed to upsert listing {}: {:?}", listing_id, err);
                }

                maybe_insert_agent_activity(
                    pool, chain, &nft_contract, &token_id, "marketplace:Listed",
                    serde_json::json!({"listing_id": listing_id, "seller": seller, "price": price.to_string(), "payment_token": payment_token}),
                    block_number, block_timestamp, &tx_hash, log_index,
                ).await;
            }
        } else if topic0 == Bought::SIGNATURE_HASH {
            if let Ok(decoded) = log.log_decode::<Bought>() {
                let e = &decoded.inner.data;
                let listing_id = e.listingId.to::<u64>() as i64;
                let buyer = format!("{:#x}", e.buyer);
                let price = BigDecimal::from_str(&e.price.to_string()).unwrap_or_default();

                tracing::info!(chain_id = chain.chain_id, "Bought #{}", listing_id);

                if let Err(err) = db::marketplace::update_listing_status(
                    pool, listing_id, chain.chain_id, "Sold", Some(&buyer), Some(&price),
                ).await {
                    tracing::error!("Failed to update listing {} as Sold: {:?}", listing_id, err);
                }

                // Cross-reference: look up NFT info from listing for activity
                if let Ok(Some(listing)) = db::marketplace::get_listing_by_id(pool, listing_id, chain.chain_id).await {
                    maybe_insert_agent_activity(
                        pool, chain, &listing.nft_contract, &listing.token_id, "marketplace:Bought",
                        serde_json::json!({"listing_id": listing_id, "buyer": buyer, "price": price.to_string()}),
                        block_number, block_timestamp, &tx_hash, log_index,
                    ).await;
                }
            }
        } else if topic0 == ListingCancelled::SIGNATURE_HASH {
            if let Ok(decoded) = log.log_decode::<ListingCancelled>() {
                let listing_id = decoded.inner.data.listingId.to::<u64>() as i64;
                tracing::info!(chain_id = chain.chain_id, "ListingCancelled #{}", listing_id);
                if let Err(err) = db::marketplace::update_listing_status(
                    pool, listing_id, chain.chain_id, "Cancelled", None, None,
                ).await {
                    tracing::error!("Failed to cancel listing {}: {:?}", listing_id, err);
                }
            }
        } else if topic0 == ListingPriceUpdated::SIGNATURE_HASH {
            if let Ok(decoded) = log.log_decode::<ListingPriceUpdated>() {
                let e = &decoded.inner.data;
                let listing_id = e.listingId.to::<u64>() as i64;
                let new_price = BigDecimal::from_str(&e.newPrice.to_string()).unwrap_or_default();
                tracing::info!(chain_id = chain.chain_id, "ListingPriceUpdated #{}", listing_id);
                if let Err(err) = db::marketplace::update_listing_price(
                    pool, listing_id, chain.chain_id, &new_price,
                ).await {
                    tracing::error!("Failed to update listing {} price: {:?}", listing_id, err);
                }
            }
        }
        // ─── Offers ─────────────────────────────────────────────────
        else if topic0 == OfferMade::SIGNATURE_HASH {
            if let Ok(decoded) = log.log_decode::<OfferMade>() {
                let e = &decoded.inner.data;
                let offer_id = e.offerId.to::<u64>() as i64;
                let offerer = format!("{:#x}", e.offerer);
                let nft_contract = format!("{:#x}", e.nftContract);
                let token_id = BigDecimal::from_str(&e.tokenId.to_string()).unwrap_or_default();
                let payment_token = format!("{:#x}", e.paymentToken);
                let amount = BigDecimal::from_str(&e.amount.to_string()).unwrap_or_default();
                let expiry = e.expiry.to::<u64>() as i64;

                tracing::info!(chain_id = chain.chain_id, "OfferMade #{}", offer_id);

                let new_offer = NewMarketplaceOffer {
                    offer_id,
                    chain_id: chain.chain_id,
                    offerer: offerer.clone(),
                    nft_contract: nft_contract.clone(),
                    token_id: token_id.clone(),
                    payment_token,
                    amount,
                    expiry,
                    block_number,
                    block_timestamp,
                    tx_hash: tx_hash.clone(),
                };
                if let Err(err) = db::marketplace::upsert_offer(pool, &new_offer).await {
                    tracing::error!("Failed to upsert offer {}: {:?}", offer_id, err);
                }

                maybe_insert_agent_activity(
                    pool, chain, &nft_contract, &token_id, "marketplace:OfferMade",
                    serde_json::json!({"offer_id": offer_id, "offerer": offerer}),
                    block_number, block_timestamp, &tx_hash, log_index,
                ).await;
            }
        } else if topic0 == OfferAccepted::SIGNATURE_HASH {
            if let Ok(decoded) = log.log_decode::<OfferAccepted>() {
                let e = &decoded.inner.data;
                let offer_id = e.offerId.to::<u64>() as i64;
                let seller = format!("{:#x}", e.seller);
                tracing::info!(chain_id = chain.chain_id, "OfferAccepted #{}", offer_id);
                if let Err(err) = db::marketplace::update_offer_status(
                    pool, offer_id, chain.chain_id, "Accepted", Some(&seller),
                ).await {
                    tracing::error!("Failed to accept offer {}: {:?}", offer_id, err);
                }
            }
        } else if topic0 == OfferCancelled::SIGNATURE_HASH {
            if let Ok(decoded) = log.log_decode::<OfferCancelled>() {
                let offer_id = decoded.inner.data.offerId.to::<u64>() as i64;
                tracing::info!(chain_id = chain.chain_id, "OfferCancelled #{}", offer_id);
                if let Err(err) = db::marketplace::update_offer_status(
                    pool, offer_id, chain.chain_id, "Cancelled", None,
                ).await {
                    tracing::error!("Failed to cancel offer {}: {:?}", offer_id, err);
                }
            }
        }
        // ─── Collection Offers ──────────────────────────────────────
        else if topic0 == CollectionOfferMade::SIGNATURE_HASH {
            if let Ok(decoded) = log.log_decode::<CollectionOfferMade>() {
                let e = &decoded.inner.data;
                let offer_id = e.offerId.to::<u64>() as i64;
                let offerer = format!("{:#x}", e.offerer);
                let nft_contract = format!("{:#x}", e.nftContract);
                let payment_token = format!("{:#x}", e.paymentToken);
                let amount = BigDecimal::from_str(&e.amount.to_string()).unwrap_or_default();
                let expiry = e.expiry.to::<u64>() as i64;

                tracing::info!(chain_id = chain.chain_id, "CollectionOfferMade #{}", offer_id);

                let new_offer = NewMarketplaceCollectionOffer {
                    offer_id,
                    chain_id: chain.chain_id,
                    offerer,
                    nft_contract,
                    payment_token,
                    amount,
                    expiry,
                    block_number,
                    block_timestamp,
                    tx_hash: tx_hash.clone(),
                };
                if let Err(err) = db::marketplace::upsert_collection_offer(pool, &new_offer).await {
                    tracing::error!("Failed to upsert collection offer {}: {:?}", offer_id, err);
                }
            }
        } else if topic0 == CollectionOfferAccepted::SIGNATURE_HASH {
            if let Ok(decoded) = log.log_decode::<CollectionOfferAccepted>() {
                let e = &decoded.inner.data;
                let offer_id = e.offerId.to::<u64>() as i64;
                let seller = format!("{:#x}", e.seller);
                let token_id = BigDecimal::from_str(&e.tokenId.to_string()).unwrap_or_default();
                tracing::info!(chain_id = chain.chain_id, "CollectionOfferAccepted #{}", offer_id);
                if let Err(err) = db::marketplace::update_collection_offer_status(
                    pool, offer_id, chain.chain_id, "Accepted", Some(&seller), Some(&token_id),
                ).await {
                    tracing::error!("Failed to accept collection offer {}: {:?}", offer_id, err);
                }
            }
        } else if topic0 == CollectionOfferCancelled::SIGNATURE_HASH {
            if let Ok(decoded) = log.log_decode::<CollectionOfferCancelled>() {
                let offer_id = decoded.inner.data.offerId.to::<u64>() as i64;
                tracing::info!(chain_id = chain.chain_id, "CollectionOfferCancelled #{}", offer_id);
                if let Err(err) = db::marketplace::update_collection_offer_status(
                    pool, offer_id, chain.chain_id, "Cancelled", None, None,
                ).await {
                    tracing::error!("Failed to cancel collection offer {}: {:?}", offer_id, err);
                }
            }
        }
        // ─── English Auctions ───────────────────────────────────────
        else if topic0 == AuctionCreated::SIGNATURE_HASH {
            if let Ok(decoded) = log.log_decode::<AuctionCreated>() {
                let e = &decoded.inner.data;
                let auction_id = e.auctionId.to::<u64>() as i64;
                let seller = format!("{:#x}", e.seller);
                let nft_contract = format!("{:#x}", e.nftContract);
                let token_id = BigDecimal::from_str(&e.tokenId.to_string()).unwrap_or_default();
                let payment_token = format!("{:#x}", e.paymentToken);
                let start_price = BigDecimal::from_str(&e.startPrice.to_string()).unwrap_or_default();
                let reserve_price = BigDecimal::from_str(&e.reservePrice.to_string()).unwrap_or_default();
                let buy_now_price = BigDecimal::from_str(&e.buyNowPrice.to_string()).unwrap_or_default();
                let start_time = e.startTime.to::<u64>() as i64;
                let end_time = e.endTime.to::<u64>() as i64;

                tracing::info!(chain_id = chain.chain_id, "AuctionCreated #{}", auction_id);

                let new_auction = NewMarketplaceAuction {
                    auction_id,
                    chain_id: chain.chain_id,
                    seller: seller.clone(),
                    nft_contract: nft_contract.clone(),
                    token_id: token_id.clone(),
                    payment_token,
                    start_price,
                    reserve_price,
                    buy_now_price,
                    start_time,
                    end_time,
                    block_number,
                    block_timestamp,
                    tx_hash: tx_hash.clone(),
                };
                if let Err(err) = db::marketplace::upsert_auction(pool, &new_auction).await {
                    tracing::error!("Failed to upsert auction {}: {:?}", auction_id, err);
                }

                maybe_insert_agent_activity(
                    pool, chain, &nft_contract, &token_id, "marketplace:AuctionCreated",
                    serde_json::json!({"auction_id": auction_id, "seller": seller}),
                    block_number, block_timestamp, &tx_hash, log_index,
                ).await;
            }
        } else if topic0 == BidPlaced::SIGNATURE_HASH {
            if let Ok(decoded) = log.log_decode::<BidPlaced>() {
                let e = &decoded.inner.data;
                let auction_id = e.auctionId.to::<u64>() as i64;
                let bidder = format!("{:#x}", e.bidder);
                let amount = BigDecimal::from_str(&e.amount.to_string()).unwrap_or_default();

                tracing::info!(chain_id = chain.chain_id, "BidPlaced #{} by {}", auction_id, bidder);

                if let Err(err) = db::marketplace::update_auction_bid(
                    pool, auction_id, chain.chain_id, &amount, &bidder,
                ).await {
                    tracing::error!("Failed to update auction {} bid: {:?}", auction_id, err);
                }

                if let Err(err) = db::marketplace::insert_auction_bid(
                    pool, auction_id, chain.chain_id, &bidder, &amount,
                    block_number, block_timestamp, &tx_hash,
                ).await {
                    tracing::error!("Failed to insert auction bid: {:?}", err);
                }
            }
        } else if topic0 == AuctionSettled::SIGNATURE_HASH {
            if let Ok(decoded) = log.log_decode::<AuctionSettled>() {
                let e = &decoded.inner.data;
                let auction_id = e.auctionId.to::<u64>() as i64;
                let winner = format!("{:#x}", e.winner);
                let amount = BigDecimal::from_str(&e.amount.to_string()).unwrap_or_default();
                tracing::info!(chain_id = chain.chain_id, "AuctionSettled #{}", auction_id);
                if let Err(err) = db::marketplace::update_auction_status(
                    pool, auction_id, chain.chain_id, "Ended", Some(&winner), Some(&amount),
                ).await {
                    tracing::error!("Failed to settle auction {}: {:?}", auction_id, err);
                }
            }
        } else if topic0 == AuctionCancelled::SIGNATURE_HASH {
            if let Ok(decoded) = log.log_decode::<AuctionCancelled>() {
                let auction_id = decoded.inner.data.auctionId.to::<u64>() as i64;
                tracing::info!(chain_id = chain.chain_id, "AuctionCancelled #{}", auction_id);
                if let Err(err) = db::marketplace::update_auction_status(
                    pool, auction_id, chain.chain_id, "Cancelled", None, None,
                ).await {
                    tracing::error!("Failed to cancel auction {}: {:?}", auction_id, err);
                }
            }
        } else if topic0 == AuctionExtended::SIGNATURE_HASH {
            if let Ok(decoded) = log.log_decode::<AuctionExtended>() {
                let e = &decoded.inner.data;
                let auction_id = e.auctionId.to::<u64>() as i64;
                let new_end_time = e.newEndTime.to::<u64>() as i64;
                tracing::info!(chain_id = chain.chain_id, "AuctionExtended #{}", auction_id);
                if let Err(err) = db::marketplace::update_auction_end_time(
                    pool, auction_id, chain.chain_id, new_end_time,
                ).await {
                    tracing::error!("Failed to extend auction {}: {:?}", auction_id, err);
                }
            }
        } else if topic0 == AuctionBuyNow::SIGNATURE_HASH {
            if let Ok(decoded) = log.log_decode::<AuctionBuyNow>() {
                let e = &decoded.inner.data;
                let auction_id = e.auctionId.to::<u64>() as i64;
                let buyer = format!("{:#x}", e.buyer);
                let price = BigDecimal::from_str(&e.price.to_string()).unwrap_or_default();
                tracing::info!(chain_id = chain.chain_id, "AuctionBuyNow #{}", auction_id);
                if let Err(err) = db::marketplace::update_auction_status(
                    pool, auction_id, chain.chain_id, "Ended", Some(&buyer), Some(&price),
                ).await {
                    tracing::error!("Failed to buy-now auction {}: {:?}", auction_id, err);
                }
            }
        } else if topic0 == AuctionReserveNotMet::SIGNATURE_HASH {
            if let Ok(decoded) = log.log_decode::<AuctionReserveNotMet>() {
                let auction_id = decoded.inner.data.auctionId.to::<u64>() as i64;
                tracing::info!(chain_id = chain.chain_id, "AuctionReserveNotMet #{}", auction_id);
                if let Err(err) = db::marketplace::update_auction_status(
                    pool, auction_id, chain.chain_id, "ReserveNotMet", None, None,
                ).await {
                    tracing::error!("Failed to mark auction {} reserve not met: {:?}", auction_id, err);
                }
            }
        }
        // ─── Dutch Auctions ────────────────────────────────────────
        else if topic0 == DutchAuctionCreated::SIGNATURE_HASH {
            if let Ok(decoded) = log.log_decode::<DutchAuctionCreated>() {
                let e = &decoded.inner.data;
                let auction_id = e.auctionId.to::<u64>() as i64;
                let seller = format!("{:#x}", e.seller);
                let nft_contract = format!("{:#x}", e.nftContract);
                let token_id = BigDecimal::from_str(&e.tokenId.to_string()).unwrap_or_default();
                let payment_token = format!("{:#x}", e.paymentToken);
                let start_price = BigDecimal::from_str(&e.startPrice.to_string()).unwrap_or_default();
                let end_price = BigDecimal::from_str(&e.endPrice.to_string()).unwrap_or_default();
                let start_time = e.startTime.to::<u64>() as i64;
                let end_time = e.endTime.to::<u64>() as i64;

                tracing::info!(chain_id = chain.chain_id, "DutchAuctionCreated #{}", auction_id);

                let new_auction = NewMarketplaceDutchAuction {
                    auction_id,
                    chain_id: chain.chain_id,
                    seller: seller.clone(),
                    nft_contract: nft_contract.clone(),
                    token_id: token_id.clone(),
                    payment_token,
                    start_price,
                    end_price,
                    start_time,
                    end_time,
                    block_number,
                    block_timestamp,
                    tx_hash: tx_hash.clone(),
                };
                if let Err(err) = db::marketplace::upsert_dutch_auction(pool, &new_auction).await {
                    tracing::error!("Failed to upsert dutch auction {}: {:?}", auction_id, err);
                }

                maybe_insert_agent_activity(
                    pool, chain, &nft_contract, &token_id, "marketplace:DutchAuctionCreated",
                    serde_json::json!({"auction_id": auction_id, "seller": seller}),
                    block_number, block_timestamp, &tx_hash, log_index,
                ).await;
            }
        } else if topic0 == DutchAuctionBought::SIGNATURE_HASH {
            if let Ok(decoded) = log.log_decode::<DutchAuctionBought>() {
                let e = &decoded.inner.data;
                let auction_id = e.auctionId.to::<u64>() as i64;
                let buyer = format!("{:#x}", e.buyer);
                let price = BigDecimal::from_str(&e.price.to_string()).unwrap_or_default();
                tracing::info!(chain_id = chain.chain_id, "DutchAuctionBought #{}", auction_id);
                if let Err(err) = db::marketplace::update_dutch_auction_status(
                    pool, auction_id, chain.chain_id, "Sold", Some(&buyer), Some(&price),
                ).await {
                    tracing::error!("Failed to update dutch auction {} as Sold: {:?}", auction_id, err);
                }
            }
        } else if topic0 == DutchAuctionCancelled::SIGNATURE_HASH {
            if let Ok(decoded) = log.log_decode::<DutchAuctionCancelled>() {
                let auction_id = decoded.inner.data.auctionId.to::<u64>() as i64;
                tracing::info!(chain_id = chain.chain_id, "DutchAuctionCancelled #{}", auction_id);
                if let Err(err) = db::marketplace::update_dutch_auction_status(
                    pool, auction_id, chain.chain_id, "Cancelled", None, None,
                ).await {
                    tracing::error!("Failed to cancel dutch auction {}: {:?}", auction_id, err);
                }
            }
        }
        // ─── Bundles ────────────────────────────────────────────────
        else if topic0 == BundleListed::SIGNATURE_HASH {
            if let Ok(decoded) = log.log_decode::<BundleListed>() {
                let e = &decoded.inner.data;
                let bundle_id = e.bundleId.to::<u64>() as i64;
                let seller = format!("{:#x}", e.seller);
                let item_count = e.itemCount.to::<u32>() as i32;
                let payment_token = format!("{:#x}", e.paymentToken);
                let price = BigDecimal::from_str(&e.price.to_string()).unwrap_or_default();
                let expiry = e.expiry.to::<u64>() as i64;

                tracing::info!(chain_id = chain.chain_id, "BundleListed #{} ({} items)", bundle_id, item_count);

                // BundleListed event doesn't include nft arrays — read from contract via getBundleListing()
                let (nft_contracts, token_ids) = {
                    use alloy::sol_types::SolCall;
                    let call = MoltMarketplace::getBundleListingCall {
                        bundleId: alloy::primitives::U256::from(bundle_id),
                    };
                    let tx = alloy::rpc::types::TransactionRequest::default()
                        .to(marketplace_address)
                        .input(alloy::primitives::Bytes::from(call.abi_encode()).into());
                    match provider.call(tx).await {
                        Ok(result_bytes) => {
                            match MoltMarketplace::getBundleListingCall::abi_decode_returns(&result_bytes) {
                                Ok(decoded) => {
                                    let nfts: Vec<String> = decoded.nftContracts.iter().map(|a| format!("{:#x}", a)).collect();
                                    let ids: Vec<BigDecimal> = decoded.tokenIds.iter().map(|id| BigDecimal::from_str(&id.to_string()).unwrap_or_default()).collect();
                                    (nfts, ids)
                                }
                                Err(err) => {
                                    tracing::warn!("Failed to decode bundle {} response: {:?}", bundle_id, err);
                                    (vec![], vec![])
                                }
                            }
                        }
                        Err(err) => {
                            tracing::warn!("Failed to read bundle {} from contract: {:?}", bundle_id, err);
                            (vec![], vec![])
                        }
                    }
                };

                let new_bundle = NewMarketplaceBundle {
                    bundle_id,
                    chain_id: chain.chain_id,
                    seller,
                    nft_contracts,
                    token_ids,
                    payment_token,
                    price,
                    expiry,
                    item_count,
                    block_number,
                    block_timestamp,
                    tx_hash: tx_hash.clone(),
                };
                if let Err(err) = db::marketplace::upsert_bundle(pool, &new_bundle).await {
                    tracing::error!("Failed to upsert bundle {}: {:?}", bundle_id, err);
                }
            }
        } else if topic0 == BundleBought::SIGNATURE_HASH {
            if let Ok(decoded) = log.log_decode::<BundleBought>() {
                let e = &decoded.inner.data;
                let bundle_id = e.bundleId.to::<u64>() as i64;
                let buyer = format!("{:#x}", e.buyer);
                let price = BigDecimal::from_str(&e.price.to_string()).unwrap_or_default();
                tracing::info!(chain_id = chain.chain_id, "BundleBought #{}", bundle_id);
                if let Err(err) = db::marketplace::update_bundle_status(
                    pool, bundle_id, chain.chain_id, "Sold", Some(&buyer), Some(&price),
                ).await {
                    tracing::error!("Failed to update bundle {} as Sold: {:?}", bundle_id, err);
                }
            }
        } else if topic0 == BundleListingCancelled::SIGNATURE_HASH {
            if let Ok(decoded) = log.log_decode::<BundleListingCancelled>() {
                let bundle_id = decoded.inner.data.bundleId.to::<u64>() as i64;
                tracing::info!(chain_id = chain.chain_id, "BundleListingCancelled #{}", bundle_id);
                if let Err(err) = db::marketplace::update_bundle_status(
                    pool, bundle_id, chain.chain_id, "Cancelled", None, None,
                ).await {
                    tracing::error!("Failed to cancel bundle {}: {:?}", bundle_id, err);
                }
            }
        }
        // ─── Config Events ──────────────────────────────────────────
        else if topic0 == PlatformFeeUpdated::SIGNATURE_HASH {
            if let Ok(decoded) = log.log_decode::<PlatformFeeUpdated>() {
                let new_fee = decoded.inner.data.newFee.to::<u32>() as i32;
                tracing::info!(chain_id = chain.chain_id, "PlatformFeeUpdated: {} bps", new_fee);
                if let Err(err) = db::marketplace::upsert_marketplace_config(
                    pool, chain.chain_id, Some(new_fee), None,
                ).await {
                    tracing::error!("Failed to update platform fee: {:?}", err);
                }
            }
        } else if topic0 == FeeRecipientUpdated::SIGNATURE_HASH {
            if let Ok(decoded) = log.log_decode::<FeeRecipientUpdated>() {
                let new_recipient = format!("{:#x}", decoded.inner.data.newRecipient);
                tracing::info!(chain_id = chain.chain_id, "FeeRecipientUpdated: {}", new_recipient);
                if let Err(err) = db::marketplace::upsert_marketplace_config(
                    pool, chain.chain_id, None, Some(&new_recipient),
                ).await {
                    tracing::error!("Failed to update fee recipient: {:?}", err);
                }
            }
        } else if topic0 == PaymentTokenAdded::SIGNATURE_HASH {
            if let Ok(decoded) = log.log_decode::<PaymentTokenAdded>() {
                let token = format!("{:#x}", decoded.inner.data.token);
                tracing::info!(chain_id = chain.chain_id, "PaymentTokenAdded: {}", token);
                if let Err(err) = db::marketplace::upsert_payment_token(
                    pool, chain.chain_id, &token, true, Some(block_number),
                ).await {
                    tracing::error!("Failed to add payment token: {:?}", err);
                }
            }
        } else if topic0 == PaymentTokenRemoved::SIGNATURE_HASH {
            if let Ok(decoded) = log.log_decode::<PaymentTokenRemoved>() {
                let token = format!("{:#x}", decoded.inner.data.token);
                tracing::info!(chain_id = chain.chain_id, "PaymentTokenRemoved: {}", token);
                if let Err(err) = db::marketplace::upsert_payment_token(
                    pool, chain.chain_id, &token, false, Some(block_number),
                ).await {
                    tracing::error!("Failed to remove payment token: {:?}", err);
                }
            }
        }
    }

    Ok(())
}

/// If the nft_contract matches the chain's identity_address, insert an activity log entry
/// for the agent NFT so marketplace events appear in the agent's activity feed.
async fn maybe_insert_agent_activity(
    pool: &PgPool,
    chain: &ChainConfig,
    nft_contract: &str,
    token_id: &BigDecimal,
    event_type: &str,
    event_data: serde_json::Value,
    block_number: i64,
    block_timestamp: Option<DateTime<Utc>>,
    tx_hash: &str,
    log_index: i32,
) {
    let identity_addr = format!("{:#x}", chain.identity_address);
    if nft_contract.to_lowercase() != identity_addr.to_lowercase() {
        return;
    }

    // Parse token_id as agent_id
    let agent_id = match token_id.to_string().parse::<i64>() {
        Ok(id) => id,
        Err(_) => return,
    };

    let activity = NewActivity {
        agent_id,
        chain_id: chain.chain_id,
        event_type: event_type.to_string(),
        event_data: Some(event_data),
        block_number,
        block_timestamp,
        tx_hash: tx_hash.to_string(),
        log_index,
    };

    if let Err(e) = db::activity::insert_activity(pool, &activity).await {
        tracing::error!("Failed to insert {} activity for agent {}: {:?}", event_type, agent_id, e);
    }
}

/// Read current marketplace config (platformFeeBps, feeRecipient) from on-chain
/// and upsert into the DB. Called at indexer startup to ensure config is in sync
/// even if initialize() didn't emit events.
pub async fn sync_marketplace_config(
    pool: &PgPool,
    provider: &HttpProvider,
    chain: &ChainConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let marketplace_address = match chain.marketplace_address {
        Some(addr) => addr,
        None => return Ok(()),
    };

    use alloy::sol_types::SolCall;

    // Read platformFeeBps
    let fee_call = MoltMarketplace::platformFeeBpsCall {};
    let fee_tx = alloy::rpc::types::TransactionRequest::default()
        .to(marketplace_address)
        .input(alloy::primitives::Bytes::from(fee_call.abi_encode()).into());
    let fee_bps = match provider.call(fee_tx).await {
        Ok(bytes) => {
            match MoltMarketplace::platformFeeBpsCall::abi_decode_returns(&bytes) {
                Ok(decoded) => Some(decoded.to::<u32>() as i32),
                Err(e) => { tracing::warn!("Failed to decode platformFeeBps: {:?}", e); None }
            }
        }
        Err(e) => { tracing::warn!("Failed to read platformFeeBps: {:?}", e); None }
    };

    // Read feeRecipient
    let recipient_call = MoltMarketplace::feeRecipientCall {};
    let recipient_tx = alloy::rpc::types::TransactionRequest::default()
        .to(marketplace_address)
        .input(alloy::primitives::Bytes::from(recipient_call.abi_encode()).into());
    let fee_recipient = match provider.call(recipient_tx).await {
        Ok(bytes) => {
            match MoltMarketplace::feeRecipientCall::abi_decode_returns(&bytes) {
                Ok(decoded) => Some(format!("{:#x}", decoded)),
                Err(e) => { tracing::warn!("Failed to decode feeRecipient: {:?}", e); None }
            }
        }
        Err(e) => { tracing::warn!("Failed to read feeRecipient: {:?}", e); None }
    };

    if fee_bps.is_some() || fee_recipient.is_some() {
        tracing::info!(
            chain_id = chain.chain_id,
            "Synced marketplace config: fee={:?} bps, recipient={:?}",
            fee_bps, fee_recipient
        );
        db::marketplace::upsert_marketplace_config(
            pool, chain.chain_id, fee_bps, fee_recipient.as_deref(),
        ).await?;
    }

    Ok(())
}
