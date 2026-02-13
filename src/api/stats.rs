use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use bigdecimal::BigDecimal;
use std::collections::HashMap;

use crate::types::{CategoryCount, ErrorResponse, StatsResponse};
use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/stats", get(get_stats))
}

/// Combined agent + feedback stats row (from a single query).
#[derive(Debug, sqlx::FromRow)]
struct AgentFeedbackStats {
    total_agents: i64,
    total_feedbacks: i64,
    total_chains: i64,
    recent_registrations_24h: i64,
    recent_feedbacks_24h: i64,
}

/// Combined marketplace stats row (from a single query).
#[derive(Debug, sqlx::FromRow)]
struct MarketplaceStats {
    total_listings: i64,
    active_listings: i64,
    total_sales: i64,
    total_volume: BigDecimal,
}

/// GET /api/stats — get global marketplace statistics
async fn get_stats(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let map_err = |e: sqlx::Error| {
        tracing::error!("Failed to get stats: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Internal Server Error".to_string(),
                message: "Failed to fetch stats".to_string(),
                status: 500,
            }),
        )
    };

    // Query 1: Combined agent + feedback counts (was 5 separate queries)
    let af_stats: AgentFeedbackStats = sqlx::query_as(
        r#"
        SELECT
            (SELECT COUNT(*) FROM agents WHERE active = true) AS total_agents,
            (SELECT COUNT(*) FROM feedbacks WHERE revoked = false) AS total_feedbacks,
            (SELECT COUNT(DISTINCT chain_id) FROM agents) AS total_chains,
            (SELECT COUNT(*) FROM agents WHERE created_at >= NOW() - INTERVAL '24 hours') AS recent_registrations_24h,
            (SELECT COUNT(*) FROM feedbacks WHERE created_at >= NOW() - INTERVAL '24 hours') AS recent_feedbacks_24h
        "#,
    )
    .fetch_one(&state.pool)
    .await
    .map_err(map_err)?;

    // Query 2: Agents by chain + top categories (run concurrently)
    let pool = &state.pool;
    let (chain_counts_result, top_categories_result, mp_stats_result) = tokio::join!(
        sqlx::query_as::<_, (i32, i64)>(
            "SELECT chain_id, COUNT(*) FROM agents WHERE active = true GROUP BY chain_id"
        )
        .fetch_all(pool),
        sqlx::query_as::<_, CategoryCount>(
            r#"
            SELECT cat AS category, COUNT(*) AS count
            FROM agents, UNNEST(categories) AS cat
            WHERE active = true
            GROUP BY cat
            ORDER BY count DESC
            LIMIT 10
            "#
        )
        .fetch_all(pool),
        // Query 3: Marketplace stats combined (was 4 separate queries)
        sqlx::query_as::<_, MarketplaceStats>(
            r#"
            SELECT
                COUNT(*) AS total_listings,
                COUNT(*) FILTER (WHERE status = 'Active') AS active_listings,
                COUNT(*) FILTER (WHERE status = 'Sold') AS total_sales,
                COALESCE(SUM(sold_price) FILTER (WHERE status = 'Sold'), 0) AS total_volume
            FROM marketplace_listings
            "#
        )
        .fetch_one(pool)
    );

    let chain_counts = chain_counts_result.map_err(map_err)?;
    let top_categories = top_categories_result.map_err(map_err)?;
    let mp_stats = mp_stats_result.map_err(map_err)?;

    let mut agents_by_chain: HashMap<String, i64> = HashMap::new();
    for (chain_id, count) in chain_counts {
        agents_by_chain.insert(chain_id.to_string(), count);
    }

    Ok(Json(StatsResponse {
        total_agents: af_stats.total_agents,
        total_feedbacks: af_stats.total_feedbacks,
        total_chains: af_stats.total_chains,
        agents_by_chain,
        top_categories,
        recent_registrations_24h: af_stats.recent_registrations_24h,
        recent_feedbacks_24h: af_stats.recent_feedbacks_24h,
        total_listings: mp_stats.total_listings,
        active_listings: mp_stats.active_listings,
        total_sales: mp_stats.total_sales,
        total_volume: mp_stats.total_volume,
    }))
}
