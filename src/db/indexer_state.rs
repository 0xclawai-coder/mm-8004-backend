use sqlx::PgPool;

/// Get the last indexed block number for a given chain_id and contract_address.
pub async fn get_last_block(
    pool: &PgPool,
    chain_id: i32,
    contract_address: &str,
) -> Result<Option<i64>, sqlx::Error> {
    let row: Option<(i64,)> = sqlx::query_as(
        r#"
        SELECT last_block
        FROM indexer_state
        WHERE chain_id = $1 AND contract_address = $2
        "#,
    )
    .bind(chain_id)
    .bind(contract_address)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| r.0))
}

/// Upsert the last indexed block number cursor for a chain/contract pair.
pub async fn update_last_block(
    pool: &PgPool,
    chain_id: i32,
    contract_address: &str,
    block_number: i64,
) -> Result<(), sqlx::Error> {
    update_last_block_with_name(pool, chain_id, contract_address, block_number, None).await
}

/// Upsert the last indexed block number cursor with a contract name label.
pub async fn update_last_block_with_name(
    pool: &PgPool,
    chain_id: i32,
    contract_address: &str,
    block_number: i64,
    contract_name: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO indexer_state (chain_id, contract_address, last_block, contract_name, updated_at)
        VALUES ($1, $2, $3, $4, NOW())
        ON CONFLICT (chain_id, contract_address) DO UPDATE SET
            last_block = EXCLUDED.last_block,
            contract_name = COALESCE(EXCLUDED.contract_name, indexer_state.contract_name),
            updated_at = NOW()
        "#,
    )
    .bind(chain_id)
    .bind(contract_address)
    .bind(block_number)
    .bind(contract_name)
    .execute(pool)
    .await?;

    Ok(())
}
