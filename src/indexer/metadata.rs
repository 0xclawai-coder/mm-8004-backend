use serde::{Deserialize, Serialize};
use sqlx::PgPool;

/// EIP-8004 agent metadata schema returned from the agent URI.
#[derive(Debug, Deserialize, Serialize)]
pub struct AgentUriMetadata {
    pub name: Option<String>,
    pub description: Option<String>,
    pub image: Option<String>,
    pub categories: Option<Vec<String>>,
    pub x402_support: Option<bool>,
    pub endpoints: Option<Vec<AgentEndpointMeta>>,
    pub capabilities: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AgentEndpointMeta {
    pub url: String,
    pub protocol: Option<String>,
}

/// Fetch metadata from an agent URI and update the agents table.
///
/// This function is designed to be called from a spawned tokio task.
/// It logs errors internally and never panics — failures are gracefully skipped.
pub async fn fetch_and_update_metadata(pool: &PgPool, agent_id: i64, chain_id: i32, uri: &str) {
    tracing::info!(
        agent_id = agent_id,
        chain_id = chain_id,
        "Fetching metadata from URI: {}",
        uri
    );

    match fetch_metadata(uri).await {
        Ok(meta) => {
            if let Err(e) = update_agent_with_metadata(pool, agent_id, chain_id, &meta).await {
                tracing::error!(
                    agent_id = agent_id,
                    chain_id = chain_id,
                    "Failed to update agent with metadata: {:?}",
                    e
                );
            } else {
                tracing::info!(
                    agent_id = agent_id,
                    chain_id = chain_id,
                    "Successfully updated agent metadata (name={:?})",
                    meta.name
                );
            }
        }
        Err(e) => {
            tracing::error!(
                agent_id = agent_id,
                chain_id = chain_id,
                uri = uri,
                "Failed to fetch metadata from URI: {:?}",
                e
            );
        }
    }
}

/// HTTP GET the agent URI and parse the response as EIP-8004 metadata JSON.
async fn fetch_metadata(uri: &str) -> Result<AgentUriMetadata, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let response = client.get(uri).send().await?;

    if !response.status().is_success() {
        return Err(format!("HTTP {} from URI: {}", response.status(), uri).into());
    }

    let meta: AgentUriMetadata = response.json().await?;
    Ok(meta)
}

/// Update the agents table with parsed metadata fields.
async fn update_agent_with_metadata(
    pool: &PgPool,
    agent_id: i64,
    chain_id: i32,
    meta: &AgentUriMetadata,
) -> Result<(), sqlx::Error> {
    // Build the full metadata JSONB from endpoints and capabilities
    let metadata_json = serde_json::json!({
        "endpoints": meta.endpoints,
        "capabilities": meta.capabilities,
    });

    sqlx::query(
        r#"
        UPDATE agents
        SET
            name = COALESCE($3, name),
            description = COALESCE($4, description),
            image = COALESCE($5, image),
            categories = COALESCE($6, categories),
            x402_support = COALESCE($7, x402_support),
            metadata = COALESCE($8, metadata),
            updated_at = NOW()
        WHERE agent_id = $1 AND chain_id = $2
        "#,
    )
    .bind(agent_id)
    .bind(chain_id)
    .bind(&meta.name)
    .bind(&meta.description)
    .bind(&meta.image)
    .bind(&meta.categories)
    .bind(meta.x402_support)
    .bind(&metadata_json)
    .execute(pool)
    .await?;

    Ok(())
}
