use sqlx::PgPool;

use crate::types::{Activity, GlobalActivity, NewActivity};

/// Get paginated activity log for an agent, optionally filtered by event_type.
pub async fn get_activities(
    pool: &PgPool,
    agent_id: i64,
    chain_id: i32,
    event_type: Option<&str>,
    offset: i64,
    limit: i64,
) -> Result<(Vec<Activity>, i64), sqlx::Error> {
    // Map category names to actual event types stored in the DB.
    // Frontend sends 'identity' or 'reputation', but the DB stores specific event names.
    let activities: Vec<Activity> = sqlx::query_as(
        r#"
        SELECT id, agent_id, chain_id, event_type, event_data,
               block_number, COALESCE(block_timestamp, created_at) AS block_timestamp, tx_hash, log_index
        FROM activity_log
        WHERE agent_id = $1 AND chain_id = $2
          AND ($3::TEXT IS NULL
            OR ($3 = 'identity' AND event_type IN ('Registered', 'URIUpdated', 'MetadataSet'))
            OR ($3 = 'reputation' AND event_type IN ('NewFeedback', 'FeedbackRevoked', 'ResponseAppended'))
            OR ($3 = 'marketplace' AND event_type LIKE 'marketplace:%')
            OR event_type = $3)
        ORDER BY block_number DESC, log_index DESC
        LIMIT $4 OFFSET $5
        "#,
    )
    .bind(agent_id)
    .bind(chain_id)
    .bind(event_type)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let total: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)
        FROM activity_log
        WHERE agent_id = $1 AND chain_id = $2
          AND ($3::TEXT IS NULL
            OR ($3 = 'identity' AND event_type IN ('Registered', 'URIUpdated', 'MetadataSet'))
            OR ($3 = 'reputation' AND event_type IN ('NewFeedback', 'FeedbackRevoked', 'ResponseAppended'))
            OR ($3 = 'marketplace' AND event_type LIKE 'marketplace:%')
            OR event_type = $3)
        "#,
    )
    .bind(agent_id)
    .bind(chain_id)
    .bind(event_type)
    .fetch_one(pool)
    .await?;

    Ok((activities, total.0))
}

/// Get paginated global activity log (all agents), optionally filtered by event_type.
/// Includes agent name via LEFT JOIN.
pub async fn get_global_activities(
    pool: &PgPool,
    event_type: Option<&str>,
    chain_id: Option<i32>,
    offset: i64,
    limit: i64,
) -> Result<(Vec<GlobalActivity>, i64), sqlx::Error> {
    let activities: Vec<GlobalActivity> = sqlx::query_as(
        r#"
        SELECT a.id, a.agent_id, a.chain_id, a.event_type, a.event_data,
               a.block_number, COALESCE(a.block_timestamp, a.created_at) AS block_timestamp,
               a.tx_hash, a.log_index,
               ag.name AS agent_name, ag.image AS agent_image
        FROM activity_log a
        LEFT JOIN agents ag ON ag.agent_id = a.agent_id AND ag.chain_id = a.chain_id
        WHERE ($1::TEXT IS NULL
            OR ($1 = 'identity' AND a.event_type IN ('Registered', 'URIUpdated', 'MetadataSet'))
            OR ($1 = 'reputation' AND a.event_type IN ('NewFeedback', 'FeedbackRevoked', 'ResponseAppended'))
            OR ($1 = 'marketplace' AND a.event_type LIKE 'marketplace:%')
            OR a.event_type = $1)
          AND ($4::INT IS NULL OR a.chain_id = $4)
        ORDER BY COALESCE(a.block_timestamp, a.created_at) DESC, a.id DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(event_type)
    .bind(limit)
    .bind(offset)
    .bind(chain_id)
    .fetch_all(pool)
    .await?;

    let total: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)
        FROM activity_log a
        WHERE ($1::TEXT IS NULL
            OR ($1 = 'identity' AND a.event_type IN ('Registered', 'URIUpdated', 'MetadataSet'))
            OR ($1 = 'reputation' AND a.event_type IN ('NewFeedback', 'FeedbackRevoked', 'ResponseAppended'))
            OR ($1 = 'marketplace' AND a.event_type LIKE 'marketplace:%')
            OR a.event_type = $1)
          AND ($2::INT IS NULL OR a.chain_id = $2)
        "#,
    )
    .bind(event_type)
    .bind(chain_id)
    .fetch_one(pool)
    .await?;

    Ok((activities, total.0))
}

/// Insert a new activity log entry.
pub async fn insert_activity(pool: &PgPool, activity: &NewActivity) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO activity_log (agent_id, chain_id, event_type, event_data, block_number, block_timestamp, tx_hash, log_index)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
    )
    .bind(activity.agent_id)
    .bind(activity.chain_id)
    .bind(&activity.event_type)
    .bind(&activity.event_data)
    .bind(activity.block_number)
    .bind(activity.block_timestamp)
    .bind(&activity.tx_hash)
    .bind(activity.log_index)
    .execute(pool)
    .await?;

    Ok(())
}
