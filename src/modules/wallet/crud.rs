use sqlx::{MySql, Pool};
use crate::modules::wallet::model::SwapAddressInfo;

pub struct WalletCrud {
    pool: Pool<MySql>,
}

impl WalletCrud {
    pub fn new(pool: Pool<MySql>) -> Self {
        Self { pool }
    }

    /// Get the next available address index by finding the maximum index used
    pub async fn get_next_index(&self) -> Result<u32, sqlx::Error> {
        let result: (Option<u32>,) = sqlx::query_as(
            "SELECT MAX(address_index) FROM swap_address_info"
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(result.0.map(|idx| idx + 1).unwrap_or(0))
    }

    /// Save address information for a swap
    pub async fn save_address_info(
        &self,
        swap_id: &str,
        our_address: &str,
        address_index: u32,
        network: &str,
    ) -> Result<(), sqlx::Error> {
        // Fetch original recipient from swaps table to keep it consistent
        let swap = sqlx::query!(
            "SELECT recipient_address, recipient_extra_id FROM swaps WHERE id = ?",
            swap_id
        )
        .fetch_one(&self.pool)
        .await?;

        let coin_type = match network.to_lowercase().as_str() {
            "bitcoin" => 0,
            "ethereum" | "polygon" | "bsc" | "arbitrum" => 60,
            "solana" => 501,
            _ => 60,
        };

        sqlx::query(
            r#"
            INSERT INTO swap_address_info (
                swap_id, our_address, address_index, blockchain_id,
                coin_type, recipient_address, recipient_extra_id
            )
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(swap_id)
        .bind(our_address)
        .bind(address_index)
        .bind(1) // Default blockchain_id for now
        .bind(coin_type)
        .bind(swap.recipient_address)
        .bind(swap.recipient_extra_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Fetch address info for a specific swap
    pub async fn get_address_info(&self, swap_id: &str) -> Result<Option<SwapAddressInfo>, sqlx::Error> {
        sqlx::query_as::<_, SwapAddressInfo>(
            "SELECT * FROM swap_address_info WHERE swap_id = ?"
        )
        .bind(swap_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Update payout status
    pub async fn mark_payout_completed(
        &self,
        swap_id: &str,
        tx_hash: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE swap_address_info 
            SET status = 'success', 
                payout_tx_hash = ?,
                broadcast_at = NOW(),
                confirmed_at = NOW()
            WHERE swap_id = ?
            "#
        )
        .bind(tx_hash)
        .bind(swap_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
