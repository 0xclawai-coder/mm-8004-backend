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

/// Resolve an agent URI and parse the response as EIP-8004 metadata JSON.
///
/// Supports:
/// - `data:application/json;base64,<base64>` — inline base64 JSON
/// - `data:application/json,<json>` — inline raw JSON (URL-encoded)
/// - `ipfs://<cid>` — resolved via public IPFS gateway
/// - `http(s)://...` — standard HTTP fetch
async fn fetch_metadata(uri: &str) -> Result<AgentUriMetadata, Box<dyn std::error::Error + Send + Sync>> {
    use base64::Engine as _;

    // Handle data: URIs
    if uri.starts_with("data:") {
        let rest = uri.strip_prefix("data:").unwrap();

        // data:application/json;base64,<payload>
        if let Some(payload) = rest
            .strip_prefix("application/json;base64,")
            .or_else(|| rest.strip_prefix("application/json; base64,"))
        {
            let decoded = base64::engine::general_purpose::STANDARD.decode(payload.trim())?;
            let meta: AgentUriMetadata = serde_json::from_slice(&decoded)?;
            return Ok(meta);
        }

        // data:application/json,<payload> (URL-encoded or raw)
        if let Some(payload) = rest.strip_prefix("application/json,") {
            let decoded_str = urlencoding::decode(payload)?;
            let meta: AgentUriMetadata = serde_json::from_str(&decoded_str)?;
            return Ok(meta);
        }

        return Err(format!("Unsupported data URI format: {}", &uri[..uri.len().min(80)]).into());
    }

    // Handle ipfs:// URIs — resolve via public gateway
    let fetch_url = if let Some(cid_path) = uri.strip_prefix("ipfs://") {
        format!("https://ipfs.io/ipfs/{}", cid_path)
    } else {
        uri.to_string()
    };

    // Standard HTTP fetch
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;

    let response = client.get(&fetch_url).send().await?;

    if !response.status().is_success() {
        return Err(format!("HTTP {} from URI: {}", response.status(), fetch_url).into());
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
