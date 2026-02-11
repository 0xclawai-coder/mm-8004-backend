use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};

use crate::db;
use crate::types::{
    ActivityParams, ActivityResponse, AgentListParams, AgentListResponse, ErrorResponse,
    PaginationParams, ReputationParams, ReputationResponse,
};
use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/agents", get(list_agents))
        .route("/agents/{id}", get(get_agent))
        .route("/agents/{id}/reputation", get(get_agent_reputation))
        .route("/agents/{id}/activity", get(get_agent_activity))
        .route("/agents/{id}/marketplace", get(get_agent_marketplace))
}

/// Parse an agent path ID in the format "chainId-agentId" (e.g., "143-1")
fn parse_agent_id(id: &str) -> Result<(i32, i64), (StatusCode, Json<ErrorResponse>)> {
    let parts: Vec<&str> = id.splitn(2, '-').collect();
    if parts.len() != 2 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Bad Request".to_string(),
                message: format!("Invalid agent id format '{}'. Expected 'chainId-agentId'.", id),
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

    let agent_id: i64 = parts[1].parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Bad Request".to_string(),
                message: format!("Invalid agent_id in '{}'", id),
                status: 400,
            }),
        )
    })?;

    Ok((chain_id, agent_id))
}

/// GET /api/agents — list agents with optional filters and pagination
async fn list_agents(
    State(state): State<AppState>,
    Query(params): Query<AgentListParams>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let (agents, total) = db::agents::get_agents(
        &state.pool,
        params.chain_id,
        params.search.as_deref(),
        params.category.as_deref(),
        params.sort(),
        params.offset(),
        params.limit(),
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to get agents: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Internal Server Error".to_string(),
                message: "Failed to fetch agents".to_string(),
                status: 500,
            }),
        )
    })?;

    Ok(Json(AgentListResponse {
        agents,
        total,
        page: params.page(),
        limit: params.limit(),
    }))
}

/// GET /api/agents/:id — get single agent detail
async fn get_agent(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let (chain_id, agent_id) = parse_agent_id(&id)?;

    let agent = db::agents::get_agent_by_id(&state.pool, agent_id, chain_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get agent: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal Server Error".to_string(),
                    message: "Failed to fetch agent".to_string(),
                    status: 500,
                }),
            )
        })?;

    match agent {
        Some(a) => Ok(Json(a)),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Not found".to_string(),
                message: format!("Agent with id {} not found", id),
                status: 404,
            }),
        )),
    }
}

/// GET /api/agents/:id/reputation — get reputation history and feedbacks
async fn get_agent_reputation(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<ReputationParams>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let (chain_id, agent_id) = parse_agent_id(&id)?;
    let range = params.range();

    let history = db::feedbacks::get_reputation_history(&state.pool, agent_id, chain_id, range)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get reputation history: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal Server Error".to_string(),
                    message: "Failed to fetch reputation history".to_string(),
                    status: 500,
                }),
            )
        })?;

    let feedbacks = db::feedbacks::get_feedbacks_for_agent(&state.pool, agent_id, chain_id, range)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get feedbacks: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal Server Error".to_string(),
                    message: "Failed to fetch feedbacks".to_string(),
                    status: 500,
                }),
            )
        })?;

    // Compute current score from the latest history point or overall average
    let current_score = history.last().and_then(|h| h.score);

    Ok(Json(ReputationResponse {
        agent_id,
        chain_id,
        current_score,
        history,
        feedbacks,
    }))
}

/// GET /api/agents/:id/activity — get activity log for an agent
async fn get_agent_activity(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<ActivityParams>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let (chain_id, agent_id) = parse_agent_id(&id)?;

    let (activities, total) = db::activity::get_activities(
        &state.pool,
        agent_id,
        chain_id,
        params.event_type.as_deref(),
        params.offset(),
        params.limit(),
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to get activities: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Internal Server Error".to_string(),
                message: "Failed to fetch activities".to_string(),
                status: 500,
            }),
        )
    })?;

    Ok(Json(ActivityResponse {
        activities,
        total,
        page: params.page(),
        limit: params.limit(),
    }))
}

/// GET /api/agents/:id/marketplace — get marketplace activity for an agent NFT
async fn get_agent_marketplace(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<PaginationParams>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let (chain_id, agent_id) = parse_agent_id(&id)?;

    // Fetch marketplace-related activities for this agent
    let (activities, total) = db::activity::get_activities(
        &state.pool,
        agent_id,
        chain_id,
        Some("marketplace"),
        params.offset(),
        params.limit(),
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to get agent marketplace history: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Internal Server Error".to_string(),
                message: "Failed to fetch marketplace history".to_string(),
                status: 500,
            }),
        )
    })?;

    Ok(Json(ActivityResponse {
        activities,
        total,
        page: params.page(),
        limit: params.limit(),
    }))
}
