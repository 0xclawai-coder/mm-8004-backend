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

    // Total agents
    let (total_agents,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM agents WHERE active = true")
            .fetch_one(&state.pool)
            .await
            .map_err(map_err)?;

    // Total feedbacks
    let (total_feedbacks,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM feedbacks WHERE revoked = false")
            .fetch_one(&state.pool)
            .await
            .map_err(map_err)?;

    // Total chains (distinct chain_ids)
    let (total_chains,): (i64,) =
        sqlx::query_as("SELECT COUNT(DISTINCT chain_id) FROM agents")
            .fetch_one(&state.pool)
            .await
            .map_err(map_err)?;

    // Agents by chain
    let chain_counts: Vec<(i32, i64)> = sqlx::query_as(
        "SELECT chain_id, COUNT(*) FROM agents WHERE active = true GROUP BY chain_id",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(map_err)?;

    let mut agents_by_chain: HashMap<String, i64> = HashMap::new();
    for (chain_id, count) in chain_counts {
        agents_by_chain.insert(chain_id.to_string(), count);
    }

    // Top categories — unnest the categories array and count
    let top_categories: Vec<CategoryCount> = sqlx::query_as(
        r#"
        SELECT cat AS category, COUNT(*) AS count
        FROM agents, UNNEST(categories) AS cat
        WHERE active = true
        GROUP BY cat
        ORDER BY count DESC
        LIMIT 10
        "#,
    )
    .fetch_all(&state.pool)
    .await
    .map_err(map_err)?;

    // Recent registrations in last 24h
    let (recent_registrations_24h,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM agents WHERE created_at >= NOW() - INTERVAL '24 hours'",
    )
    .fetch_one(&state.pool)
    .await
    .map_err(map_err)?;

    // Recent feedbacks in last 24h
    let (recent_feedbacks_24h,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM feedbacks WHERE created_at >= NOW() - INTERVAL '24 hours'",
    )
    .fetch_one(&state.pool)
    .await
    .map_err(map_err)?;

    // Marketplace stats
    let (total_listings,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM marketplace_listings")
            .fetch_one(&state.pool)
            .await
            .map_err(map_err)?;

    let (active_listings,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM marketplace_listings WHERE status = 'Active'")
            .fetch_one(&state.pool)
            .await
            .map_err(map_err)?;

    let (total_sales,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM marketplace_listings WHERE status = 'Sold'")
            .fetch_one(&state.pool)
            .await
            .map_err(map_err)?;

    let total_volume: BigDecimal = sqlx::query_scalar(
        "SELECT COALESCE(SUM(sold_price), 0) FROM marketplace_listings WHERE status = 'Sold'",
    )
    .fetch_one(&state.pool)
    .await
    .map_err(map_err)?;

    Ok(Json(StatsResponse {
        total_agents,
        total_feedbacks,
        total_chains,
        agents_by_chain,
        top_categories,
        recent_registrations_24h,
        recent_feedbacks_24h,
        total_listings,
        active_listings,
        total_sales,
        total_volume,
    }))
}
