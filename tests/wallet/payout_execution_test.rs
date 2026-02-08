// =============================================================================
// INTEGRATION TESTS - PAYOUT EXECUTION
// Tests for transferring converted crypto to user's recipient address
// Flow: Trocador sends to us → We deduct commission → We send to user
// =============================================================================

#[path = "../common/mod.rs"]
mod common;

use exchange_shared::modules::wallet::crud::WalletCrud;
use exchange_shared::modules::wallet::schema::{GenerateAddressRequest, PayoutRequest};
use exchange_shared::services::wallet::manager::WalletManager;
use exchange_shared::modules::wallet::model::PayoutStatus;
use common::TestContext;
use uuid::Uuid;

// Helper to create a dummy swap in DB
async fn create_payout_ready_swap(db: &sqlx::Pool<sqlx::MySql>, swap_id: &str, recipient: &str, amount: f64) {
    sqlx::query(
        r#"
        INSERT INTO swaps (
            id, provider_id, from_currency, from_network, to_currency, to_network,
            amount, estimated_receive, rate, deposit_address, recipient_address, status
        )
        VALUES (?, 'changenow', 'BTC', 'bitcoin', 'ETH', 'ethereum', 0.1, ?, 15.0, 'dep_addr', ?, 'completed')
        "#
    )
    .bind(swap_id)
    .bind(amount)
    .bind(recipient)
    .execute(db)
    .await
    .expect("Failed to create payout ready swap");
}

// =============================================================================
// TEST 1: Commission Deduction During Payout
// Trocador gives us 1.0 ETH → We deduct commission and track it
// =============================================================================

#[tokio::test]
async fn test_commission_deduction_on_payout() {
    let ctx = TestContext::new().await;
    let seed_phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    
    let crud = WalletCrud::new(ctx.db.clone());
    let manager = WalletManager::new(crud, seed_phrase.to_string());
    
    let swap_id = Uuid::new_v4().to_string();
    let recipient = "0x742d35Cc6634C0532925a3b844Bc9e7595f5bE1234";
    let amount_from_trocador = 1.0;
    
    create_payout_ready_swap(&ctx.db, &swap_id, recipient, amount_from_trocador).await;
    
    // 1. Generate our receiving address first
    manager.get_or_generate_address(GenerateAddressRequest {
        swap_id: swap_id.clone(),
        ticker: "ETH".to_string(),
        network: "ethereum".to_string(),
    }).await.unwrap();
    
    // 2. Mock receiving funds by updating the payout_amount in DB
    // In real flow, the monitor service would do this
    sqlx::query("UPDATE swap_address_info SET payout_amount = ? WHERE swap_id = ?")
        .bind(amount_from_trocador * 0.99) // 1% commission deducted
        .bind(&swap_id)
        .execute(&ctx.db)
        .await
        .unwrap();
    
    // 3. Execute payout
    let res = manager.process_payout(PayoutRequest { swap_id: swap_id.clone() }).await.unwrap();
    
    assert_eq!(res.status, PayoutStatus::Success);
    assert!(!res.tx_hash.is_empty());
    assert_eq!(res.amount, 0.99);
    
    println!("✅ Commission deduction correct: {:.2} ETH to user", res.amount);
    ctx.cleanup().await;
}

// =============================================================================
// TEST 2: Payout Tracking Audit Trail
// Every payout is logged with status, recipient in the database
// =============================================================================

#[tokio::test]
async fn test_payout_audit_trail() {
    let ctx = TestContext::new().await;
    let seed_phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    
    let crud = WalletCrud::new(ctx.db.clone());
    let manager = WalletManager::new(crud, seed_phrase.to_string());
    
    let swap_id = Uuid::new_v4().to_string();
    create_payout_ready_swap(&ctx.db, &swap_id, "0x742d35Cc6634C0532925a3b844Bc9e7595f5bE12", 0.5).await;
    
    manager.get_or_generate_address(GenerateAddressRequest {
        swap_id: swap_id.clone(),
        ticker: "ETH".to_string(),
        network: "ethereum".to_string(),
    }).await.unwrap();
    
    // Execute payout
    manager.process_payout(PayoutRequest { swap_id: swap_id.clone() }).await.unwrap();
    
    // Verify status in DB
    let info = WalletCrud::new(ctx.db.clone()).get_address_info(&swap_id).await.unwrap().unwrap();
    assert_eq!(info.status, "success");
    assert!(info.payout_tx_hash.is_some());
    
    println!("✅ Payout audit trail maintained in DB");
    ctx.cleanup().await;
}