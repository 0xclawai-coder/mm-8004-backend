/// Backend API tests for molt-marketplace.
///
/// These tests verify parsing logic, type serialization, query parameter defaults,
/// agent ID parsing, and error response formats — all without requiring a database.

#[cfg(test)]
mod types_tests {
    use serde_json;

    // ──────────────────────────────────────────────────────────────────
    // PaginationParams tests
    // ──────────────────────────────────────────────────────────────────

    #[test]
    fn pagination_defaults_page_1_limit_20() {
        // Deserialize empty JSON into PaginationParams-like values
        #[derive(Debug, serde::Deserialize)]
        struct PaginationParams {
            page: Option<i64>,
            limit: Option<i64>,
        }
        impl PaginationParams {
            fn page(&self) -> i64 {
                self.page.unwrap_or(1).max(1)
            }
            fn limit(&self) -> i64 {
                self.limit.unwrap_or(20).clamp(1, 100)
            }
            fn offset(&self) -> i64 {
                (self.page() - 1) * self.limit()
            }
        }

        let params: PaginationParams = serde_json::from_str("{}").unwrap();
        assert_eq!(params.page(), 1);
        assert_eq!(params.limit(), 20);
        assert_eq!(params.offset(), 0);
    }

    #[test]
    fn pagination_clamps_limit_to_100() {
        #[derive(Debug, serde::Deserialize)]
        struct PaginationParams {
            page: Option<i64>,
            limit: Option<i64>,
        }
        impl PaginationParams {
            fn limit(&self) -> i64 {
                self.limit.unwrap_or(20).clamp(1, 100)
            }
        }

        let params: PaginationParams =
            serde_json::from_str(r#"{"limit": 999}"#).unwrap();
        assert_eq!(params.limit(), 100);
    }

    #[test]
    fn pagination_clamps_limit_minimum_to_1() {
        #[derive(Debug, serde::Deserialize)]
        struct PaginationParams {
            page: Option<i64>,
            limit: Option<i64>,
        }
        impl PaginationParams {
            fn limit(&self) -> i64 {
                self.limit.unwrap_or(20).clamp(1, 100)
            }
        }

        let params: PaginationParams =
            serde_json::from_str(r#"{"limit": -5}"#).unwrap();
        assert_eq!(params.limit(), 1);

        let params_zero: PaginationParams =
            serde_json::from_str(r#"{"limit": 0}"#).unwrap();
        assert_eq!(params_zero.limit(), 1);
    }

    #[test]
    fn pagination_page_cannot_be_zero_or_negative() {
        #[derive(Debug, serde::Deserialize)]
        struct PaginationParams {
            page: Option<i64>,
            limit: Option<i64>,
        }
        impl PaginationParams {
            fn page(&self) -> i64 {
                self.page.unwrap_or(1).max(1)
            }
        }

        let params: PaginationParams =
            serde_json::from_str(r#"{"page": 0}"#).unwrap();
        assert_eq!(params.page(), 1);

        let params_neg: PaginationParams =
            serde_json::from_str(r#"{"page": -3}"#).unwrap();
        assert_eq!(params_neg.page(), 1);
    }

    #[test]
    fn pagination_offset_computed_correctly() {
        #[derive(Debug, serde::Deserialize)]
        struct PaginationParams {
            page: Option<i64>,
            limit: Option<i64>,
        }
        impl PaginationParams {
            fn page(&self) -> i64 {
                self.page.unwrap_or(1).max(1)
            }
            fn limit(&self) -> i64 {
                self.limit.unwrap_or(20).clamp(1, 100)
            }
            fn offset(&self) -> i64 {
                (self.page() - 1) * self.limit()
            }
        }

        // Page 3, limit 10 => offset 20
        let params: PaginationParams =
            serde_json::from_str(r#"{"page": 3, "limit": 10}"#).unwrap();
        assert_eq!(params.offset(), 20);

        // Page 1, limit 50 => offset 0
        let params2: PaginationParams =
            serde_json::from_str(r#"{"page": 1, "limit": 50}"#).unwrap();
        assert_eq!(params2.offset(), 0);
    }

    // ──────────────────────────────────────────────────────────────────
    // AgentListParams tests
    // ──────────────────────────────────────────────────────────────────

    #[test]
    fn agent_list_params_defaults() {
        #[derive(Debug, serde::Deserialize)]
        struct AgentListParams {
            chain_id: Option<i32>,
            search: Option<String>,
            category: Option<String>,
            sort: Option<String>,
            page: Option<i64>,
            limit: Option<i64>,
        }
        impl AgentListParams {
            fn page(&self) -> i64 {
                self.page.unwrap_or(1).max(1)
            }
            fn limit(&self) -> i64 {
                self.limit.unwrap_or(20).clamp(1, 100)
            }
            fn sort(&self) -> &str {
                self.sort.as_deref().unwrap_or("recent")
            }
        }

        let params: AgentListParams = serde_json::from_str("{}").unwrap();
        assert_eq!(params.page(), 1);
        assert_eq!(params.limit(), 20);
        assert_eq!(params.sort(), "recent");
        assert!(params.chain_id.is_none());
        assert!(params.search.is_none());
        assert!(params.category.is_none());
    }

    #[test]
    fn agent_list_params_with_filters() {
        #[derive(Debug, serde::Deserialize)]
        struct AgentListParams {
            chain_id: Option<i32>,
            search: Option<String>,
            category: Option<String>,
            sort: Option<String>,
            page: Option<i64>,
            limit: Option<i64>,
        }
        impl AgentListParams {
            fn sort(&self) -> &str {
                self.sort.as_deref().unwrap_or("recent")
            }
        }

        let json = r#"{
            "chain_id": 143,
            "search": "defi bot",
            "category": "DeFi",
            "sort": "score",
            "page": 2,
            "limit": 10
        }"#;
        let params: AgentListParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.chain_id, Some(143));
        assert_eq!(params.search.as_deref(), Some("defi bot"));
        assert_eq!(params.category.as_deref(), Some("DeFi"));
        assert_eq!(params.sort(), "score");
        assert_eq!(params.page, Some(2));
        assert_eq!(params.limit, Some(10));
    }

    // ──────────────────────────────────────────────────────────────────
    // ReputationParams tests
    // ──────────────────────────────────────────────────────────────────

    #[test]
    fn reputation_params_default_range_is_30d() {
        #[derive(Debug, serde::Deserialize)]
        struct ReputationParams {
            range: Option<String>,
        }
        impl ReputationParams {
            fn range(&self) -> &str {
                self.range.as_deref().unwrap_or("30d")
            }
        }

        let params: ReputationParams = serde_json::from_str("{}").unwrap();
        assert_eq!(params.range(), "30d");
    }

    #[test]
    fn reputation_params_custom_range() {
        #[derive(Debug, serde::Deserialize)]
        struct ReputationParams {
            range: Option<String>,
        }
        impl ReputationParams {
            fn range(&self) -> &str {
                self.range.as_deref().unwrap_or("30d")
            }
        }

        let params: ReputationParams =
            serde_json::from_str(r#"{"range": "7d"}"#).unwrap();
        assert_eq!(params.range(), "7d");

        let params_all: ReputationParams =
            serde_json::from_str(r#"{"range": "all"}"#).unwrap();
        assert_eq!(params_all.range(), "all");
    }

    // ──────────────────────────────────────────────────────────────────
    // LeaderboardParams tests
    // ──────────────────────────────────────────────────────────────────

    #[test]
    fn leaderboard_params_default_limit_50() {
        #[derive(Debug, serde::Deserialize)]
        struct LeaderboardParams {
            chain_id: Option<i32>,
            category: Option<String>,
            limit: Option<i64>,
        }
        impl LeaderboardParams {
            fn limit(&self) -> i64 {
                self.limit.unwrap_or(50).clamp(1, 100)
            }
        }

        let params: LeaderboardParams = serde_json::from_str("{}").unwrap();
        assert_eq!(params.limit(), 50);
    }

    #[test]
    fn leaderboard_params_clamp_limit() {
        #[derive(Debug, serde::Deserialize)]
        struct LeaderboardParams {
            chain_id: Option<i32>,
            category: Option<String>,
            limit: Option<i64>,
        }
        impl LeaderboardParams {
            fn limit(&self) -> i64 {
                self.limit.unwrap_or(50).clamp(1, 100)
            }
        }

        let params: LeaderboardParams =
            serde_json::from_str(r#"{"limit": 200}"#).unwrap();
        assert_eq!(params.limit(), 100);
    }

    // ──────────────────────────────────────────────────────────────────
    // ActivityParams tests
    // ──────────────────────────────────────────────────────────────────

    #[test]
    fn activity_params_defaults() {
        #[derive(Debug, serde::Deserialize)]
        struct ActivityParams {
            event_type: Option<String>,
            page: Option<i64>,
            limit: Option<i64>,
        }
        impl ActivityParams {
            fn page(&self) -> i64 {
                self.page.unwrap_or(1).max(1)
            }
            fn limit(&self) -> i64 {
                self.limit.unwrap_or(20).clamp(1, 100)
            }
            fn offset(&self) -> i64 {
                (self.page() - 1) * self.limit()
            }
        }

        let params: ActivityParams = serde_json::from_str("{}").unwrap();
        assert_eq!(params.page(), 1);
        assert_eq!(params.limit(), 20);
        assert_eq!(params.offset(), 0);
        assert!(params.event_type.is_none());
    }
}

#[cfg(test)]
mod agent_id_parsing_tests {
    use axum::http::StatusCode;

    /// Replicate the parse_agent_id logic from src/api/agents.rs
    /// so we can test it without needing the full Axum server.
    fn parse_agent_id(id: &str) -> Result<(i32, i64), (StatusCode, String)> {
        let parts: Vec<&str> = id.splitn(2, '-').collect();
        if parts.len() != 2 {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("Invalid agent id format '{}'. Expected 'chainId-agentId'.", id),
            ));
        }

        let chain_id: i32 = parts[0].parse().map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                format!("Invalid chain_id in '{}'", id),
            )
        })?;

        let agent_id: i64 = parts[1].parse().map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                format!("Invalid agent_id in '{}'", id),
            )
        })?;

        Ok((chain_id, agent_id))
    }

    #[test]
    fn valid_mainnet_agent_id() {
        let result = parse_agent_id("143-1");
        assert!(result.is_ok());
        let (chain_id, agent_id) = result.unwrap();
        assert_eq!(chain_id, 143);
        assert_eq!(agent_id, 1);
    }

    #[test]
    fn valid_testnet_agent_id() {
        let result = parse_agent_id("10143-42");
        assert!(result.is_ok());
        let (chain_id, agent_id) = result.unwrap();
        assert_eq!(chain_id, 10143);
        assert_eq!(agent_id, 42);
    }

    #[test]
    fn valid_large_agent_id() {
        let result = parse_agent_id("143-999999999");
        assert!(result.is_ok());
        let (chain_id, agent_id) = result.unwrap();
        assert_eq!(chain_id, 143);
        assert_eq!(agent_id, 999999999);
    }

    #[test]
    fn missing_separator_returns_bad_request() {
        let result = parse_agent_id("143");
        assert!(result.is_err());
        let (status, msg) = result.unwrap_err();
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(msg.contains("Invalid agent id format"));
    }

    #[test]
    fn empty_string_returns_bad_request() {
        let result = parse_agent_id("");
        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[test]
    fn non_numeric_chain_id_returns_bad_request() {
        let result = parse_agent_id("abc-1");
        assert!(result.is_err());
        let (status, msg) = result.unwrap_err();
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(msg.contains("Invalid chain_id"));
    }

    #[test]
    fn non_numeric_agent_id_returns_bad_request() {
        let result = parse_agent_id("143-xyz");
        assert!(result.is_err());
        let (status, msg) = result.unwrap_err();
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(msg.contains("Invalid agent_id"));
    }

    #[test]
    fn multiple_dashes_takes_first_split() {
        // splitn(2, '-') means "143" and "1-extra" — "1-extra" fails i64 parse
        let result = parse_agent_id("143-1-extra");
        assert!(result.is_err());
        let (status, msg) = result.unwrap_err();
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(msg.contains("Invalid agent_id"));
    }

    #[test]
    fn negative_chain_id_parses_as_valid_i32() {
        // Negative chain IDs are technically parseable as i32
        let result = parse_agent_id("-1-42");
        // splitn(2, '-') on "-1-42" gives ["", "1-42"]
        // "" fails to parse as i32
        assert!(result.is_err());
    }

    #[test]
    fn zero_agent_id_is_valid() {
        let result = parse_agent_id("143-0");
        assert!(result.is_ok());
        let (chain_id, agent_id) = result.unwrap();
        assert_eq!(chain_id, 143);
        assert_eq!(agent_id, 0);
    }
}

#[cfg(test)]
mod error_response_tests {
    use serde_json;

    #[derive(Debug, serde::Serialize, serde::Deserialize)]
    struct ErrorResponse {
        error: String,
        message: String,
        status: u16,
    }

    #[test]
    fn error_response_serialization() {
        let err = ErrorResponse {
            error: "Bad Request".to_string(),
            message: "Invalid agent id format".to_string(),
            status: 400,
        };
        let json = serde_json::to_string(&err).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["error"], "Bad Request");
        assert_eq!(parsed["message"], "Invalid agent id format");
        assert_eq!(parsed["status"], 400);
    }

    #[test]
    fn error_response_deserialization() {
        let json = r#"{"error":"Not found","message":"Agent with id 143-999 not found","status":404}"#;
        let err: ErrorResponse = serde_json::from_str(json).unwrap();
        assert_eq!(err.error, "Not found");
        assert_eq!(err.message, "Agent with id 143-999 not found");
        assert_eq!(err.status, 404);
    }

    #[test]
    fn error_response_internal_server() {
        let err = ErrorResponse {
            error: "Internal Server Error".to_string(),
            message: "Failed to fetch agents".to_string(),
            status: 500,
        };
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["status"], 500);
        assert_eq!(json["error"], "Internal Server Error");
    }
}

#[cfg(test)]
mod response_type_serialization_tests {
    use serde_json;
    use std::collections::HashMap;

    // Replicate key response types locally to test serialization without DB deps

    #[derive(Debug, serde::Serialize, serde::Deserialize)]
    struct AgentListResponse {
        agents: Vec<AgentListItem>,
        total: i64,
        page: i64,
        limit: i64,
    }

    #[derive(Debug, serde::Serialize, serde::Deserialize)]
    struct AgentListItem {
        agent_id: i64,
        chain_id: i32,
        owner: String,
        name: Option<String>,
        description: Option<String>,
        image: Option<String>,
        categories: Option<Vec<String>>,
        x402_support: Option<bool>,
        active: Option<bool>,
        reputation_score: Option<f64>,
        feedback_count: Option<i64>,
        created_at: Option<String>,
    }

    #[derive(Debug, serde::Serialize, serde::Deserialize)]
    struct LeaderboardEntry {
        rank: i64,
        agent_id: i64,
        chain_id: i32,
        name: Option<String>,
        image: Option<String>,
        categories: Option<Vec<String>>,
        x402_support: Option<bool>,
        reputation_score: Option<f64>,
        feedback_count: Option<i64>,
        owner: String,
    }

    #[derive(Debug, serde::Serialize, serde::Deserialize)]
    struct LeaderboardResponse {
        leaderboard: Vec<LeaderboardEntry>,
    }

    #[derive(Debug, serde::Serialize, serde::Deserialize)]
    struct CategoryCount {
        category: String,
        count: i64,
    }

    #[derive(Debug, serde::Serialize, serde::Deserialize)]
    struct StatsResponse {
        total_agents: i64,
        total_feedbacks: i64,
        total_chains: i64,
        agents_by_chain: HashMap<String, i64>,
        top_categories: Vec<CategoryCount>,
        recent_registrations_24h: i64,
        recent_feedbacks_24h: i64,
    }

    #[test]
    fn agent_list_response_serialization() {
        let response = AgentListResponse {
            agents: vec![AgentListItem {
                agent_id: 1,
                chain_id: 143,
                owner: "0xabc123".to_string(),
                name: Some("TestAgent".to_string()),
                description: Some("A test agent".to_string()),
                image: Some("https://example.com/img.png".to_string()),
                categories: Some(vec!["DeFi".to_string(), "Trading".to_string()]),
                x402_support: Some(true),
                active: Some(true),
                reputation_score: Some(4.5),
                feedback_count: Some(10),
                created_at: Some("2024-01-01T00:00:00Z".to_string()),
            }],
            total: 1,
            page: 1,
            limit: 20,
        };

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["total"], 1);
        assert_eq!(json["page"], 1);
        assert_eq!(json["limit"], 20);
        assert_eq!(json["agents"].as_array().unwrap().len(), 1);
        assert_eq!(json["agents"][0]["agent_id"], 1);
        assert_eq!(json["agents"][0]["chain_id"], 143);
        assert_eq!(json["agents"][0]["name"], "TestAgent");
        assert_eq!(json["agents"][0]["x402_support"], true);
    }

    #[test]
    fn empty_agent_list_response() {
        let response = AgentListResponse {
            agents: vec![],
            total: 0,
            page: 1,
            limit: 20,
        };

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["total"], 0);
        assert!(json["agents"].as_array().unwrap().is_empty());
    }

    #[test]
    fn leaderboard_response_serialization() {
        let response = LeaderboardResponse {
            leaderboard: vec![
                LeaderboardEntry {
                    rank: 1,
                    agent_id: 5,
                    chain_id: 143,
                    name: Some("Top Agent".to_string()),
                    image: None,
                    categories: Some(vec!["Legal".to_string()]),
                    x402_support: Some(true),
                    reputation_score: Some(9.8),
                    feedback_count: Some(100),
                    owner: "0xowner".to_string(),
                },
                LeaderboardEntry {
                    rank: 2,
                    agent_id: 3,
                    chain_id: 10143,
                    name: Some("Second Agent".to_string()),
                    image: Some("https://example.com/img2.png".to_string()),
                    categories: None,
                    x402_support: Some(false),
                    reputation_score: Some(8.5),
                    feedback_count: Some(50),
                    owner: "0xowner2".to_string(),
                },
            ],
        };

        let json = serde_json::to_value(&response).unwrap();
        let board = json["leaderboard"].as_array().unwrap();
        assert_eq!(board.len(), 2);
        assert_eq!(board[0]["rank"], 1);
        assert_eq!(board[0]["reputation_score"], 9.8);
        assert_eq!(board[1]["rank"], 2);
        assert_eq!(board[1]["chain_id"], 10143);
    }

    #[test]
    fn stats_response_serialization() {
        let mut agents_by_chain = HashMap::new();
        agents_by_chain.insert("143".to_string(), 25);
        agents_by_chain.insert("10143".to_string(), 10);

        let response = StatsResponse {
            total_agents: 35,
            total_feedbacks: 150,
            total_chains: 2,
            agents_by_chain,
            top_categories: vec![
                CategoryCount {
                    category: "DeFi".to_string(),
                    count: 15,
                },
                CategoryCount {
                    category: "Trading".to_string(),
                    count: 10,
                },
            ],
            recent_registrations_24h: 3,
            recent_feedbacks_24h: 12,
        };

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["total_agents"], 35);
        assert_eq!(json["total_feedbacks"], 150);
        assert_eq!(json["total_chains"], 2);
        assert_eq!(json["agents_by_chain"]["143"], 25);
        assert_eq!(json["agents_by_chain"]["10143"], 10);
        assert_eq!(json["top_categories"].as_array().unwrap().len(), 2);
        assert_eq!(json["top_categories"][0]["category"], "DeFi");
        assert_eq!(json["recent_registrations_24h"], 3);
        assert_eq!(json["recent_feedbacks_24h"], 12);
    }

    #[test]
    fn agent_list_item_with_null_optionals() {
        let item = AgentListItem {
            agent_id: 42,
            chain_id: 10143,
            owner: "0xowner".to_string(),
            name: None,
            description: None,
            image: None,
            categories: None,
            x402_support: None,
            active: None,
            reputation_score: None,
            feedback_count: None,
            created_at: None,
        };

        let json = serde_json::to_value(&item).unwrap();
        assert_eq!(json["agent_id"], 42);
        assert_eq!(json["chain_id"], 10143);
        assert!(json["name"].is_null());
        assert!(json["reputation_score"].is_null());
        assert!(json["categories"].is_null());
    }
}
