use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};

use crate::types::{ErrorResponse, LeaderboardEntry, LeaderboardParams, LeaderboardResponse};
use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/leaderboard", get(get_leaderboard))
}

/// GET /api/leaderboard — get ranked agents by reputation score
async fn get_leaderboard(
    State(state): State<AppState>,
    Query(params): Query<LeaderboardParams>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let limit = params.limit();

    let entries: Vec<LeaderboardEntry> = sqlx::query_as(
        r#"
        SELECT
            ROW_NUMBER() OVER (ORDER BY AVG(CASE WHEN f.revoked = false THEN f.value / POWER(10, COALESCE(f.value_decimals, 0)) ELSE NULL END) DESC NULLS LAST) AS rank,
            a.agent_id,
            a.chain_id,
            a.name,
            a.image,
            a.categories,
            a.x402_support,
            AVG(CASE WHEN f.revoked = false THEN f.value / POWER(10, COALESCE(f.value_decimals, 0)) ELSE NULL END)::FLOAT8 AS reputation_score,
            COUNT(CASE WHEN f.revoked = false THEN 1 ELSE NULL END) AS feedback_count,
            a.owner
        FROM agents a
        LEFT JOIN feedbacks f ON a.agent_id = f.agent_id AND a.chain_id = f.chain_id
        WHERE a.active = true
          AND ($1::INT IS NULL OR a.chain_id = $1)
          AND ($2::TEXT IS NULL OR (
                CASE WHEN $2 = 'others'
                    THEN (a.categories IS NULL OR cardinality(a.categories) = 0 OR NOT (a.categories && ARRAY['defi','analytics','security','identity','trading','ai','compute','gaming','social','dao']))
                    ELSE $2 = ANY(a.categories)
                END
            ))
        GROUP BY a.id
        HAVING COUNT(CASE WHEN f.revoked = false THEN 1 ELSE NULL END) > 0
        ORDER BY reputation_score DESC NULLS LAST
        LIMIT $3
        "#,
    )
    .bind(params.chain_id)
    .bind(params.category.as_deref())
    .bind(limit)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to get leaderboard: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Internal Server Error".to_string(),
                message: "Failed to fetch leaderboard".to_string(),
                status: 500,
            }),
        )
    })?;

    Ok(Json(LeaderboardResponse {
        leaderboard: entries,
    }))
}
