use sqlx::PgPool;

use crate::types::{AgentDetailResponse, AgentListItem, NewAgent};

/// Get a paginated list of agents with optional filtering, search, and sorting.
/// LEFT JOINs feedbacks to compute average reputation score and feedback count.
pub async fn get_agents(
    pool: &PgPool,
    chain_id: Option<i32>,
    search: Option<&str>,
    category: Option<&str>,
    sort: &str,
    offset: i64,
    limit: i64,
) -> Result<(Vec<AgentListItem>, i64), sqlx::Error> {
    // Build the ORDER BY clause based on sort parameter
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
            AND ($3::TEXT IS NULL OR $3 = ANY(a.categories))
        GROUP BY a.id
        ORDER BY {}
        LIMIT $4 OFFSET $5
        "#,
        order_clause
    );

    let agents: Vec<AgentListItem> = sqlx::query_as(&base_query)
        .bind(chain_id)
        .bind(search)
        .bind(category)
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
            AND ($3::TEXT IS NULL OR $3 = ANY(a.categories))
        "#,
    )
    .bind(chain_id)
    .bind(search)
    .bind(category)
    .fetch_one(pool)
    .await?;

    Ok((agents, total.0))
}

/// Get a single agent by agent_id and chain_id, with reputation data.
pub async fn get_agent_by_id(
    pool: &PgPool,
    agent_id: i64,
    chain_id: i32,
) -> Result<Option<AgentDetailResponse>, sqlx::Error> {
    let row: Option<AgentDetailResponse> = sqlx::query_as(
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
