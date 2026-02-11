use alloy::eips::BlockNumberOrTag;
use alloy::primitives::Address;
use alloy::providers::{Provider, ProviderBuilder};
use chrono::{DateTime, Utc};

/// Configuration for a single chain to index.
#[derive(Debug, Clone)]
pub struct ChainConfig {
    pub chain_id: i32,
    pub rpc_url: String,
    pub identity_address: Address,
    pub reputation_address: Address,
    /// Block number where the contracts were deployed. Indexer starts from here.
    pub start_block: u64,
}

/// The alloy HTTP provider type returned by ProviderBuilder::new().connect_http().
/// In alloy v1, this is a FillProvider wrapping a RootProvider with default fillers.
pub type HttpProvider = alloy::providers::fillers::FillProvider<
    alloy::providers::fillers::JoinFill<
        alloy::providers::Identity,
        alloy::providers::fillers::JoinFill<
            alloy::providers::fillers::GasFiller,
            alloy::providers::fillers::JoinFill<
                alloy::providers::fillers::BlobGasFiller,
                alloy::providers::fillers::JoinFill<
                    alloy::providers::fillers::NonceFiller,
                    alloy::providers::fillers::ChainIdFiller,
                >,
            >,
        >,
    >,
    alloy::providers::RootProvider,
>;

/// Create an alloy HTTP provider for the given chain config.
pub fn create_provider(config: &ChainConfig) -> Result<HttpProvider, Box<dyn std::error::Error + Send + Sync>> {
    let url = config.rpc_url.parse()?;
    let provider = ProviderBuilder::new().connect_http(url);
    Ok(provider)
}

/// Get the latest block number from the RPC provider.
pub async fn get_latest_block(provider: &HttpProvider) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
    let block_number = provider.get_block_number().await?;
    Ok(block_number)
}

/// Get the timestamp of a specific block from the RPC provider.
/// Returns the block's timestamp as a `DateTime<Utc>`.
pub async fn get_block_timestamp(
    provider: &HttpProvider,
    block_number: u64,
) -> Result<DateTime<Utc>, Box<dyn std::error::Error + Send + Sync>> {
    let block = provider
        .get_block_by_number(BlockNumberOrTag::Number(block_number))
        .await?
        .ok_or_else(|| format!("Block {} not found", block_number))?;
    let ts = block.header.timestamp;
    let dt = DateTime::<Utc>::from_timestamp(ts as i64, 0)
        .unwrap_or_default();
    Ok(dt)
}

/// Build all chain configs from environment variables with fallback defaults.
///
/// Environment variables:
/// - MONAD_MAINNET_RPC_URL (default: https://rpc.monad.xyz)
/// - MONAD_TESTNET_RPC_URL (default: https://testnet-rpc.monad.xyz)
/// - INDEX_MAINNET (default: "true") — set to "false" to skip mainnet
/// - INDEX_TESTNET (default: "true") — set to "false" to skip testnet
pub fn get_chain_configs() -> Vec<ChainConfig> {
    let mut configs = Vec::new();

    let index_mainnet = std::env::var("INDEX_MAINNET")
        .unwrap_or_else(|_| "true".to_string());
    let index_testnet = std::env::var("INDEX_TESTNET")
        .unwrap_or_else(|_| "true".to_string());

    if index_mainnet != "false" {
        let rpc_url = std::env::var("MONAD_MAINNET_RPC")
            .unwrap_or_else(|_| "https://rpc.monad.xyz".to_string());

        configs.push(ChainConfig {
            chain_id: 143,
            rpc_url,
            identity_address: "0x8004A169FB4a3325136EB29fA0ceB6D2e539a432"
                .parse::<Address>()
                .expect("Invalid mainnet identity address"),
            reputation_address: "0x8004BAa17C55a88189AE136b182e5fdA19dE9b63"
                .parse::<Address>()
                .expect("Invalid mainnet reputation address"),
            start_block: 52_952_790,
        });
    }

    if index_testnet != "false" {
        let rpc_url = std::env::var("MONAD_TESTNET_RPC")
            .unwrap_or_else(|_| "https://testnet-rpc.monad.xyz".to_string());

        configs.push(ChainConfig {
            chain_id: 10143,
            rpc_url,
            identity_address: "0x8004A818BFB912233c491871b3d84c89A494BD9e"
                .parse::<Address>()
                .expect("Invalid testnet identity address"),
            reputation_address: "0x8004B663056A597Dffe9eCcC1965A193B7388713"
                .parse::<Address>()
                .expect("Invalid testnet reputation address"),
            start_block: 10_391_697,
        });
    }

    configs
}
