use std::collections::HashMap;
use std::str::FromStr;

use alloy::providers::Provider;
use alloy::rpc::types::Filter;
use alloy::sol;
use alloy::sol_types::SolEvent;
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::provider::{self, ChainConfig, HttpProvider};
use crate::db;
use crate::types::{NewActivity, NewFeedback as NewFeedbackDb};

// Load ReputationRegistry ABI from official erc-8004 contracts.
// The generated `NewFeedback` struct name matches the Solidity event name.
// We import the DB type as `NewFeedbackDb` to avoid the naming conflict.
sol!(ReputationRegistry, "abi/ReputationRegistry.json");

use ReputationRegistry::{NewFeedback, FeedbackRevoked, ResponseAppended};

/// Index reputation events (NewFeedback, FeedbackRevoked, ResponseAppended) for a block range.
pub async fn index_reputation_events(
    pool: &PgPool,
    provider: &HttpProvider,
    chain: &ChainConfig,
    from_block: u64,
    to_block: u64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let filter = Filter::new()
        .address(chain.reputation_address)
        .from_block(from_block)
        .to_block(to_block)
        .event_signature(vec![
            NewFeedback::SIGNATURE_HASH,
            FeedbackRevoked::SIGNATURE_HASH,
            ResponseAppended::SIGNATURE_HASH,
        ]);

    let logs = provider.get_logs(&filter).await?;

    tracing::debug!(
        chain_id = chain.chain_id,
        "Fetched {} reputation logs for blocks {} - {}",
        logs.len(),
        from_block,
        to_block
    );

    // Cache block timestamps to avoid duplicate RPC calls for the same block
    let mut block_ts_cache: HashMap<u64, DateTime<Utc>> = HashMap::new();

    for log in logs {
        let block_num_raw = log.block_number.unwrap_or(0);
        let block_number = block_num_raw as i64;
        let tx_hash = log
            .transaction_hash
            .map(|h| format!("{:#x}", h))
            .unwrap_or_default();
        let log_index = log.log_index.unwrap_or(0) as i32;

        // Fetch block timestamp (cached per block)
        let block_timestamp = if let Some(ts) = block_ts_cache.get(&block_num_raw) {
            Some(*ts)
        } else {
            match provider::get_block_timestamp(provider, block_num_raw).await {
                Ok(ts) => {
                    block_ts_cache.insert(block_num_raw, ts);
                    Some(ts)
                }
                Err(e) => {
                    tracing::warn!("Failed to fetch timestamp for block {}: {:?}", block_num_raw, e);
                    None
                }
            }
        };

        let topic0 = match log.topic0() {
            Some(t) => *t,
            None => {
                tracing::warn!(
                    "Skipping reputation log with no topic0 at block {}",
                    block_number
                );
                continue;
            }
        };

        if topic0 == NewFeedback::SIGNATURE_HASH {
            match log.log_decode::<NewFeedback>() {
                Ok(decoded) => {
                    let event = &decoded.inner.data;
                    let agent_id = event.agentId.to::<u64>() as i64;
                    let client = format!("{:#x}", event.clientAddress);
                    let feedback_index = event.feedbackIndex as i64;
                    let value_raw = event.value;
                    let value_decimals = event.valueDecimals as i32;
                    let tag1 = event.tag1.clone();
                    let tag2 = event.tag2.clone();
                    let endpoint = event.endpoint.clone();
                    let feedback_uri = event.feedbackURI.clone();
                    let feedback_hash = format!("{:#x}", event.feedbackHash);

                    // Convert int128 value to BigDecimal
                    let value = BigDecimal::from_str(&value_raw.to_string())
                        .unwrap_or_else(|_| BigDecimal::from(0));

                    tracing::info!(
                        chain_id = chain.chain_id,
                        agent_id = agent_id,
                        "NewFeedback event: agent {} feedback #{} from {}",
                        agent_id,
                        feedback_index,
                        client
                    );

                    // Insert feedback
                    let new_feedback = NewFeedbackDb {
                        agent_id,
                        chain_id: chain.chain_id,
                        client_address: client.clone(),
                        feedback_index,
                        value,
                        value_decimals,
                        tag1: if tag1.is_empty() {
                            None
                        } else {
                            Some(tag1.clone())
                        },
                        tag2: if tag2.is_empty() {
                            None
                        } else {
                            Some(tag2.clone())
                        },
                        endpoint: if endpoint.is_empty() {
                            None
                        } else {
                            Some(endpoint.clone())
                        },
                        feedback_uri: if feedback_uri.is_empty() {
                            None
                        } else {
                            Some(feedback_uri.clone())
                        },
                        feedback_hash: Some(feedback_hash.clone()),
                        block_number,
                        block_timestamp,
                        tx_hash: tx_hash.clone(),
                    };
                    if let Err(e) = db::feedbacks::insert_feedback(pool, &new_feedback).await {
                        tracing::error!(
                            "Failed to insert feedback for agent {}: {:?}",
                            agent_id,
                            e
                        );
                    }

                    // Compute normalized value for activity display
                    let normalized_value = value_raw as f64
                        / 10f64.powi(value_decimals);

                    // Insert activity
                    let activity = NewActivity {
                        agent_id,
                        chain_id: chain.chain_id,
                        event_type: "NewFeedback".to_string(),
                        event_data: Some(serde_json::json!({
                            "client": client,
                            "feedback_index": feedback_index,
                            "value": normalized_value,
                            "value_decimals": value_decimals,
                            "tag1": tag1,
                            "tag2": tag2,
                            "endpoint": endpoint,
                            "feedback_uri": feedback_uri,
                        })),
                        block_number,
                        block_timestamp,
                        tx_hash: tx_hash.clone(),
                        log_index,
                    };
                    if let Err(e) = db::activity::insert_activity(pool, &activity).await {
                        tracing::error!("Failed to insert NewFeedback activity: {:?}", e);
                    }
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to decode NewFeedback event at block {}: {:?}",
                        block_number,
                        e
                    );
                }
            }
        } else if topic0 == FeedbackRevoked::SIGNATURE_HASH {
            match log.log_decode::<FeedbackRevoked>() {
                Ok(decoded) => {
                    let event = &decoded.inner.data;
                    let agent_id = event.agentId.to::<u64>() as i64;
                    let client = format!("{:#x}", event.clientAddress);
                    let feedback_index = event.feedbackIndex as i64;

                    tracing::info!(
                        chain_id = chain.chain_id,
                        agent_id = agent_id,
                        "FeedbackRevoked event: agent {} feedback #{} by {}",
                        agent_id,
                        feedback_index,
                        client
                    );

                    // Revoke the feedback
                    if let Err(e) = db::feedbacks::revoke_feedback(
                        pool,
                        agent_id,
                        chain.chain_id,
                        feedback_index,
                    )
                    .await
                    {
                        tracing::error!(
                            "Failed to revoke feedback for agent {}: {:?}",
                            agent_id,
                            e
                        );
                    }

                    // Insert activity
                    let activity = NewActivity {
                        agent_id,
                        chain_id: chain.chain_id,
                        event_type: "FeedbackRevoked".to_string(),
                        event_data: Some(serde_json::json!({
                            "client": client,
                            "feedback_index": feedback_index,
                        })),
                        block_number,
                        block_timestamp,
                        tx_hash: tx_hash.clone(),
                        log_index,
                    };
                    if let Err(e) = db::activity::insert_activity(pool, &activity).await {
                        tracing::error!("Failed to insert FeedbackRevoked activity: {:?}", e);
                    }
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to decode FeedbackRevoked event at block {}: {:?}",
                        block_number,
                        e
                    );
                }
            }
        } else if topic0 == ResponseAppended::SIGNATURE_HASH {
            match log.log_decode::<ResponseAppended>() {
                Ok(decoded) => {
                    let event = &decoded.inner.data;
                    let agent_id = event.agentId.to::<u64>() as i64;
                    let client = format!("{:#x}", event.clientAddress);
                    let feedback_index = event.feedbackIndex as i64;
                    let responder = format!("{:#x}", event.responder);
                    let response_uri = event.responseURI.clone();
                    let response_hash = format!("{:#x}", event.responseHash);

                    tracing::info!(
                        chain_id = chain.chain_id,
                        agent_id = agent_id,
                        "ResponseAppended event: agent {} feedback #{}",
                        agent_id,
                        feedback_index,
                    );

                    // Insert feedback response
                    if let Err(e) = insert_feedback_response(
                        pool,
                        feedback_index,
                        agent_id,
                        chain.chain_id,
                        &response_uri,
                        block_number,
                        block_timestamp,
                        &tx_hash,
                    )
                    .await
                    {
                        tracing::error!(
                            "Failed to insert feedback response for agent {}: {:?}",
                            agent_id,
                            e
                        );
                    }

                    // Insert activity
                    let activity = NewActivity {
                        agent_id,
                        chain_id: chain.chain_id,
                        event_type: "ResponseAppended".to_string(),
                        event_data: Some(serde_json::json!({
                            "client": client,
                            "feedback_index": feedback_index,
                            "responder": responder,
                            "response_uri": response_uri,
                            "response_hash": response_hash,
                        })),
                        block_number,
                        block_timestamp,
                        tx_hash: tx_hash.clone(),
                        log_index,
                    };
                    if let Err(e) = db::activity::insert_activity(pool, &activity).await {
                        tracing::error!("Failed to insert ResponseAppended activity: {:?}", e);
                    }
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to decode ResponseAppended event at block {}: {:?}",
                        block_number,
                        e
                    );
                }
            }
        }
    }

    Ok(())
}

/// Insert a feedback response into the feedback_responses table.
async fn insert_feedback_response(
    pool: &PgPool,
    feedback_index: i64,
    agent_id: i64,
    chain_id: i32,
    response_uri: &str,
    block_number: i64,
    block_timestamp: Option<DateTime<Utc>>,
    tx_hash: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO feedback_responses (feedback_id, agent_id, chain_id, response_uri, block_number, block_timestamp, tx_hash)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(feedback_index)
    .bind(agent_id)
    .bind(chain_id)
    .bind(response_uri)
    .bind(block_number)
    .bind(block_timestamp)
    .bind(tx_hash)
    .execute(pool)
    .await?;

    Ok(())
}
