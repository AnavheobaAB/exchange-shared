# Architecture Analysis: Blockchain Listener Implementation

## Current Flow (What You Have)

```
1. User requests swap: BTC → ETH
2. Platform calls /swap/rates → Gets quotes with commission already deducted
3. Platform generates YOUR_ETH_ADDRESS (HD wallet, index N)
4. Platform calls Trocador /new_trade with:
   - recipient_address: YOUR_ETH_ADDRESS (not user's!)
   - User sees: estimated_receive (already has YOUR commission deducted)
5. Trocador returns: deposit_address (where user sends BTC)
6. User sends BTC → Trocador's deposit_address
7. Trocador swaps BTC → ETH
8. Trocador sends ETH → YOUR_ETH_ADDRESS
9. [CURRENT] Monitor polls Trocador status API
10. [CURRENT] When status="finished" → trigger payout
11. [MISSING] Verify funds actually arrived on blockchain
12. [MISSING] Check balance matches expected amount
13. Platform sends ETH → User's actual address
```

## The Problem

**Your current code at step 9-10:**
- Polls Trocador API: `GET /trade?id=xxx`
- When `status="finished"` → immediately triggers payout
- **RISK:** You're trusting Trocador without verifying blockchain!

**What if:**
- Trocador says "finished" but didn't send funds?
- Trocador sent wrong amount?
- Transaction is still pending (not confirmed)?
- Network congestion delays the transaction?

**Result:** You could send user funds BEFORE you actually received them = LOSS OF MONEY!

## The Correct Architecture

### Phase 1: Add Blockchain Verification (IMMEDIATE FIX)

Keep Trocador polling BUT add blockchain verification before payout:

```rust
// In monitor/engine.rs
if trocador_trade.status == "finished" {
    // DON'T immediately payout!
    // First, verify funds on blockchain
    
    let our_address = get_our_address_for_swap(swap_id);
    let expected_amount = get_expected_amount(swap_id);
    
    // Check blockchain balance
    let actual_balance = rpc_client.get_balance(our_address).await?;
    
    if actual_balance >= expected_amount {
        // Funds confirmed! Now safe to payout
        trigger_payout(swap_id).await?;
    } else {
        // Funds not arrived yet, keep polling
        log::warn!("Trocador says finished but funds not on chain yet");
    }
}
```

### Phase 2: Blockchain Event Listeners (OPTIMAL SOLUTION)

Replace polling with event-driven architecture:

```rust
// New module: src/services/blockchain/listener.rs

pub struct BlockchainListener {
    rpc_clients: HashMap<String, Arc<dyn BlockchainProvider>>,
    db: Pool<MySql>,
}

impl BlockchainListener {
    pub async fn monitor_addresses(&self) {
        loop {
            // Get all pending swaps with our addresses
            let pending = self.get_pending_swaps().await;
            
            for swap in pending {
                // Check if funds arrived
                let balance = self.check_address_balance(&swap).await;
                
                if balance >= swap.expected_amount {
                    // Funds detected! Trigger payout
                    self.trigger_payout(swap).await;
                }
            }
            
            tokio::time::sleep(Duration::from_secs(30)).await;
        }
    }
}
```

## Detailed Implementation Plan

### Step 1: Fix Immediate Test Failures (30 minutes)

**Problem:** Tests failing due to:
1. Missing providers in database
2. Rate limiting on Trocador API
3. Wrong test expectations

**Solution:**
```rust
// 1. Ensure providers sync before tests
// 2. Add delays between API calls
// 3. Adjust test expectations to match real API
```

### Step 2: Add Blockchain Verification to Payouts (1 hour)

**File:** `src/services/wallet/manager.rs`

**Current code:**
```rust
pub async fn process_payout(&self, req: PayoutRequest) -> Result<PayoutResponse, String> {
    // Gets address info
    // Signs transaction
    // Broadcasts
    // ❌ NO VERIFICATION OF RECEIVED FUNDS!
}
```

**Fixed code:**
```rust
pub async fn process_payout(&self, req: PayoutRequest) -> Result<PayoutResponse, String> {
    let info = self.crud.get_address_info(&req.swap_id).await?;
    
    // ✅ STEP 1: Verify funds actually arrived on blockchain
    let actual_balance = self.provider.get_balance(&info.our_address).await?;
    
    // ✅ STEP 2: Get expected amount from swap
    let expected_amount = self.get_expected_amount(&req.swap_id).await?;
    
    // ✅ STEP 3: Verify balance matches (with tolerance for gas)
    if actual_balance < expected_amount * 0.99 {
        return Err(format!(
            "Insufficient balance: expected {}, got {}",
            expected_amount, actual_balance
        ));
    }
    
    // ✅ STEP 4: Use ACTUAL received amount (not estimated)
    let raw_received = actual_balance;
    
    // Calculate commission on actual amount
    let platform_fee = self.calculate_commission(raw_received);
    let final_payout = raw_received - platform_fee;
    
    // Sign and broadcast
    let tx_hash = self.sign_and_broadcast(final_payout, &info).await?;
    
    // Update DB with actual amounts
    self.crud.mark_payout_completed(&req.swap_id, &tx_hash, raw_received, platform_fee).await?;
    
    Ok(PayoutResponse { tx_hash, amount: final_payout, status: Success })
}
```

### Step 3: Update Monitor Engine (1 hour)

**File:** `src/services/monitor/engine.rs`

**Current code:**
```rust
if trocador_trade.status == "finished" {
    // ❌ Immediately triggers payout without verification
    wallet_manager.process_payout(swap_id).await?;
}
```

**Fixed code:**
```rust
if trocador_trade.status == "finished" {
    // ✅ Trocador says finished, but verify on blockchain first
    
    // Get our address for this swap
    let address_info = wallet_crud.get_address_info(&swap_id).await?;
    
    // Check blockchain balance
    let balance = rpc_client.get_balance(&address_info.our_address).await?;
    
    // Get expected amount from swap
    let expected = self.get_expected_trocador_amount(&swap_id).await?;
    
    if balance >= expected * 0.99 { // 1% tolerance
        tracing::info!("Funds confirmed on blockchain for swap {}", swap_id);
        
        // Now safe to trigger payout
        match wallet_manager.process_payout(PayoutRequest { swap_id }).await {
            Ok(_) => {
                final_status = "completed".to_string();
                next_poll_secs = 86400; // Stop polling
            }
            Err(e) => {
                tracing::error!("Payout failed: {}", e);
                final_status = "payout_failed".to_string();
                next_poll_secs = 300; // Retry in 5 min
            }
        }
    } else {
        // Trocador says finished but funds not on chain yet
        tracing::warn!(
            "Trocador finished but blockchain balance {} < expected {}",
            balance, expected
        );
        final_status = "awaiting_funds".to_string();
        next_poll_secs = 60; // Check again in 1 min
    }
}
```

### Step 4: Add Balance Checking Methods (30 minutes)

**File:** `src/modules/swap/crud.rs`

```rust
impl SwapCrud {
    /// Get the amount Trocador should have sent to us
    pub async fn get_expected_trocador_amount(&self, swap_id: &str) -> Result<f64, SwapError> {
        let swap: (f64, f64) = sqlx::query_as(
            "SELECT estimated_receive, platform_fee FROM swaps WHERE id = ?"
        )
        .bind(swap_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| SwapError::DatabaseError(e.to_string()))?;
        
        // Trocador sends us: user_amount + our_commission
        // Because we told them to send to OUR address
        Ok(swap.0 + swap.1)
    }
}
```

**File:** `src/modules/wallet/crud.rs`

```rust
impl WalletCrud {
    /// Update payout with actual received amounts
    pub async fn mark_payout_completed(
        &self,
        swap_id: &str,
        tx_hash: &str,
        actual_received: f64,
        commission_taken: f64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE swap_address_info 
            SET status = 'success', 
                payout_tx_hash = ?,
                payout_amount = ?,
                commission_rate = ?,
                broadcast_at = NOW(),
                confirmed_at = NOW()
            WHERE swap_id = ?
            "#
        )
        .bind(tx_hash)
        .bind(actual_received)
        .bind(commission_taken / actual_received)
        .bind(swap_id)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
}
```

### Step 5: Implement Blockchain Listener (Future - 4 hours)

**New file:** `src/services/blockchain/listener.rs`

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use sqlx::{MySql, Pool};
use crate::services::wallet::rpc::BlockchainProvider;

pub struct BlockchainListener {
    db: Pool<MySql>,
    providers: HashMap<String, Arc<dyn BlockchainProvider>>,
}

impl BlockchainListener {
    pub fn new(db: Pool<MySql>) -> Self {
        let mut providers = HashMap::new();
        
        // Initialize RPC clients for each chain
        providers.insert(
            "ethereum".to_string(),
            Arc::new(HttpRpcClient::new(env::var("ETH_RPC_URL").unwrap())) as Arc<dyn BlockchainProvider>
        );
        providers.insert(
            "polygon".to_string(),
            Arc::new(HttpRpcClient::new(env::var("POLYGON_RPC_URL").unwrap())) as Arc<dyn BlockchainProvider>
        );
        // Add more chains...
        
        Self { db, providers }
    }
    
    /// Main monitoring loop
    pub async fn run(&self) {
        let mut tick = interval(Duration::from_secs(30));
        
        loop {
            tick.tick().await;
            
            if let Err(e) = self.check_pending_swaps().await {
                tracing::error!("Listener error: {}", e);
            }
        }
    }
    
    /// Check all pending swaps for incoming funds
    async fn check_pending_swaps(&self) -> Result<(), String> {
        // Get swaps where Trocador status is "finished" but we haven't paid out yet
        let pending: Vec<(String, String, String, f64)> = sqlx::query_as(
            r#"
            SELECT s.id, sa.our_address, s.to_network, s.estimated_receive + s.platform_fee
            FROM swaps s
            JOIN swap_address_info sa ON s.id = sa.swap_id
            WHERE s.status IN ('sending', 'exchanging')
            AND sa.status = 'pending'
            "#
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| e.to_string())?;
        
        for (swap_id, our_address, network, expected_amount) in pending {
            if let Some(provider) = self.providers.get(&network) {
                match provider.get_balance(&our_address).await {
                    Ok(balance) if balance >= expected_amount * 0.99 => {
                        tracing::info!(
                            "✅ Funds detected for swap {}: {} (expected {})",
                            swap_id, balance, expected_amount
                        );
                        
                        // Trigger payout
                        self.trigger_payout(&swap_id).await?;
                    }
                    Ok(balance) => {
                        tracing::debug!(
                            "Swap {} balance: {} / {} (waiting...)",
                            swap_id, balance, expected_amount
                        );
                    }
                    Err(e) => {
                        tracing::error!("Failed to check balance for {}: {}", swap_id, e);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    async fn trigger_payout(&self, swap_id: &str) -> Result<(), String> {
        // Update swap status to trigger payout
        sqlx::query("UPDATE swaps SET status = 'funds_received' WHERE id = ?")
            .bind(swap_id)
            .execute(&self.db)
            .await
            .map_err(|e| e.to_string())?;
        
        // The monitor engine will pick this up and execute payout
        Ok(())
    }
}
```

## Database Schema Updates

### Add new status to swaps table

```sql
ALTER TABLE swaps MODIFY COLUMN status ENUM(
    'waiting', 
    'confirming', 
    'exchanging', 
    'sending', 
    'funds_received',  -- NEW: Trocador sent to us, verified on chain
    'completed',       -- User received funds
    'failed', 
    'refunded', 
    'expired'
) NOT NULL DEFAULT 'waiting';
```

### Update swap_address_info to track actual amounts

```sql
ALTER TABLE swap_address_info 
ADD COLUMN actual_received DOUBLE DEFAULT NULL AFTER payout_amount,
ADD COLUMN commission_taken DOUBLE DEFAULT NULL AFTER actual_received;
```

## Testing Strategy

### Unit Tests

```rust
#[tokio::test]
async fn test_payout_requires_blockchain_verification() {
    let mock_provider = MockBlockchainProvider {
        balance: 0.0, // No funds yet
    };
    
    let wallet_manager = WalletManager::new(crud, seed, Arc::new(mock_provider));
    
    let result = wallet_manager.process_payout(PayoutRequest { swap_id }).await;
    
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Insufficient balance"));
}

#[tokio::test]
async fn test_payout_succeeds_with_sufficient_balance() {
    let mock_provider = MockBlockchainProvider {
        balance: 1.5, // Sufficient funds
    };
    
    let wallet_manager = WalletManager::new(crud, seed, Arc::new(mock_provider));
    
    let result = wallet_manager.process_payout(PayoutRequest { swap_id }).await;
    
    assert!(result.is_ok());
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_end_to_end_swap_with_blockchain_verification() {
    // 1. Create swap
    let swap = create_swap(btc_to_eth).await;
    
    // 2. Simulate Trocador sending funds to our address
    fund_address(swap.our_address, 1.5).await;
    
    // 3. Monitor should detect funds
    monitor.check_pending_swaps().await;
    
    // 4. Verify payout was triggered
    let swap_status = get_swap_status(swap.id).await;
    assert_eq!(swap_status, "completed");
}
```

## Migration Path

### Week 1: Fix Tests + Add Verification
- ✅ Fix provider sync issues
- ✅ Add blockchain balance verification to payouts
- ✅ Update monitor to check blockchain before payout
- ✅ All tests passing

### Week 2: Optimize Monitoring
- ✅ Reduce Trocador API polling frequency
- ✅ Add caching for blockchain balance checks
- ✅ Implement retry logic for failed payouts

### Week 3: Blockchain Listeners
- ✅ Implement BlockchainListener service
- ✅ Replace Trocador polling with blockchain events
- ✅ Keep Trocador status as backup/fallback

### Week 4: Production Hardening
- ✅ Add webhook support from Trocador
- ✅ Implement multi-chain support
- ✅ Add monitoring/alerting for stuck swaps

## Summary

**Current State:**
- ❌ Trusting Trocador API without blockchain verification
- ❌ Risk of paying users before receiving funds
- ❌ Tests failing due to missing providers

**After Phase 1 (Immediate Fix):**
- ✅ Blockchain verification before payouts
- ✅ Safe from paying without receiving
- ✅ Tests passing

**After Phase 2 (Optimal):**
- ✅ Event-driven blockchain listeners
- ✅ No dependency on Trocador status API
- ✅ Real-time fund detection
- ✅ Trustless verification

**Key Insight:**
Your architecture is fundamentally correct (middleman model), but the implementation needs blockchain verification as the source of truth, not Trocador's status API.
