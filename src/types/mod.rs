use bigdecimal::BigDecimal;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::collections::HashMap;

// ─── Database Models ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Agent {
    pub id: i32,
    pub agent_id: i64,
    pub chain_id: i32,
    pub owner: String,
    pub uri: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub image: Option<String>,
    pub categories: Option<Vec<String>>,
    pub x402_support: Option<bool>,
    pub active: Option<bool>,
    pub block_number: Option<i64>,
    pub block_timestamp: Option<DateTime<Utc>>,
    pub tx_hash: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetadata {
    pub version: Option<String>,
    pub endpoints: Option<Vec<AgentEndpoint>>,
    pub capabilities: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEndpoint {
    pub url: String,
    pub protocol: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Feedback {
    pub id: i32,
    pub agent_id: i64,
    pub chain_id: i32,
    pub client_address: String,
    pub feedback_index: i64,
    pub value: BigDecimal,
    pub value_decimals: Option<i32>,
    pub tag1: Option<String>,
    pub tag2: Option<String>,
    pub endpoint: Option<String>,
    pub feedback_uri: Option<String>,
    pub feedback_hash: Option<String>,
    pub revoked: Option<bool>,
    pub block_number: i64,
    pub block_timestamp: Option<DateTime<Utc>>,
    pub tx_hash: String,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct FeedbackResponse {
    pub id: i32,
    pub feedback_id: i64,
    pub agent_id: i64,
    pub chain_id: i32,
    pub response_uri: Option<String>,
    pub block_number: i64,
    pub block_timestamp: Option<DateTime<Utc>>,
    pub tx_hash: String,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Activity {
    pub id: i32,
    pub agent_id: i64,
    pub chain_id: i32,
    pub event_type: String,
    pub event_data: Option<serde_json::Value>,
    pub block_number: i64,
    pub block_timestamp: Option<DateTime<Utc>>,
    pub tx_hash: String,
    pub log_index: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct IndexerState {
    pub chain_id: i32,
    pub contract_address: String,
    pub last_block: i64,
    pub updated_at: Option<DateTime<Utc>>,
}

// ─── Marketplace Database Models ──────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MarketplaceListing {
    pub id: i32,
    pub listing_id: i64,
    pub chain_id: i32,
    pub seller: String,
    pub nft_contract: String,
    pub token_id: BigDecimal,
    pub payment_token: String,
    pub price: BigDecimal,
    pub expiry: i64,
    pub status: String,
    pub buyer: Option<String>,
    pub sold_price: Option<BigDecimal>,
    pub block_number: i64,
    pub block_timestamp: Option<DateTime<Utc>>,
    pub tx_hash: String,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MarketplaceOffer {
    pub id: i32,
    pub offer_id: i64,
    pub chain_id: i32,
    pub offerer: String,
    pub nft_contract: String,
    pub token_id: BigDecimal,
    pub payment_token: String,
    pub amount: BigDecimal,
    pub expiry: i64,
    pub status: String,
    pub accepted_by: Option<String>,
    pub block_number: i64,
    pub block_timestamp: Option<DateTime<Utc>>,
    pub tx_hash: String,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MarketplaceCollectionOffer {
    pub id: i32,
    pub offer_id: i64,
    pub chain_id: i32,
    pub offerer: String,
    pub nft_contract: String,
    pub payment_token: String,
    pub amount: BigDecimal,
    pub expiry: i64,
    pub status: String,
    pub accepted_by: Option<String>,
    pub accepted_token_id: Option<BigDecimal>,
    pub block_number: i64,
    pub block_timestamp: Option<DateTime<Utc>>,
    pub tx_hash: String,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MarketplaceAuction {
    pub id: i32,
    pub auction_id: i64,
    pub chain_id: i32,
    pub seller: String,
    pub nft_contract: String,
    pub token_id: BigDecimal,
    pub payment_token: String,
    pub start_price: BigDecimal,
    pub reserve_price: BigDecimal,
    pub buy_now_price: BigDecimal,
    pub highest_bid: Option<BigDecimal>,
    pub highest_bidder: Option<String>,
    pub start_time: i64,
    pub end_time: i64,
    pub bid_count: Option<i32>,
    pub status: String,
    pub winner: Option<String>,
    pub settled_price: Option<BigDecimal>,
    pub block_number: i64,
    pub block_timestamp: Option<DateTime<Utc>>,
    pub tx_hash: String,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MarketplaceAuctionBid {
    pub id: i32,
    pub auction_id: i64,
    pub chain_id: i32,
    pub bidder: String,
    pub amount: BigDecimal,
    pub block_number: i64,
    pub block_timestamp: Option<DateTime<Utc>>,
    pub tx_hash: String,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MarketplaceDutchAuction {
    pub id: i32,
    pub auction_id: i64,
    pub chain_id: i32,
    pub seller: String,
    pub nft_contract: String,
    pub token_id: BigDecimal,
    pub payment_token: String,
    pub start_price: BigDecimal,
    pub end_price: BigDecimal,
    pub start_time: i64,
    pub end_time: i64,
    pub status: String,
    pub buyer: Option<String>,
    pub sold_price: Option<BigDecimal>,
    pub block_number: i64,
    pub block_timestamp: Option<DateTime<Utc>>,
    pub tx_hash: String,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MarketplaceBundle {
    pub id: i32,
    pub bundle_id: i64,
    pub chain_id: i32,
    pub seller: String,
    pub nft_contracts: Vec<String>,
    pub token_ids: Vec<BigDecimal>,
    pub payment_token: String,
    pub price: BigDecimal,
    pub expiry: i64,
    pub item_count: i32,
    pub status: String,
    pub buyer: Option<String>,
    pub sold_price: Option<BigDecimal>,
    pub block_number: i64,
    pub block_timestamp: Option<DateTime<Utc>>,
    pub tx_hash: String,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

// ─── API Response Types ────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct AgentListItem {
    pub agent_id: i64,
    pub chain_id: i32,
    pub owner: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub image: Option<String>,
    pub categories: Option<Vec<String>>,
    pub x402_support: Option<bool>,
    pub active: Option<bool>,
    pub reputation_score: Option<f64>,
    pub feedback_count: Option<i64>,
    pub block_timestamp: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentListResponse {
    pub agents: Vec<AgentListItem>,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct AgentDetailResponse {
    pub agent_id: i64,
    pub chain_id: i32,
    pub owner: String,
    pub uri: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub image: Option<String>,
    pub categories: Option<Vec<String>>,
    pub x402_support: Option<bool>,
    pub active: Option<bool>,
    pub metadata: Option<serde_json::Value>,
    pub reputation_score: Option<f64>,
    pub feedback_count: Option<i64>,
    pub positive_feedback_count: Option<i64>,
    pub negative_feedback_count: Option<i64>,
    pub block_timestamp: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct ReputationHistoryPoint {
    pub date: NaiveDate,
    pub score: Option<f64>,
    pub feedback_count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReputationResponse {
    pub agent_id: i64,
    pub chain_id: i32,
    pub current_score: Option<f64>,
    pub history: Vec<ReputationHistoryPoint>,
    pub feedbacks: Vec<Feedback>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ActivityResponse {
    pub activities: Vec<Activity>,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct LeaderboardEntry {
    pub rank: i64,
    pub agent_id: i64,
    pub chain_id: i32,
    pub name: Option<String>,
    pub image: Option<String>,
    pub categories: Option<Vec<String>>,
    pub x402_support: Option<bool>,
    pub reputation_score: Option<f64>,
    pub feedback_count: Option<i64>,
    pub owner: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LeaderboardResponse {
    pub leaderboard: Vec<LeaderboardEntry>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct CategoryCount {
    pub category: String,
    pub count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatsResponse {
    pub total_agents: i64,
    pub total_feedbacks: i64,
    pub total_chains: i64,
    pub agents_by_chain: HashMap<String, i64>,
    pub top_categories: Vec<CategoryCount>,
    pub recent_registrations_24h: i64,
    pub recent_feedbacks_24h: i64,
    // Marketplace stats
    pub total_listings: i64,
    pub active_listings: i64,
    pub total_sales: i64,
    pub total_volume: BigDecimal,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MarketplaceListingListResponse {
    pub listings: Vec<MarketplaceListing>,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MarketplaceOfferListResponse {
    pub offers: Vec<MarketplaceOffer>,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MarketplaceCollectionOfferListResponse {
    pub offers: Vec<MarketplaceCollectionOffer>,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MarketplaceAuctionListResponse {
    pub auctions: Vec<MarketplaceAuction>,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MarketplaceAuctionDetailResponse {
    #[serde(flatten)]
    pub auction: MarketplaceAuction,
    pub bids: Vec<MarketplaceAuctionBid>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MarketplaceDutchAuctionListResponse {
    pub auctions: Vec<MarketplaceDutchAuction>,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MarketplaceBundleListResponse {
    pub bundles: Vec<MarketplaceBundle>,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MarketplaceUserPortfolioResponse {
    pub listings: Vec<MarketplaceListing>,
    pub offers: Vec<MarketplaceOffer>,
    pub bids: Vec<MarketplaceAuctionBid>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MarketplaceStatsResponse {
    pub total_listings: i64,
    pub active_listings: i64,
    pub total_sales: i64,
    pub total_volume: BigDecimal,
    pub active_auctions: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    pub status: u16,
}

// ─── Query Parameters ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

impl PaginationParams {
    pub fn page(&self) -> i64 {
        self.page.unwrap_or(1).max(1)
    }

    pub fn limit(&self) -> i64 {
        self.limit.unwrap_or(20).clamp(1, 100)
    }

    pub fn offset(&self) -> i64 {
        (self.page() - 1) * self.limit()
    }
}

#[derive(Debug, Deserialize)]
pub struct AgentListParams {
    pub chain_id: Option<i32>,
    pub search: Option<String>,
    pub category: Option<String>,
    pub sort: Option<String>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

impl AgentListParams {
    pub fn page(&self) -> i64 {
        self.page.unwrap_or(1).max(1)
    }

    pub fn limit(&self) -> i64 {
        self.limit.unwrap_or(20).clamp(1, 100)
    }

    pub fn offset(&self) -> i64 {
        (self.page() - 1) * self.limit()
    }

    pub fn sort(&self) -> &str {
        self.sort.as_deref().unwrap_or("recent")
    }
}

#[derive(Debug, Deserialize)]
pub struct ReputationParams {
    pub range: Option<String>,
}

impl ReputationParams {
    pub fn range(&self) -> &str {
        self.range.as_deref().unwrap_or("30d")
    }
}

#[derive(Debug, Deserialize)]
pub struct ActivityParams {
    pub event_type: Option<String>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

impl ActivityParams {
    pub fn page(&self) -> i64 {
        self.page.unwrap_or(1).max(1)
    }

    pub fn limit(&self) -> i64 {
        self.limit.unwrap_or(20).clamp(1, 100)
    }

    pub fn offset(&self) -> i64 {
        (self.page() - 1) * self.limit()
    }
}

#[derive(Debug, Deserialize)]
pub struct LeaderboardParams {
    pub chain_id: Option<i32>,
    pub category: Option<String>,
    pub limit: Option<i64>,
}

impl LeaderboardParams {
    pub fn limit(&self) -> i64 {
        self.limit.unwrap_or(50).clamp(1, 100)
    }
}

// ─── Marketplace Query Parameters ────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct MarketplaceListParams {
    pub chain_id: Option<i32>,
    pub nft_contract: Option<String>,
    pub seller: Option<String>,
    pub status: Option<String>,
    pub sort: Option<String>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

impl MarketplaceListParams {
    pub fn page(&self) -> i64 {
        self.page.unwrap_or(1).max(1)
    }
    pub fn limit(&self) -> i64 {
        self.limit.unwrap_or(20).clamp(1, 100)
    }
    pub fn offset(&self) -> i64 {
        (self.page() - 1) * self.limit()
    }
    pub fn status(&self) -> &str {
        self.status.as_deref().unwrap_or("Active")
    }
    pub fn sort(&self) -> &str {
        self.sort.as_deref().unwrap_or("recent")
    }
}

#[derive(Debug, Deserialize)]
pub struct MarketplaceOfferParams {
    pub chain_id: Option<i32>,
    pub nft_contract: Option<String>,
    pub token_id: Option<String>,
    pub offerer: Option<String>,
    pub status: Option<String>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

impl MarketplaceOfferParams {
    pub fn page(&self) -> i64 {
        self.page.unwrap_or(1).max(1)
    }
    pub fn limit(&self) -> i64 {
        self.limit.unwrap_or(20).clamp(1, 100)
    }
    pub fn offset(&self) -> i64 {
        (self.page() - 1) * self.limit()
    }
}

#[derive(Debug, Deserialize)]
pub struct MarketplaceCollectionOfferParams {
    pub chain_id: Option<i32>,
    pub nft_contract: Option<String>,
    pub offerer: Option<String>,
    pub status: Option<String>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

impl MarketplaceCollectionOfferParams {
    pub fn page(&self) -> i64 {
        self.page.unwrap_or(1).max(1)
    }
    pub fn limit(&self) -> i64 {
        self.limit.unwrap_or(20).clamp(1, 100)
    }
    pub fn offset(&self) -> i64 {
        (self.page() - 1) * self.limit()
    }
}

#[derive(Debug, Deserialize)]
pub struct MarketplaceAuctionParams {
    pub chain_id: Option<i32>,
    pub nft_contract: Option<String>,
    pub seller: Option<String>,
    pub status: Option<String>,
    pub sort: Option<String>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

impl MarketplaceAuctionParams {
    pub fn page(&self) -> i64 {
        self.page.unwrap_or(1).max(1)
    }
    pub fn limit(&self) -> i64 {
        self.limit.unwrap_or(20).clamp(1, 100)
    }
    pub fn offset(&self) -> i64 {
        (self.page() - 1) * self.limit()
    }
    pub fn sort(&self) -> &str {
        self.sort.as_deref().unwrap_or("recent")
    }
}

#[derive(Debug, Deserialize)]
pub struct MarketplaceBundleParams {
    pub chain_id: Option<i32>,
    pub seller: Option<String>,
    pub status: Option<String>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

impl MarketplaceBundleParams {
    pub fn page(&self) -> i64 {
        self.page.unwrap_or(1).max(1)
    }
    pub fn limit(&self) -> i64 {
        self.limit.unwrap_or(20).clamp(1, 100)
    }
    pub fn offset(&self) -> i64 {
        (self.page() - 1) * self.limit()
    }
}

#[derive(Debug, Deserialize)]
pub struct MarketplaceUserParams {
    pub chain_id: Option<i32>,
}

// ─── Insert helpers (for DB write operations) ──────────────────────────

#[derive(Debug, Clone)]
pub struct NewAgent {
    pub agent_id: i64,
    pub chain_id: i32,
    pub owner: String,
    pub uri: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub image: Option<String>,
    pub categories: Option<Vec<String>>,
    pub x402_support: bool,
    pub active: bool,
    pub block_number: Option<i64>,
    pub block_timestamp: Option<DateTime<Utc>>,
    pub tx_hash: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewFeedback {
    pub agent_id: i64,
    pub chain_id: i32,
    pub client_address: String,
    pub feedback_index: i64,
    pub value: BigDecimal,
    pub value_decimals: i32,
    pub tag1: Option<String>,
    pub tag2: Option<String>,
    pub endpoint: Option<String>,
    pub feedback_uri: Option<String>,
    pub feedback_hash: Option<String>,
    pub block_number: i64,
    pub block_timestamp: Option<DateTime<Utc>>,
    pub tx_hash: String,
}

#[derive(Debug, Clone)]
pub struct NewActivity {
    pub agent_id: i64,
    pub chain_id: i32,
    pub event_type: String,
    pub event_data: Option<serde_json::Value>,
    pub block_number: i64,
    pub block_timestamp: Option<DateTime<Utc>>,
    pub tx_hash: String,
    pub log_index: i32,
}

// ─── Marketplace Insert Helpers ─────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct NewMarketplaceListing {
    pub listing_id: i64,
    pub chain_id: i32,
    pub seller: String,
    pub nft_contract: String,
    pub token_id: BigDecimal,
    pub payment_token: String,
    pub price: BigDecimal,
    pub expiry: i64,
    pub block_number: i64,
    pub block_timestamp: Option<DateTime<Utc>>,
    pub tx_hash: String,
}

#[derive(Debug, Clone)]
pub struct NewMarketplaceOffer {
    pub offer_id: i64,
    pub chain_id: i32,
    pub offerer: String,
    pub nft_contract: String,
    pub token_id: BigDecimal,
    pub payment_token: String,
    pub amount: BigDecimal,
    pub expiry: i64,
    pub block_number: i64,
    pub block_timestamp: Option<DateTime<Utc>>,
    pub tx_hash: String,
}

#[derive(Debug, Clone)]
pub struct NewMarketplaceCollectionOffer {
    pub offer_id: i64,
    pub chain_id: i32,
    pub offerer: String,
    pub nft_contract: String,
    pub payment_token: String,
    pub amount: BigDecimal,
    pub expiry: i64,
    pub block_number: i64,
    pub block_timestamp: Option<DateTime<Utc>>,
    pub tx_hash: String,
}

#[derive(Debug, Clone)]
pub struct NewMarketplaceAuction {
    pub auction_id: i64,
    pub chain_id: i32,
    pub seller: String,
    pub nft_contract: String,
    pub token_id: BigDecimal,
    pub payment_token: String,
    pub start_price: BigDecimal,
    pub reserve_price: BigDecimal,
    pub buy_now_price: BigDecimal,
    pub start_time: i64,
    pub end_time: i64,
    pub block_number: i64,
    pub block_timestamp: Option<DateTime<Utc>>,
    pub tx_hash: String,
}

#[derive(Debug, Clone)]
pub struct NewMarketplaceDutchAuction {
    pub auction_id: i64,
    pub chain_id: i32,
    pub seller: String,
    pub nft_contract: String,
    pub token_id: BigDecimal,
    pub payment_token: String,
    pub start_price: BigDecimal,
    pub end_price: BigDecimal,
    pub start_time: i64,
    pub end_time: i64,
    pub block_number: i64,
    pub block_timestamp: Option<DateTime<Utc>>,
    pub tx_hash: String,
}

#[derive(Debug, Clone)]
pub struct NewMarketplaceBundle {
    pub bundle_id: i64,
    pub chain_id: i32,
    pub seller: String,
    pub nft_contracts: Vec<String>,
    pub token_ids: Vec<BigDecimal>,
    pub payment_token: String,
    pub price: BigDecimal,
    pub expiry: i64,
    pub item_count: i32,
    pub block_number: i64,
    pub block_timestamp: Option<DateTime<Utc>>,
    pub tx_hash: String,
}
