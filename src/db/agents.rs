use sqlx::PgPool;

use crate::types::{AgentDetailRow, AgentListItem, NewAgent, ScoreByTag, ScoreByTagRow};

/// Get a paginated list of agents with optional filtering, search, and sorting.
/// LEFT JOINs feedbacks to compute average reputation score and feedback count.
pub async fn get_agents(
    pool: &PgPool,
    chain_id: Option<i32>,
    search: Option<&str>,
    category: Option<&str>,
    owner: Option<&str>,
    sort: &str,
    offset: i64,
    limit: i64,
) -> Result<(Vec<AgentListItem>, i64), sqlx::Error> {
    // Build the ORDER BY clause based on sort parameter
    // SAFETY: order_clause is from a hardcoded whitelist, not user input
    let order_clause = match sort {
        "score" => "reputation_score DESC NULLS LAST",
        "name" => "a.name ASC NULLS LAST",
        _ => "a.created_at DESC NULLS LAST", // "recent" default
    };

    // We use a raw query approach with format since sqlx doesn't support dynamic ORDER BY
    // in the macro. We build the query as a string.
    let base_query = format!(
        r#"
        SELECT
            a.agent_id,
            a.chain_id,
            a.owner,
            a.name,
            a.description,
            a.image,
            a.categories,
            a.x402_support,
            a.active,
            AVG(CASE WHEN f.revoked = false THEN f.value / POWER(10, COALESCE(f.value_decimals, 0)) ELSE NULL END)::FLOAT8 AS reputation_score,
            COUNT(CASE WHEN f.revoked = false THEN 1 ELSE NULL END) AS feedback_count,
            COALESCE(a.block_timestamp, a.created_at) AS block_timestamp
        FROM agents a
        LEFT JOIN feedbacks f ON a.agent_id = f.agent_id AND a.chain_id = f.chain_id
        WHERE 1=1
            AND ($1::INT IS NULL OR a.chain_id = $1)
            AND ($2::TEXT IS NULL OR a.name ILIKE '%' || $2 || '%' OR a.description ILIKE '%' || $2 || '%')
            AND ($3::TEXT IS NULL OR (
                CASE WHEN $3 = 'others'
                    THEN NOT (a.categories && ARRAY['defi','analytics','security','identity','trading','ai','compute','gaming','social','dao'])
                    ELSE $3 = ANY(a.categories)
                END
            ))
            AND ($4::TEXT IS NULL OR LOWER(a.owner) = LOWER($4))
        GROUP BY a.id
        ORDER BY {}
        LIMIT $5 OFFSET $6
        "#,
        order_clause
    );

    let agents: Vec<AgentListItem> = sqlx::query_as(&base_query)
        .bind(chain_id)
        .bind(search)
        .bind(category)
        .bind(owner)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

    // Count total matching agents
    let total: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)
        FROM agents a
        WHERE 1=1
            AND ($1::INT IS NULL OR a.chain_id = $1)
            AND ($2::TEXT IS NULL OR a.name ILIKE '%' || $2 || '%' OR a.description ILIKE '%' || $2 || '%')
            AND ($3::TEXT IS NULL OR (
                CASE WHEN $3 = 'others'
                    THEN NOT (a.categories && ARRAY['defi','analytics','security','identity','trading','ai','compute','gaming','social','dao'])
                    ELSE $3 = ANY(a.categories)
                END
            ))
            AND ($4::TEXT IS NULL OR LOWER(a.owner) = LOWER($4))
        "#,
    )
    .bind(chain_id)
    .bind(search)
    .bind(category)
    .bind(owner)
    .fetch_one(pool)
    .await?;

    Ok((agents, total.0))
}

/// Classify a score into a scale type based on tag name and value range.
fn classify_scale(tag: &str, min_val: f64, max_val: f64) -> &'static str {
    if tag == "elo" {
        return "elo";
    }
    if tag == "slash" {
        return "raw";
    }
    // Boolean: values are exactly 0, 1, or -1
    if (min_val == 0.0 || min_val == 1.0 || min_val == -1.0)
        && (max_val == 0.0 || max_val == 1.0 || max_val == -1.0)
    {
        return "boolean";
    }
    // Percentage: all values in 0~100 range
    if min_val >= 0.0 && max_val <= 100.0 {
        return "percentage";
    }
    "raw"
}

/// Sort priority: percentage first, then elo, then others
fn scale_priority(scale: &str) -> u8 {
    match scale {
        "percentage" => 0,
        "elo" => 1,
        "boolean" => 2,
        _ => 3,
    }
}

/// Get scores grouped by tag1 for an agent, classified by scale and sorted.
pub async fn get_scores_by_tag(
    pool: &PgPool,
    agent_id: i64,
    chain_id: i32,
) -> Result<Vec<ScoreByTag>, sqlx::Error> {
    let rows: Vec<ScoreByTagRow> = sqlx::query_as(
        r#"
        SELECT
            tag1 AS score_type,
            MODE() WITHIN GROUP (ORDER BY tag2) AS label,
            AVG(value / POWER(10, COALESCE(value_decimals, 0)))::FLOAT8 AS value,
            COUNT(*)::BIGINT AS count,
            MIN(value / POWER(10, COALESCE(value_decimals, 0)))::FLOAT8 AS min_value,
            MAX(value / POWER(10, COALESCE(value_decimals, 0)))::FLOAT8 AS max_value
        FROM feedbacks
        WHERE agent_id = $1 AND chain_id = $2 AND revoked = false AND tag1 IS NOT NULL
        GROUP BY tag1
        ORDER BY count DESC
        "#,
    )
    .bind(agent_id)
    .bind(chain_id)
    .fetch_all(pool)
    .await?;

    let mut scores: Vec<ScoreByTag> = rows
        .into_iter()
        .map(|r| {
            let scale = classify_scale(
                &r.score_type,
                r.min_value.unwrap_or(0.0),
                r.max_value.unwrap_or(0.0),
            );
            ScoreByTag {
                score_type: r.score_type,
                label: r.label,
                value: r.value,
                count: r.count,
                min_value: r.min_value,
                max_value: r.max_value,
                scale: scale.to_string(),
            }
        })
        .collect();

    // Sort: percentage first, then elo, then boolean, then raw. Within same scale, by count desc.
    scores.sort_by(|a, b| {
        scale_priority(&a.scale)
            .cmp(&scale_priority(&b.scale))
            .then(b.count.cmp(&a.count))
    });

    Ok(scores)
}

/// Get a single agent by agent_id and chain_id, with reputation data.
pub async fn get_agent_by_id(
    pool: &PgPool,
    agent_id: i64,
    chain_id: i32,
) -> Result<Option<AgentDetailRow>, sqlx::Error> {
    let row: Option<AgentDetailRow> = sqlx::query_as(
        r#"
        SELECT
            a.agent_id,
            a.chain_id,
            a.owner,
            a.uri,
            a.name,
            a.description,
            a.image,
            a.categories,
            a.x402_support,
            a.active,
            a.metadata,
            AVG(CASE WHEN f.revoked = false THEN f.value / POWER(10, COALESCE(f.value_decimals, 0)) ELSE NULL END)::FLOAT8 AS reputation_score,
            COUNT(CASE WHEN f.revoked = false THEN 1 ELSE NULL END) AS feedback_count,
            COUNT(CASE WHEN f.revoked = false AND f.value / POWER(10, COALESCE(f.value_decimals, 0)) >= 3 THEN 1 ELSE NULL END) AS positive_feedback_count,
            COUNT(CASE WHEN f.revoked = false AND f.value / POWER(10, COALESCE(f.value_decimals, 0)) < 3 THEN 1 ELSE NULL END) AS negative_feedback_count,
            COALESCE(a.block_timestamp, a.created_at) AS block_timestamp
        FROM agents a
        LEFT JOIN feedbacks f ON a.agent_id = f.agent_id AND a.chain_id = f.chain_id
        WHERE a.agent_id = $1 AND a.chain_id = $2
        GROUP BY a.id
        "#,
    )
    .bind(agent_id)
    .bind(chain_id)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

/// Update the owner of an agent when a Transfer event is detected.
pub async fn update_agent_owner(
    pool: &PgPool,
    agent_id: i64,
    chain_id: i32,
    new_owner: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE agents
        SET owner = $3, updated_at = NOW()
        WHERE agent_id = $1 AND chain_id = $2
        "#,
    )
    .bind(agent_id)
    .bind(chain_id)
    .bind(new_owner)
    .execute(pool)
    .await?;

    Ok(())
}

/// Insert or update an agent on conflict (agent_id, chain_id).
pub async fn upsert_agent(pool: &PgPool, agent: &NewAgent) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO agents (agent_id, chain_id, owner, uri, metadata, name, description, image, categories, x402_support, active, block_number, block_timestamp, tx_hash)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
        ON CONFLICT (agent_id, chain_id) DO UPDATE SET
            owner = CASE WHEN EXCLUDED.owner = '' THEN agents.owner ELSE EXCLUDED.owner END,
            uri = COALESCE(EXCLUDED.uri, agents.uri),
            metadata = COALESCE(EXCLUDED.metadata, agents.metadata),
            name = COALESCE(EXCLUDED.name, agents.name),
            description = COALESCE(EXCLUDED.description, agents.description),
            image = COALESCE(EXCLUDED.image, agents.image),
            categories = COALESCE(EXCLUDED.categories, agents.categories),
            x402_support = EXCLUDED.x402_support,
            active = EXCLUDED.active,
            block_number = EXCLUDED.block_number,
            block_timestamp = COALESCE(EXCLUDED.block_timestamp, agents.block_timestamp),
            tx_hash = EXCLUDED.tx_hash,
            updated_at = NOW()
        "#,
    )
    .bind(agent.agent_id)
    .bind(agent.chain_id)
    .bind(&agent.owner)
    .bind(&agent.uri)
    .bind(&agent.metadata)
    .bind(&agent.name)
    .bind(&agent.description)
    .bind(&agent.image)
    .bind(&agent.categories)
    .bind(agent.x402_support)
    .bind(agent.active)
    .bind(agent.block_number)
    .bind(agent.block_timestamp)
    .bind(&agent.tx_hash)
    .execute(pool)
    .await?;

    Ok(())
}
