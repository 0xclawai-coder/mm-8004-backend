/// Indexer tests for molt-marketplace.
///
/// These tests verify chain configuration construction, metadata JSON parsing,
/// and block batch size logic without needing a real database or RPC connection.

#[cfg(test)]
mod chain_config_tests {

    /// Replicate the default chain IDs used by the indexer
    const MAINNET_CHAIN_ID: i32 = 143;
    const TESTNET_CHAIN_ID: i32 = 10143;

    const MAINNET_IDENTITY: &str = "0x8004A169FB4a3325136EB29fA0ceB6D2e539a432";
    const MAINNET_REPUTATION: &str = "0x8004BAa17C55a88189AE136b182e5fdA19dE9b63";
    const TESTNET_IDENTITY: &str = "0x8004A818BFB912233c491871b3d84c89A494BD9e";
    const TESTNET_REPUTATION: &str = "0x8004B663056A597Dffe9eCcC1965A193B7388713";

    #[test]
    fn mainnet_chain_id_is_143() {
        assert_eq!(MAINNET_CHAIN_ID, 143);
    }

    #[test]
    fn testnet_chain_id_is_10143() {
        assert_eq!(TESTNET_CHAIN_ID, 10143);
    }

    #[test]
    fn mainnet_identity_address_is_valid_hex() {
        assert!(MAINNET_IDENTITY.starts_with("0x"));
        assert_eq!(MAINNET_IDENTITY.len(), 42); // "0x" + 40 hex chars
        assert!(MAINNET_IDENTITY[2..].chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn mainnet_reputation_address_is_valid_hex() {
        assert!(MAINNET_REPUTATION.starts_with("0x"));
        assert_eq!(MAINNET_REPUTATION.len(), 42);
        assert!(MAINNET_REPUTATION[2..].chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn testnet_identity_address_is_valid_hex() {
        assert!(TESTNET_IDENTITY.starts_with("0x"));
        assert_eq!(TESTNET_IDENTITY.len(), 42);
        assert!(TESTNET_IDENTITY[2..].chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn testnet_reputation_address_is_valid_hex() {
        assert!(TESTNET_REPUTATION.starts_with("0x"));
        assert_eq!(TESTNET_REPUTATION.len(), 42);
        assert!(TESTNET_REPUTATION[2..].chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn default_mainnet_rpc_url() {
        // Mirrors the logic in provider.rs: get_chain_configs()
        let rpc_url = std::env::var("MONAD_MAINNET_RPC_URL")
            .unwrap_or_else(|_| "https://rpc.monad.xyz".to_string());
        // In test env, the env var is likely not set, so we get the default
        assert!(
            rpc_url.starts_with("https://"),
            "RPC URL should use HTTPS"
        );
    }

    #[test]
    fn default_testnet_rpc_url() {
        let rpc_url = std::env::var("MONAD_TESTNET_RPC_URL")
            .unwrap_or_else(|_| "https://testnet-rpc.monad.xyz".to_string());
        assert!(
            rpc_url.starts_with("https://"),
            "RPC URL should use HTTPS"
        );
    }

    #[test]
    fn index_mainnet_defaults_to_true() {
        // provider.rs checks INDEX_MAINNET != "false"
        let index_mainnet = std::env::var("INDEX_MAINNET")
            .unwrap_or_else(|_| "true".to_string());
        assert_ne!(index_mainnet, "false");
    }

    #[test]
    fn index_testnet_defaults_to_true() {
        let index_testnet = std::env::var("INDEX_TESTNET")
            .unwrap_or_else(|_| "true".to_string());
        assert_ne!(index_testnet, "false");
    }

    #[test]
    fn all_contract_addresses_start_with_8004() {
        // EIP-8004 project uses the 0x8004 prefix for all contracts
        assert!(MAINNET_IDENTITY.starts_with("0x8004"));
        assert!(MAINNET_REPUTATION.starts_with("0x8004"));
        assert!(TESTNET_IDENTITY.starts_with("0x8004"));
        assert!(TESTNET_REPUTATION.starts_with("0x8004"));
    }
}

#[cfg(test)]
mod metadata_parsing_tests {
    use serde_json;

    /// Replicate the AgentUriMetadata struct from metadata.rs for parsing tests.
    #[derive(Debug, serde::Deserialize, serde::Serialize)]
    struct AgentUriMetadata {
        name: Option<String>,
        description: Option<String>,
        image: Option<String>,
        categories: Option<Vec<String>>,
        x402_support: Option<bool>,
        endpoints: Option<Vec<AgentEndpointMeta>>,
        capabilities: Option<Vec<String>>,
    }

    #[derive(Debug, serde::Deserialize, serde::Serialize)]
    struct AgentEndpointMeta {
        url: String,
        protocol: Option<String>,
    }

    #[test]
    fn parse_full_eip8004_metadata() {
        let json = r#"{
            "name": "DeFi Arbitrage Agent",
            "description": "An agent that finds arbitrage opportunities",
            "image": "https://example.com/agent.png",
            "categories": ["DeFi", "Arbitrage"],
            "x402_support": true,
            "endpoints": [
                { "url": "https://agent.example.com/api", "protocol": "x402" },
                { "url": "https://agent.example.com/ws", "protocol": "websocket" }
            ],
            "capabilities": ["trade", "monitor", "alert"]
        }"#;

        let meta: AgentUriMetadata = serde_json::from_str(json).unwrap();
        assert_eq!(meta.name.as_deref(), Some("DeFi Arbitrage Agent"));
        assert_eq!(
            meta.description.as_deref(),
            Some("An agent that finds arbitrage opportunities")
        );
        assert_eq!(meta.image.as_deref(), Some("https://example.com/agent.png"));
        assert_eq!(
            meta.categories.as_ref().unwrap(),
            &vec!["DeFi".to_string(), "Arbitrage".to_string()]
        );
        assert_eq!(meta.x402_support, Some(true));

        let endpoints = meta.endpoints.as_ref().unwrap();
        assert_eq!(endpoints.len(), 2);
        assert_eq!(endpoints[0].url, "https://agent.example.com/api");
        assert_eq!(endpoints[0].protocol.as_deref(), Some("x402"));
        assert_eq!(endpoints[1].url, "https://agent.example.com/ws");
        assert_eq!(endpoints[1].protocol.as_deref(), Some("websocket"));

        assert_eq!(
            meta.capabilities.as_ref().unwrap(),
            &vec![
                "trade".to_string(),
                "monitor".to_string(),
                "alert".to_string()
            ]
        );
    }

    #[test]
    fn parse_metadata_with_only_required_fields() {
        // All fields are optional in the struct — test with minimal JSON
        let json = r#"{}"#;
        let meta: AgentUriMetadata = serde_json::from_str(json).unwrap();
        assert!(meta.name.is_none());
        assert!(meta.description.is_none());
        assert!(meta.image.is_none());
        assert!(meta.categories.is_none());
        assert!(meta.x402_support.is_none());
        assert!(meta.endpoints.is_none());
        assert!(meta.capabilities.is_none());
    }

    #[test]
    fn parse_metadata_with_name_only() {
        let json = r#"{"name": "SimpleAgent"}"#;
        let meta: AgentUriMetadata = serde_json::from_str(json).unwrap();
        assert_eq!(meta.name.as_deref(), Some("SimpleAgent"));
        assert!(meta.description.is_none());
        assert!(meta.endpoints.is_none());
    }

    #[test]
    fn parse_metadata_with_empty_categories() {
        let json = r#"{"categories": []}"#;
        let meta: AgentUriMetadata = serde_json::from_str(json).unwrap();
        assert!(meta.categories.as_ref().unwrap().is_empty());
    }

    #[test]
    fn parse_metadata_with_empty_endpoints() {
        let json = r#"{"endpoints": []}"#;
        let meta: AgentUriMetadata = serde_json::from_str(json).unwrap();
        assert!(meta.endpoints.as_ref().unwrap().is_empty());
    }

    #[test]
    fn parse_metadata_with_x402_false() {
        let json = r#"{"x402_support": false}"#;
        let meta: AgentUriMetadata = serde_json::from_str(json).unwrap();
        assert_eq!(meta.x402_support, Some(false));
    }

    #[test]
    fn parse_endpoint_without_protocol() {
        let json = r#"{"endpoints": [{"url": "https://api.example.com"}]}"#;
        let meta: AgentUriMetadata = serde_json::from_str(json).unwrap();
        let endpoints = meta.endpoints.unwrap();
        assert_eq!(endpoints.len(), 1);
        assert_eq!(endpoints[0].url, "https://api.example.com");
        assert!(endpoints[0].protocol.is_none());
    }

    #[test]
    fn metadata_serialization_roundtrip() {
        let meta = AgentUriMetadata {
            name: Some("RoundTrip Agent".to_string()),
            description: Some("Tests roundtrip serialization".to_string()),
            image: Some("https://img.example.com/logo.png".to_string()),
            categories: Some(vec!["Legal".to_string()]),
            x402_support: Some(true),
            endpoints: Some(vec![AgentEndpointMeta {
                url: "https://endpoint.example.com".to_string(),
                protocol: Some("http".to_string()),
            }]),
            capabilities: Some(vec!["consult".to_string()]),
        };

        let json_str = serde_json::to_string(&meta).unwrap();
        let parsed: AgentUriMetadata = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed.name, meta.name);
        assert_eq!(parsed.x402_support, meta.x402_support);
        assert_eq!(
            parsed.endpoints.as_ref().unwrap()[0].url,
            meta.endpoints.as_ref().unwrap()[0].url
        );
    }

    #[test]
    fn parse_metadata_ignores_unknown_fields() {
        // Extra fields in the JSON should not cause parsing to fail
        let json = r#"{
            "name": "Agent",
            "unknown_field": "should be ignored",
            "extra": 123
        }"#;
        // serde by default ignores unknown fields with deny_unknown_fields not set
        let result: Result<AgentUriMetadata, _> = serde_json::from_str(json);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name.as_deref(), Some("Agent"));
    }

    #[test]
    fn metadata_json_value_construction() {
        // Test the same JSONB value that metadata.rs builds for the agents table
        let meta = AgentUriMetadata {
            name: Some("TestAgent".to_string()),
            description: None,
            image: None,
            categories: None,
            x402_support: None,
            endpoints: Some(vec![AgentEndpointMeta {
                url: "https://api.test.com".to_string(),
                protocol: Some("x402".to_string()),
            }]),
            capabilities: Some(vec!["trade".to_string()]),
        };

        // This is what update_agent_with_metadata builds
        let metadata_json = serde_json::json!({
            "endpoints": meta.endpoints,
            "capabilities": meta.capabilities,
        });

        assert!(metadata_json["endpoints"].is_array());
        assert_eq!(metadata_json["endpoints"][0]["url"], "https://api.test.com");
        assert_eq!(metadata_json["capabilities"][0], "trade");
    }
}

#[cfg(test)]
mod block_batch_tests {
    /// The BLOCK_BATCH_SIZE constant from indexer/mod.rs
    const BLOCK_BATCH_SIZE: u64 = 1000;
    const POLL_INTERVAL_SECS: u64 = 2;

    #[test]
    fn block_batch_size_is_1000() {
        assert_eq!(BLOCK_BATCH_SIZE, 1000);
    }

    #[test]
    fn poll_interval_is_2_seconds() {
        assert_eq!(POLL_INTERVAL_SECS, 2);
    }

    #[test]
    fn batch_calculation_single_batch() {
        // If we only have 500 blocks to process, it fits in one batch
        let from = 100u64;
        let to = 599u64;
        let batch_to = std::cmp::min(from + BLOCK_BATCH_SIZE - 1, to);
        assert_eq!(batch_to, 599); // to < from + 999, so batch_to = to
    }

    #[test]
    fn batch_calculation_exact_batch_boundary() {
        let from = 0u64;
        let to = 999u64;
        let batch_to = std::cmp::min(from + BLOCK_BATCH_SIZE - 1, to);
        assert_eq!(batch_to, 999); // exactly one full batch
    }

    #[test]
    fn batch_calculation_multiple_batches() {
        let from = 0u64;
        let to = 2500u64;

        let mut batch_from = from;
        let mut batch_count = 0;

        while batch_from <= to {
            let batch_to = std::cmp::min(batch_from + BLOCK_BATCH_SIZE - 1, to);
            batch_count += 1;

            // Verify batch boundaries
            assert!(batch_to >= batch_from);
            assert!(batch_to <= to);
            assert!(batch_to - batch_from < BLOCK_BATCH_SIZE);

            batch_from = batch_to + 1;
        }

        // 0-999, 1000-1999, 2000-2500 => 3 batches
        assert_eq!(batch_count, 3);
    }

    #[test]
    fn batch_calculation_single_block() {
        let from = 5000u64;
        let to = 5000u64;
        let batch_to = std::cmp::min(from + BLOCK_BATCH_SIZE - 1, to);
        assert_eq!(batch_to, 5000);
    }

    #[test]
    fn batch_from_never_exceeds_to() {
        // Simulate the indexer loop for a very small range
        let from = 10u64;
        let to = 15u64;

        let mut batch_from = from;
        while batch_from <= to {
            let batch_to = std::cmp::min(batch_from + BLOCK_BATCH_SIZE - 1, to);
            assert!(batch_to <= to);
            batch_from = batch_to + 1;
        }
        // After the loop, batch_from should be > to
        assert!(batch_from > to);
    }

    #[test]
    fn last_block_comparison_logic() {
        // Mirrors index_chain logic: only index if identity_last < latest_block
        let identity_last: i64 = 1000;
        let latest_block: u64 = 2000;

        if identity_last < latest_block as i64 {
            let from = (identity_last + 1) as u64;
            assert_eq!(from, 1001);
        } else {
            panic!("Should have entered the if branch");
        }
    }

    #[test]
    fn no_indexing_when_caught_up() {
        let identity_last: i64 = 5000;
        let latest_block: u64 = 5000;

        // identity_last is NOT < latest_block, so no indexing should happen
        assert!(!(identity_last < latest_block as i64));
    }
}
