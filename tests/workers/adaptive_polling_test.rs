// =============================================================================
// INTEGRATION TESTS - ADAPTIVE POLLING
// Verifies interval decay logic (Urgent vs Stale)
// =============================================================================

#[path = "../common/mod.rs"]
mod common;

use common::TestContext;
use chrono::Utc;
use exchange_shared::services::monitor::MonitorEngine;
use exchange_shared::modules::monitor::model::PollingState;

#[tokio::test]
async fn test_polling_interval_decay_logic() {
    let ctx = TestContext::new().await;
    let engine = MonitorEngine::new(ctx.db.clone(), ctx.redis.clone(), "seed".to_string());
    
    // 1. Fresh state: poll_count < 10 -> 15s
    let state_fresh = PollingState {
        swap_id: "test".into(),
        last_polled_at: None,
        next_poll_at: Utc::now(),
        poll_count: 5,
        last_status: "waiting".into(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    assert_eq!(engine.calculate_next_interval(&state_fresh), 15);

    // 2. Mid state: 10 <= poll_count < 50 -> 60s
    let state_mid = PollingState {
        poll_count: 25,
        ..state_fresh.clone()
    };
    assert_eq!(engine.calculate_next_interval(&state_mid), 60);

    // 3. Stale state: poll_count >= 50 -> 300s
    let state_stale = PollingState {
        poll_count: 100,
        ..state_fresh.clone()
    };
    assert_eq!(engine.calculate_next_interval(&state_stale), 300);

    println!("✅ Adaptive interval logic verified");
}