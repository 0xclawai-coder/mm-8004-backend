use sqlx::PgPool;

use crate::types::{Feedback, NewFeedback, ReputationHistoryPoint};

/// Get feedbacks for an agent with optional time range filtering.
/// Range is one of: "7d", "30d", "90d", "all"
pub async fn get_feedbacks_for_agent(
    pool: &PgPool,
    agent_id: i64,
    chain_id: i32,
    range: &str,
) -> Result<Vec<Feedback>, sqlx::Error> {
    let interval = match range {
        "7d" => Some("7 days"),
        "30d" => Some("30 days"),
        "90d" => Some("90 days"),
        _ => None, // "all"
    };

    // SAFETY: interval_str is from a hardcoded whitelist, not user input
    let feedbacks: Vec<Feedback> = if let Some(interval_str) = interval {
        let query = format!(
            r#"
            SELECT id, agent_id, chain_id, client_address, feedback_index,
                   value, value_decimals, tag1, tag2, endpoint, feedback_uri,
                   feedback_hash, revoked, block_number, block_timestamp, tx_hash, created_at
            FROM feedbacks
            WHERE agent_id = $1 AND chain_id = $2
              AND created_at >= NOW() - INTERVAL '{}'
            ORDER BY created_at DESC
            LIMIT 500
            "#,
            interval_str
        );
        sqlx::query_as(&query)
            .bind(agent_id)
            .bind(chain_id)
            .fetch_all(pool)
            .await?
    } else {
        // "all" range — still cap at 500 to prevent unbounded responses
        sqlx::query_as(
            r#"
            SELECT id, agent_id, chain_id, client_address, feedback_index,
                   value, value_decimals, tag1, tag2, endpoint, feedback_uri,
                   feedback_hash, revoked, block_number, block_timestamp, tx_hash, created_at
            FROM feedbacks
            WHERE agent_id = $1 AND chain_id = $2
            ORDER BY created_at DESC
            LIMIT 500
            "#,
        )
        .bind(agent_id)
        .bind(chain_id)
        .fetch_all(pool)
        .await?
    };

    Ok(feedbacks)
}

/// Get daily aggregated reputation scores for an agent within a time range.
pub async fn get_reputation_history(
    pool: &PgPool,
    agent_id: i64,
    chain_id: i32,
    range: &str,
) -> Result<Vec<ReputationHistoryPoint>, sqlx::Error> {
    let interval = match range {
        "7d" => Some("7 days"),
        "30d" => Some("30 days"),
        "90d" => Some("90 days"),
        _ => None,
    };

    // SAFETY: interval_str is from a hardcoded whitelist, not user input
    let rows: Vec<ReputationHistoryPoint> = if let Some(interval_str) = interval {
        let query = format!(
            r#"
            SELECT
                DATE(created_at) AS date,
                AVG(CASE WHEN revoked = false THEN value / POWER(10, COALESCE(value_decimals, 0)) ELSE NULL END)::FLOAT8 AS score,
                COUNT(CASE WHEN revoked = false THEN 1 ELSE NULL END) AS feedback_count
            FROM feedbacks
            WHERE agent_id = $1 AND chain_id = $2
              AND created_at >= NOW() - INTERVAL '{}'
            GROUP BY DATE(created_at)
            ORDER BY date ASC
            LIMIT 365
            "#,
            interval_str
        );
        sqlx::query_as(&query)
            .bind(agent_id)
            .bind(chain_id)
            .fetch_all(pool)
            .await?
    } else {
        // "all" range — cap at 365 days of history to prevent unbounded results
        sqlx::query_as(
            r#"
            SELECT
                DATE(created_at) AS date,
                AVG(CASE WHEN revoked = false THEN value / POWER(10, COALESCE(value_decimals, 0)) ELSE NULL END)::FLOAT8 AS score,
                COUNT(CASE WHEN revoked = false THEN 1 ELSE NULL END) AS feedback_count
            FROM feedbacks
            WHERE agent_id = $1 AND chain_id = $2
            GROUP BY DATE(created_at)
            ORDER BY date ASC
            LIMIT 365
            "#,
        )
        .bind(agent_id)
        .bind(chain_id)
        .fetch_all(pool)
        .await?
    };

    Ok(rows)
}

/// Insert a new feedback record.
pub async fn insert_feedback(pool: &PgPool, feedback: &NewFeedback) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO feedbacks (agent_id, chain_id, client_address, feedback_index, value, value_decimals, tag1, tag2, endpoint, feedback_uri, feedback_hash, block_number, block_timestamp, tx_hash)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
        "#,
    )
    .bind(feedback.agent_id)
    .bind(feedback.chain_id)
    .bind(&feedback.client_address)
    .bind(feedback.feedback_index)
    .bind(&feedback.value)
    .bind(feedback.value_decimals)
    .bind(&feedback.tag1)
    .bind(&feedback.tag2)
    .bind(&feedback.endpoint)
    .bind(&feedback.feedback_uri)
    .bind(&feedback.feedback_hash)
    .bind(feedback.block_number)
    .bind(feedback.block_timestamp)
    .bind(&feedback.tx_hash)
    .execute(pool)
    .await?;

    Ok(())
}

/// Mark a feedback as revoked by agent_id, chain_id, and feedback_index.
pub async fn revoke_feedback(
    pool: &PgPool,
    agent_id: i64,
    chain_id: i32,
    feedback_index: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE feedbacks
        SET revoked = true
        WHERE agent_id = $1 AND chain_id = $2 AND feedback_index = $3
        "#,
    )
    .bind(agent_id)
    .bind(chain_id)
    .bind(feedback_index)
    .execute(pool)
    .await?;

    Ok(())
}
