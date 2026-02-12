use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};

use crate::db;
use crate::types::{ActivityParams, ErrorResponse, GlobalActivityResponse};
use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/activity", get(get_global_activity))
}

/// GET /api/activity — global activity feed across all agents
async fn get_global_activity(
    State(state): State<AppState>,
    Query(params): Query<ActivityParams>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let (activities, total) = db::activity::get_global_activities(
        &state.pool,
        params.event_type.as_deref(),
        params.offset(),
        params.limit(),
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to get global activities: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Internal Server Error".to_string(),
                message: "Failed to fetch global activities".to_string(),
                status: 500,
            }),
        )
    })?;

    Ok(Json(GlobalActivityResponse {
        activities,
        total,
        page: params.page(),
        limit: params.limit(),
    }))
}
