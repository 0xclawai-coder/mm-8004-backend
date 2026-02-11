use sqlx::PgPool;

use crate::types::{Activity, NewActivity};

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
