use std::time::Duration;
use sqlx::{MySql, Pool};
use crate::modules::monitor::crud::MonitorCrud;
use crate::services::trocador::TrocadorClient;
use crate::services::wallet::manager::WalletManager;
use crate::services::wallet::rpc::HttpRpcClient;
use crate::services::redis_cache::RedisService;
use crate::modules::monitor::model::PollingState;

use crate::services::monitor::strategy::PollingStrategy;

pub struct MonitorEngine {
    db: Pool<MySql>,
    redis: RedisService,
    master_seed: String,
    strategy: PollingStrategy,
}

impl MonitorEngine {
    pub fn new(db: Pool<MySql>, redis: RedisService, master_seed: String) -> Self {
        // Initialize strategy with default costs:
        // Cp = 1.0 (one poll)
        // Cd = 0.05 (20 seconds of delay equals cost of one poll)
        let strategy = PollingStrategy::new(1.0, 0.05);
        Self { db, redis, master_seed, strategy }
    }

    /// Start the background polling loop
    pub async fn run(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        let monitor_crud = MonitorCrud::new(self.db.clone());

        loop {
            interval.tick().await;
            
            if let Ok(polls) = monitor_crud.get_due_polls().await {
                for poll in polls {
                    let _ = self.process_poll(poll).await;
                }
            }
        }
    }

    /// Process a single swap poll
    pub async fn process_poll(&self, state: PollingState) -> Result<(), String> {
        // 1. Distributed Lock to prevent concurrency
        let lock_key = format!("lock:monitor:{}", state.swap_id);
        if !self.redis.try_lock(&lock_key, 30).await.unwrap_or(false) {
            return Ok(());
        }

        // 2. Fetch Swap Details
        let swap = sqlx::query!(
            "SELECT provider_swap_id, status, created_at FROM swaps WHERE id = ?",
            state.swap_id
        )
        .fetch_optional(&self.db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Swap not found".to_string())?;

        let provider_swap_id = swap.provider_swap_id
            .ok_or_else(|| "No provider trade ID".to_string())?;

        // 3. Check Trocador Status
        let api_key = std::env::var("TROCADOR_API_KEY").unwrap_or_default();
        let client = TrocadorClient::new(api_key);
        
        let trocador_trade = client.get_trade_status(&provider_swap_id).await
            .map_err(|e| e.to_string())?;

        // 4. THE BRIDGE: Trigger Payout if finished
        let final_status: String;
        let next_poll_secs: u64;

        if trocador_trade.status == "finished" {
            tracing::info!("Swap {} finished on Trocador. Executing bridge payout.", state.swap_id);
            
            let wallet_crud = crate::modules::wallet::crud::WalletCrud::new(self.db.clone());
            let rpc_url = std::env::var("ETH_RPC_URL").unwrap_or_else(|_| "http://localhost:8545".to_string());
            let provider = std::sync::Arc::new(HttpRpcClient::new(rpc_url));
            let wallet_manager = WalletManager::new(wallet_crud, self.master_seed.clone(), provider);
            
            match wallet_manager.process_payout(crate::modules::wallet::schema::PayoutRequest {
                swap_id: state.swap_id.clone(),
            }).await {
                Ok(_) => {
                    final_status = "completed".to_string();
                    next_poll_secs = 3600 * 24; // Stop polling (once a day for cleanup)
                    
                    sqlx::query!("UPDATE swaps SET status = 'completed', updated_at = NOW() WHERE id = ?", state.swap_id)
                        .execute(&self.db).await.ok();
                }
                Err(e) => {
                    tracing::error!("Bridge payout failed for {}: {}", state.swap_id, e);
                    final_status = "payout_failed".to_string();
                    next_poll_secs = 60; // Retry soon
                }
            }
        } else {
            final_status = trocador_trade.status.clone();
            // Update internal swap status if changed (e.g. 'confirming' -> 'sending')
            if trocador_trade.status != swap.status {
                sqlx::query!("UPDATE swaps SET status = ?, updated_at = NOW() WHERE id = ?", trocador_trade.status, state.swap_id)
                    .execute(&self.db).await.ok();
            }
            
            // 5. OPTIMAL POLLING LOGIC
            let elapsed = chrono::Utc::now() - swap.created_at;
            let elapsed_secs = elapsed.num_seconds().max(0) as u64;
            next_poll_secs = self.strategy.calculate_next_interval(elapsed_secs).as_secs();
        }

        // 6. Update Monitoring State
        let monitor_crud = MonitorCrud::new(self.db.clone());
        let _ = monitor_crud.update_poll_result(&state.swap_id, &final_status, next_poll_secs).await;

        Ok(())
    }
}