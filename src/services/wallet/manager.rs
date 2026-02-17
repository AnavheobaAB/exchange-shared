use std::sync::Arc;
use crate::modules::wallet::crud::WalletCrud;
use crate::modules::wallet::schema::{GenerateAddressRequest, WalletAddressResponse, PayoutRequest, PayoutResponse};
use super::derivation;
use super::signing::SigningService;
use super::rpc::BlockchainProvider;
use crate::services::pricing::{PricingContext, PricingStrategy, AdaptivePricingStrategy};

pub struct WalletManager {
    crud: WalletCrud,
    master_seed: String,
    provider: Arc<dyn BlockchainProvider>,
}

impl WalletManager {
    pub fn new(crud: WalletCrud, master_seed: String, provider: Arc<dyn BlockchainProvider>) -> Self {
        Self { crud, master_seed, provider }
    }

    /// High-level orchestrator to generate a new swap address
    pub async fn get_or_generate_address(
        &self,
        req: GenerateAddressRequest,
    ) -> Result<WalletAddressResponse, String> {
        // 1. Check if swap already has an address assigned in DB
        if let Ok(Some(existing)) = self.crud.get_address_info(&req.swap_id).await {
            return Ok(WalletAddressResponse {
                address: existing.our_address,
                address_index: existing.address_index,
                swap_id: existing.swap_id,
            });
        }

        // 2. Get next available HD index
        let index = self.crud.get_next_index().await
            .map_err(|e: sqlx::Error| format!("DB Error: {}", e))?;

        // 3. Use high-level dispatcher to derive address
        let address = derivation::derive_address(&self.master_seed, &req.ticker, &req.network, index).await?;

        // 4. Save to DB
        self.crud.save_address_info(
            &req.swap_id,
            &address,
            index,
            &req.network,
            &req.user_recipient_address,
            req.user_recipient_extra_id.as_deref(),
        ).await
            .map_err(|e: sqlx::Error| format!("Failed to save address info: {}", e))?;

        Ok(WalletAddressResponse {
            address,
            address_index: index,
            swap_id: req.swap_id,
        })
    }

    /// Orchestrate a payout to the user with idempotency and blockchain verification
    pub async fn process_payout(
        &self,
        req: PayoutRequest,
    ) -> Result<PayoutResponse, String> {
        // 1. Get address info and check for existing payout
        let info = self.crud.get_address_info(&req.swap_id).await
            .map_err(|e: sqlx::Error| e.to_string())?
            .ok_or_else(|| "No address info found for swap".to_string())?;

        // 2. IDEMPOTENCY CHECK: If already has tx_hash or status is success, return early
        if let Some(tx_hash) = info.payout_tx_hash {
            return Ok(PayoutResponse {
                tx_hash,
                amount: info.payout_amount.unwrap_or(0.0),
                status: crate::modules::wallet::model::PayoutStatus::Success,
            });
        }

        // 3. BLOCKCHAIN VERIFICATION: Check actual balance on chain
        let actual_balance = self.provider.get_balance(&info.our_address).await
            .map_err(|e| format!("Failed to get blockchain balance: {}", e))?;
        
        tracing::info!(
            "Swap {}: Blockchain balance check - Address: {}, Balance: {}",
            req.swap_id, info.our_address, actual_balance
        );
        
        // Minimum threshold check (must have at least some funds)
        if actual_balance < 0.0001 {
            return Err(format!(
                "Insufficient balance on blockchain: {} (address: {})",
                actual_balance, info.our_address
            ));
        }

        // 4. Prepare for Transaction Building
        // We need the SENDER address to get the correct nonce.
        // Assuming EVM for now based on previous mock.
        // TODO: Switch based on network (info.network or req context)
        let sender_address = derivation::derive_evm_address(&self.master_seed, info.address_index).await?;
        let private_key = derivation::derive_evm_key(&self.master_seed).await?; 

        // 5. Fetch Chain Data (Real RPC Calls)
        let nonce = self.provider.get_transaction_count(&sender_address).await
            .map_err(|e| format!("Failed to get nonce: {}", e))?;
            
        let gas_price = self.provider.get_gas_price().await
            .map_err(|e| format!("Failed to get gas price: {}", e))?;

        // 6. ALGORITHMIC PRICING: Calculate final payout after dynamic fee
        let pricing_strategy = AdaptivePricingStrategy::default();
        let raw_received = actual_balance; // Use ACTUAL balance from blockchain
        
        // Estimate gas cost in native token (Gas Price * typical limit 21000)
        let gas_limit = 21000.0;
        let estimated_gas_native = (gas_price as f64 * gas_limit) / 1_000_000_000_000_000_000.0;

        let ctx = PricingContext {
            amount_usd: raw_received, // Heuristic: raw amount as USD proxy for testing
            network_gas_cost_native: estimated_gas_native,
            provider_spread_percentage: 0.0, // No spread during payout phase
        };

        let (commission_rate, gas_floor) = pricing_strategy.calculate_fees(&ctx);
        
        let mut platform_fee = raw_received * commission_rate;
        if platform_fee < gas_floor {
            platform_fee = gas_floor;
        }

        let final_payout: f64 = (raw_received - platform_fee - estimated_gas_native).max(0.0);

        if final_payout <= 0.0 {
            return Err(format!(
                "Payout amount too small to cover fees: received={}, fee={}, gas={}",
                raw_received, platform_fee, estimated_gas_native
            ));
        }

        tracing::info!(
            "Swap {}: Payout calculation - Received: {}, Commission: {}, Gas: {}, Final: {}",
            req.swap_id, raw_received, platform_fee, estimated_gas_native, final_payout
        );

        // 7. Build and Sign Transaction
        let tx = crate::modules::wallet::schema::EvmTransaction {
            to_address: info.recipient_address,
            amount: final_payout,
            token: "ETH".to_string(), 
            chain_id: 1, 
            nonce,
            gas_price,
        };

        let signature = SigningService::sign_evm_transaction(&private_key, &tx)?;

        // 8. Broadcast Real Transaction
        let tx_hash = self.provider.send_raw_transaction(&signature).await
            .map_err(|e| format!("Failed to broadcast: {}", e))?;

        // 9. Update DB with tx_hash, actual amounts, AND status
        self.crud.mark_payout_completed(&req.swap_id, &tx_hash, raw_received, platform_fee).await
            .map_err(|e: sqlx::Error| e.to_string())?;

        Ok(PayoutResponse {
            tx_hash,
            amount: final_payout,
            status: crate::modules::wallet::model::PayoutStatus::Success,
        })
    }
}
