use std::collections::HashMap;

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::provider::{self, ChainConfig};

/// Backfill block_timestamp for all rows that have a block_number but NULL block_timestamp.
/// Groups by (chain_id, block_number), fetches timestamp from RPC, then batch-updates all 4 tables.
/// Runs once on startup before the indexer loop.
pub async fn backfill_block_timestamps(pool: &PgPool) {
    let chains = provider::get_chain_configs();
    if chains.is_empty() {
        return;
    }

    // Build a map of chain_id -> ChainConfig for RPC access
    let chain_map: HashMap<i32, &ChainConfig> = chains.iter().map(|c| (c.chain_id, c)).collect();

    // Query all distinct (chain_id, block_number) pairs that need backfill
    let rows: Vec<(i32, i64)> = match sqlx::query_as(
        r#"
        SELECT DISTINCT chain_id, block_number FROM (
            SELECT chain_id, block_number FROM agents WHERE block_number IS NOT NULL AND block_timestamp IS NULL
            UNION
            SELECT chain_id, block_number FROM feedbacks WHERE block_timestamp IS NULL
            UNION
            SELECT chain_id, block_number FROM feedback_responses WHERE block_timestamp IS NULL
            UNION
            SELECT chain_id, block_number FROM activity_log WHERE block_timestamp IS NULL
        ) AS t
        ORDER BY chain_id, block_number
        "#,
    )
    .fetch_all(pool)
    .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Failed to query blocks needing timestamp backfill: {:?}", e);
            return;
        }
    };

    if rows.is_empty() {
        tracing::info!("Block timestamp backfill: nothing to do");
        return;
    }

    tracing::info!("Block timestamp backfill: {} unique blocks to fill", rows.len());

    // Cache providers per chain
    let mut providers: HashMap<i32, super::provider::HttpProvider> = HashMap::new();
    let mut filled = 0u64;
    let mut failed = 0u64;

    for (chain_id, block_number) in &rows {
        // Get or create provider for this chain
        let prov = if let Some(p) = providers.get(chain_id) {
            p
        } else {
            let config = match chain_map.get(chain_id) {
                Some(c) => c,
                None => {
                    tracing::warn!("No chain config for chain_id={}, skipping", chain_id);
                    continue;
                }
            };
            match provider::create_provider(config) {
                Ok(p) => {
                    providers.insert(*chain_id, p);
                    providers.get(chain_id).unwrap()
                }
                Err(e) => {
                    tracing::error!("Failed to create provider for chain {}: {:?}", chain_id, e);
                    continue;
                }
            }
        };

        // Fetch block timestamp from RPC
        let ts = match provider::get_block_timestamp(prov, *block_number as u64).await {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!(
                    "Failed to fetch timestamp for chain={} block={}: {:?}",
                    chain_id, block_number, e
                );
                failed += 1;
                continue;
            }
        };

        // Update all 4 tables
        if let Err(e) = update_timestamps(pool, *chain_id, *block_number, ts).await {
            tracing::warn!(
                "Failed to update timestamps for chain={} block={}: {:?}",
                chain_id, block_number, e
            );
            failed += 1;
            continue;
        }

        filled += 1;
        if filled.is_multiple_of(50) {
            tracing::info!("Block timestamp backfill progress: {}/{}", filled, rows.len());
        }
    }

    tracing::info!(
        "Block timestamp backfill complete: {} filled, {} failed",
        filled, failed
    );
}

/// Update block_timestamp for all rows matching (chain_id, block_number) across all tables.
async fn update_timestamps(
    pool: &PgPool,
    chain_id: i32,
    block_number: i64,
    ts: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE agents SET block_timestamp = $1 WHERE chain_id = $2 AND block_number = $3 AND block_timestamp IS NULL",
    )
    .bind(ts)
    .bind(chain_id)
    .bind(block_number)
    .execute(pool)
    .await?;

    sqlx::query(
        "UPDATE feedbacks SET block_timestamp = $1 WHERE chain_id = $2 AND block_number = $3 AND block_timestamp IS NULL",
    )
    .bind(ts)
    .bind(chain_id)
    .bind(block_number)
    .execute(pool)
    .await?;

    sqlx::query(
        "UPDATE feedback_responses SET block_timestamp = $1 WHERE chain_id = $2 AND block_number = $3 AND block_timestamp IS NULL",
    )
    .bind(ts)
    .bind(chain_id)
    .bind(block_number)
    .execute(pool)
    .await?;

    sqlx::query(
        "UPDATE activity_log SET block_timestamp = $1 WHERE chain_id = $2 AND block_number = $3 AND block_timestamp IS NULL",
    )
    .bind(ts)
    .bind(chain_id)
    .bind(block_number)
    .execute(pool)
    .await?;

    Ok(())
}
