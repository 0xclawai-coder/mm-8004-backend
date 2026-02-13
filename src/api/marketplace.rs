use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};

use crate::db;
use crate::types::{
    ErrorResponse, MarketplaceAuctionDetailResponse, MarketplaceAuctionListResponse,
    MarketplaceAuctionParams, MarketplaceBundleListResponse, MarketplaceBundleParams,
    MarketplaceCollectionOfferListResponse, MarketplaceCollectionOfferParams,
    MarketplaceDutchAuctionListResponse, MarketplaceListParams, MarketplaceListingListResponse,
    MarketplaceOfferListResponse, MarketplaceOfferParams, MarketplaceUserParams,
};
use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/marketplace/listings", get(list_listings))
        .route("/marketplace/listings/{id}", get(get_listing))
        .route("/marketplace/offers", get(list_offers))
        .route("/marketplace/collection-offers", get(list_collection_offers))
        .route("/marketplace/auctions", get(list_auctions))
        .route("/marketplace/auctions/{id}", get(get_auction))
        .route("/marketplace/dutch-auctions", get(list_dutch_auctions))
        .route("/marketplace/bundles", get(list_bundles))
        .route("/marketplace/user/{address}", get(get_user_portfolio))
        .route("/marketplace/stats", get(get_marketplace_stats))
}

/// Parse a composite ID in the format "chainId-entityId" (e.g., "143-1")
fn parse_id(id: &str) -> Result<(i32, i64), (StatusCode, Json<ErrorResponse>)> {
    let parts: Vec<&str> = id.splitn(2, '-').collect();
    if parts.len() != 2 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Bad Request".to_string(),
                message: format!("Invalid id format '{}'. Expected 'chainId-entityId'.", id),
                status: 400,
            }),
        ));
    }

    let chain_id: i32 = parts[0].parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Bad Request".to_string(),
                message: format!("Invalid chain_id in '{}'", id),
                status: 400,
            }),
        )
    })?;

    let entity_id: i64 = parts[1].parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Bad Request".to_string(),
                message: format!("Invalid entity_id in '{}'", id),
                status: 400,
            }),
        )
    })?;

    Ok((chain_id, entity_id))
}

fn map_err(e: sqlx::Error) -> (StatusCode, Json<ErrorResponse>) {
    tracing::error!("Marketplace DB error: {:?}", e);
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "Internal Server Error".to_string(),
            message: "Failed to fetch marketplace data".to_string(),
            status: 500,
        }),
    )
}

/// GET /api/marketplace/listings
async fn list_listings(
    State(state): State<AppState>,
    Query(params): Query<MarketplaceListParams>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let (listings, total) = db::marketplace::get_listings(
        &state.pool,
        params.chain_id,
        params.nft_contract.as_deref(),
        params.seller.as_deref(),
        params.status(),
        params.sort(),
        params.offset(),
        params.limit(),
    )
    .await
    .map_err(map_err)?;

    Ok(Json(MarketplaceListingListResponse {
        listings,
        total,
        page: params.page(),
        limit: params.limit(),
    }))
}

/// GET /api/marketplace/listings/:chainId-:listingId
async fn get_listing(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let (chain_id, listing_id) = parse_id(&id)?;
    let listing = db::marketplace::get_listing_by_id(&state.pool, listing_id, chain_id)
        .await
        .map_err(map_err)?;

    match listing {
        Some(l) => {
            // Embed agent data in the listing response to avoid a second API call
            let token_id_i64 = l.token_id.to_string().parse::<i64>().unwrap_or(0);
            let agent = db::agents::get_agent_by_id(&state.pool, token_id_i64, chain_id)
                .await
                .ok()
                .flatten();
            let scores = if agent.is_some() {
                db::agents::get_scores_by_tag(&state.pool, token_id_i64, chain_id)
                    .await
                    .unwrap_or_default()
            } else {
                vec![]
            };

            let mut response = serde_json::to_value(&l).unwrap();
            if let Some(a) = agent {
                let agent_detail = crate::types::AgentDetailResponse { agent: a, scores };
                response["agent"] = serde_json::to_value(&agent_detail).unwrap();
            }
            Ok(Json(response))
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Not found".to_string(),
                message: format!("Listing {} not found", id),
                status: 404,
            }),
        )),
    }
}

/// GET /api/marketplace/offers
async fn list_offers(
    State(state): State<AppState>,
    Query(params): Query<MarketplaceOfferParams>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let (offers, total) = db::marketplace::get_offers(
        &state.pool,
        params.chain_id,
        params.nft_contract.as_deref(),
        params.token_id.as_deref(),
        params.offerer.as_deref(),
        params.status.as_deref(),
        params.offset(),
        params.limit(),
    )
    .await
    .map_err(map_err)?;

    Ok(Json(MarketplaceOfferListResponse {
        offers,
        total,
        page: params.page(),
        limit: params.limit(),
    }))
}

/// GET /api/marketplace/collection-offers
async fn list_collection_offers(
    State(state): State<AppState>,
    Query(params): Query<MarketplaceCollectionOfferParams>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let (offers, total) = db::marketplace::get_collection_offers(
        &state.pool,
        params.chain_id,
        params.nft_contract.as_deref(),
        params.offerer.as_deref(),
        params.status.as_deref(),
        params.offset(),
        params.limit(),
    )
    .await
    .map_err(map_err)?;

    Ok(Json(MarketplaceCollectionOfferListResponse {
        offers,
        total,
        page: params.page(),
        limit: params.limit(),
    }))
}

/// GET /api/marketplace/auctions
async fn list_auctions(
    State(state): State<AppState>,
    Query(params): Query<MarketplaceAuctionParams>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let (auctions, total) = db::marketplace::get_auctions(
        &state.pool,
        params.chain_id,
        params.nft_contract.as_deref(),
        params.seller.as_deref(),
        params.status.as_deref(),
        params.sort(),
        params.offset(),
        params.limit(),
    )
    .await
    .map_err(map_err)?;

    Ok(Json(MarketplaceAuctionListResponse {
        auctions,
        total,
        page: params.page(),
        limit: params.limit(),
    }))
}

/// GET /api/marketplace/auctions/:chainId-:auctionId (includes bid history)
async fn get_auction(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let (chain_id, auction_id) = parse_id(&id)?;
    let result = db::marketplace::get_auction_with_bids(&state.pool, auction_id, chain_id)
        .await
        .map_err(map_err)?;

    match result {
        Some((auction, bids)) => Ok(Json(MarketplaceAuctionDetailResponse { auction, bids })),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Not found".to_string(),
                message: format!("Auction {} not found", id),
                status: 404,
            }),
        )),
    }
}

/// GET /api/marketplace/dutch-auctions
async fn list_dutch_auctions(
    State(state): State<AppState>,
    Query(params): Query<MarketplaceListParams>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let (auctions, total) = db::marketplace::get_dutch_auctions(
        &state.pool,
        params.chain_id,
        params.nft_contract.as_deref(),
        params.status.as_deref(),
        params.offset(),
        params.limit(),
    )
    .await
    .map_err(map_err)?;

    Ok(Json(MarketplaceDutchAuctionListResponse {
        auctions,
        total,
        page: params.page(),
        limit: params.limit(),
    }))
}

/// GET /api/marketplace/bundles
async fn list_bundles(
    State(state): State<AppState>,
    Query(params): Query<MarketplaceBundleParams>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let (bundles, total) = db::marketplace::get_bundles(
        &state.pool,
        params.chain_id,
        params.seller.as_deref(),
        params.status.as_deref(),
        params.offset(),
        params.limit(),
    )
    .await
    .map_err(map_err)?;

    Ok(Json(MarketplaceBundleListResponse {
        bundles,
        total,
        page: params.page(),
        limit: params.limit(),
    }))
}

/// GET /api/marketplace/user/:address
async fn get_user_portfolio(
    State(state): State<AppState>,
    Path(address): Path<String>,
    Query(params): Query<MarketplaceUserParams>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let portfolio = db::marketplace::get_user_portfolio(
        &state.pool,
        &address.to_lowercase(),
        params.chain_id,
    )
    .await
    .map_err(map_err)?;

    Ok(Json(portfolio))
}

/// GET /api/marketplace/stats
async fn get_marketplace_stats(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let stats = db::marketplace::get_marketplace_stats(&state.pool)
        .await
        .map_err(map_err)?;

    Ok(Json(stats))
}
