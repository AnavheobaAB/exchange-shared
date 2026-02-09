use crate::modules::wallet::crud::WalletCrud;
use crate::modules::wallet::schema::{GenerateAddressRequest, WalletAddressResponse, PayoutRequest, PayoutResponse};
use super::derivation;
use super::signing::SigningService;

pub struct WalletManager {
    crud: WalletCrud,
    master_seed: String,
}

impl WalletManager {
    pub fn new(crud: WalletCrud, master_seed: String) -> Self {
        Self { crud, master_seed }
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

        // 3. Derive address based on chain/network
        let address = match req.network.to_lowercase().as_str() {
            "ethereum" | "polygon" | "bsc" | "arbitrum" | "optimism" => {
                derivation::derive_evm_address(&self.master_seed, index).await?
            }
            "bitcoin" => {
                derivation::derive_btc_address(&self.master_seed, index).await?
            }
            "solana" => {
                derivation::derive_solana_address(&self.master_seed, index).await?
            }
            _ => return Err(format!("Unsupported network: {}", req.network)),
        };

        // 4. Save to DB
        self.crud.save_address_info(&req.swap_id, &address, index, &req.network).await
            .map_err(|e: sqlx::Error| format!("Failed to save address info: {}", e))?;

        Ok(WalletAddressResponse {
            address,
            address_index: index,
            swap_id: req.swap_id,
        })
    }

    /// Orchestrate a payout to the user with idempotency protection
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

        // 3. Derive private key
        let private_key = derivation::derive_evm_key(&self.master_seed).await?; 

        // 4. Build and Sign Transaction (Mocked for now)
        let mock_tx = crate::modules::wallet::schema::EvmTransaction {
            to_address: info.recipient_address,
            amount: info.payout_amount.unwrap_or(0.0),
            token: "ETH".to_string(),
            chain_id: 1,
            nonce: 0,
            gas_price: 20000000000,
        };

        let signature = SigningService::sign_evm_transaction(&private_key, &mock_tx)?;

        // 5. Broadcast
        let tx_hash = format!("0x_broadcasted_{}", signature.chars().take(10).collect::<String>());

        // 6. Update DB with tx_hash AND status
        self.crud.mark_payout_completed(&req.swap_id, &tx_hash).await
            .map_err(|e: sqlx::Error| e.to_string())?;

        Ok(PayoutResponse {
            tx_hash,
            amount: info.payout_amount.unwrap_or(0.0),
            status: crate::modules::wallet::model::PayoutStatus::Success,
        })
    }
}