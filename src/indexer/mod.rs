pub mod backfill;
pub mod identity;
pub mod marketplace;
pub mod metadata;
pub mod provider;
pub mod reputation;

use provider::ChainConfig;
use sqlx::PgPool;

/// Block batch size per eth_getLogs call to avoid RPC limits.
/// Both mainnet and testnet RPCs are limited to 100 block range.
pub const BLOCK_BATCH_SIZE: u64 = 100;

/// Number of parallel batches to run concurrently per contract.
/// Total blocks per cycle = BLOCK_BATCH_SIZE * PARALLEL_BATCHES (e.g., 100 * 10 = 1000).
pub const PARALLEL_BATCHES: u64 = 10;

/// Poll interval between indexer cycles (in seconds).
/// Only applies when fully caught up to latest block.
pub const POLL_INTERVAL_SECS: u64 = 2;

/// Run the indexer loop for all configured chains.
/// This function runs forever, polling for new events every POLL_INTERVAL_SECS.
pub async fn run_indexer(pool: PgPool) {
    let chains = provider::get_chain_configs();

    if chains.is_empty() {
        tracing::warn!("No chain configs found — indexer has nothing to index");
        return;
    }

    tracing::info!(
        "Starting indexer for {} chain(s) | batch_size={} | parallel={} | poll_interval={}s",
        chains.len(),
        BLOCK_BATCH_SIZE,
        PARALLEL_BATCHES,
        POLL_INTERVAL_SECS
    );
    for chain in &chains {
        let masked_rpc = if chain.rpc_url.len() > 20 {
            format!("{}...{}", &chain.rpc_url[..16], &chain.rpc_url[chain.rpc_url.len()-8..])
        } else {
            chain.rpc_url.clone()
        };
        tracing::info!(
            "  Chain {} | rpc={} | start_block={} | identity={} | reputation={} | marketplace={}",
            chain.chain_id,
            masked_rpc,
            chain.start_block,
            chain.identity_address,
            chain.reputation_address,
            chain.marketplace_address.map(|a| a.to_string()).unwrap_or_else(|| "none".to_string())
        );
    }

    // Sync marketplace config from on-chain at startup (initialize() doesn't emit events)
    for chain in &chains {
        if chain.marketplace_address.is_some() {
            match provider::create_provider(chain) {
                Ok(prov) => {
                    if let Err(e) = marketplace::sync_marketplace_config(&pool, &prov, chain).await {
                        tracing::error!(chain_id = chain.chain_id, "Failed to sync marketplace config: {:?}", e);
                    }
                }
                Err(e) => {
                    tracing::error!(chain_id = chain.chain_id, "Failed to create provider for config sync: {:?}", e);
                }
            }
        }
    }

    loop {
        let mut all_caught_up = true;
        for chain in &chains {
            match index_chain(&pool, chain).await {
                Ok(caught_up) => {
                    if !caught_up {
                        all_caught_up = false;
                    }
                }
                Err(e) => {
                    tracing::error!(
                        chain_id = chain.chain_id,
                        "Indexer error for chain {}: {:?}",
                        chain.chain_id,
                        e
                    );
                }
            }
        }

        // Only sleep when fully caught up to latest block
        if all_caught_up {
            tokio::time::sleep(std::time::Duration::from_secs(POLL_INTERVAL_SECS)).await;
        }
    }
}

/// Index a single cycle for a chain: run PARALLEL_BATCHES concurrent batches for identity + reputation.
/// Returns Ok(true) if caught up to latest block, Ok(false) if still behind.
async fn index_chain(pool: &PgPool, chain: &ChainConfig) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let provider = provider::create_provider(chain)?;
    let batch_size = BLOCK_BATCH_SIZE;

    // Get the latest block number from the RPC
    let latest_block = provider::get_latest_block(&provider).await?;

    let identity_addr = chain.identity_address.to_string();
    let reputation_addr = chain.reputation_address.to_string();
    let marketplace_addr = chain.marketplace_address.map(|a| a.to_string());

    let identity_last = crate::db::indexer_state::get_last_block(pool, chain.chain_id, &identity_addr)
        .await?
        .unwrap_or(chain.start_block as i64 - 1);
    let reputation_last = crate::db::indexer_state::get_last_block(pool, chain.chain_id, &reputation_addr)
        .await?
        .unwrap_or(chain.start_block as i64 - 1);
    let marketplace_last = if let Some(ref addr) = marketplace_addr {
        let mp_start = chain.marketplace_start_block.unwrap_or(chain.start_block);
        crate::db::indexer_state::get_last_block(pool, chain.chain_id, addr)
            .await?
            .unwrap_or(mp_start as i64 - 1)
    } else {
        latest_block as i64 // treated as caught up
    };

    let identity_behind = identity_last < latest_block as i64;
    let reputation_behind = reputation_last < latest_block as i64;
    let marketplace_behind = marketplace_last < latest_block as i64;

    if !identity_behind && !reputation_behind && !marketplace_behind {
        return Ok(true);
    }

    // Run identity, reputation, and marketplace indexing in parallel
    let (identity_result, reputation_result, marketplace_result) = tokio::join!(
        index_contract_parallel(
            pool,
            &provider,
            chain,
            identity_last,
            latest_block,
            batch_size,
            ContractType::Identity,
        ),
        index_contract_parallel(
            pool,
            &provider,
            chain,
            reputation_last,
            latest_block,
            batch_size,
            ContractType::Reputation,
        ),
        index_contract_parallel(
            pool,
            &provider,
            chain,
            marketplace_last,
            latest_block,
            batch_size,
            ContractType::Marketplace,
        ),
    );

    // Update indexer state for identity
    if let Ok(Some(last)) = identity_result {
        crate::db::indexer_state::update_last_block_with_name(
            pool, chain.chain_id, &identity_addr, last, Some("IdentityRegistry"),
        ).await?;
    } else if let Err(e) = identity_result {
        tracing::error!(chain_id = chain.chain_id, "Identity indexing error: {:?}", e);
    }

    // Update indexer state for reputation
    if let Ok(Some(last)) = reputation_result {
        crate::db::indexer_state::update_last_block_with_name(
            pool, chain.chain_id, &reputation_addr, last, Some("ReputationRegistry"),
        ).await?;
    } else if let Err(e) = reputation_result {
        tracing::error!(chain_id = chain.chain_id, "Reputation indexing error: {:?}", e);
    }

    // Update indexer state for marketplace
    if let Some(ref addr) = marketplace_addr {
        if let Ok(Some(last)) = marketplace_result {
            crate::db::indexer_state::update_last_block_with_name(
                pool, chain.chain_id, addr, last, Some("MoltMarketplace"),
            ).await?;
        } else if let Err(e) = marketplace_result {
            tracing::error!(chain_id = chain.chain_id, "Marketplace indexing error: {:?}", e);
        }
    }

    Ok(false)
}

#[derive(Clone, Copy)]
enum ContractType {
    Identity,
    Reputation,
    Marketplace,
}

/// Run up to PARALLEL_BATCHES concurrent batch indexing tasks for a single contract.
/// Returns the highest successfully completed block number, or None if already caught up.
async fn index_contract_parallel(
    pool: &PgPool,
    _provider: &provider::HttpProvider,
    chain: &ChainConfig,
    last_block: i64,
    latest_block: u64,
    batch_size: u64,
    contract_type: ContractType,
) -> Result<Option<i64>, Box<dyn std::error::Error + Send + Sync>> {
    if last_block >= latest_block as i64 {
        return Ok(None);
    }

    // Skip marketplace if no address configured
    if matches!(contract_type, ContractType::Marketplace) && chain.marketplace_address.is_none() {
        return Ok(None);
    }

    let contract_name = match contract_type {
        ContractType::Identity => "identity",
        ContractType::Reputation => "reputation",
        ContractType::Marketplace => "marketplace",
    };

    // Build batch ranges
    let mut batches: Vec<(u64, u64)> = Vec::new();
    let mut cursor = (last_block + 1) as u64;
    for _ in 0..PARALLEL_BATCHES {
        if cursor > latest_block {
            break;
        }
        let from = cursor;
        let to = std::cmp::min(from + batch_size - 1, latest_block);
        batches.push((from, to));
        cursor = to + 1;
    }

    if batches.is_empty() {
        return Ok(None);
    }

    // If only 1 batch (near tip), run directly without spawning tasks
    if batches.len() == 1 {
        let (from, to) = batches[0];
        tracing::info!(
            chain_id = chain.chain_id,
            "Indexing {} events blocks {} - {}",
            contract_name, from, to
        );
        let prov = provider::create_provider(chain)?;
        match contract_type {
            ContractType::Identity => {
                identity::index_identity_events(pool, &prov, chain, from, to).await?;
            }
            ContractType::Reputation => {
                reputation::index_reputation_events(pool, &prov, chain, from, to).await?;
            }
            ContractType::Marketplace => {
                marketplace::index_marketplace_events(pool, &prov, chain, from, to).await?;
            }
        }
        return Ok(Some(to as i64));
    }

    let total_from = batches.first().unwrap().0;
    let total_to = batches.last().unwrap().1;
    tracing::info!(
        chain_id = chain.chain_id,
        "Indexing {} events blocks {} - {} ({} parallel batches)",
        contract_name, total_from, total_to, batches.len()
    );

    // Run all batches concurrently
    let mut handles = Vec::new();
    for (from, to) in &batches {
        let pool = pool.clone();
        let chain = chain.clone();
        let from = *from;
        let to = *to;
        // contract_type is Copy, no need to rebind
        // We need to create a new provider per task since HttpProvider is not Send-safe across awaits
        // in different tasks. Instead, share the provider ref — alloy providers are Send+Sync.
        let prov = provider::create_provider(&chain)?;
        handles.push(tokio::spawn(async move {
            match contract_type {
                ContractType::Identity => {
                    identity::index_identity_events(&pool, &prov, &chain, from, to).await
                }
                ContractType::Reputation => {
                    reputation::index_reputation_events(&pool, &prov, &chain, from, to).await
                }
                ContractType::Marketplace => {
                    marketplace::index_marketplace_events(&pool, &prov, &chain, from, to).await
                }
            }
        }));
    }

    // Wait for all and find the highest completed block
    let mut highest_completed: Option<i64> = None;
    let mut all_ok = true;
    for (i, handle) in handles.into_iter().enumerate() {
        match handle.await {
            Ok(Ok(())) => {
                let (_, to) = batches[i];
                highest_completed = Some(to as i64);
            }
            Ok(Err(e)) => {
                let (from, to) = batches[i];
                tracing::error!(
                    chain_id = chain.chain_id,
                    "Batch {} {}-{} failed: {:?}", contract_name, from, to, e
                );
                all_ok = false;
                break; // Stop at first failure to keep sequential consistency
            }
            Err(e) => {
                let (from, to) = batches[i];
                tracing::error!(
                    chain_id = chain.chain_id,
                    "Batch {} {}-{} panicked: {:?}", contract_name, from, to, e
                );
                all_ok = false;
                break;
            }
        }
    }

    // Only update to highest block if all preceding batches succeeded
    if !all_ok {
        // Find last successful sequential block
        // highest_completed already has the last successful one before the break
    }

    Ok(highest_completed)
}
