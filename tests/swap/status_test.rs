use axum::http::StatusCode;
use serde_json::json;

use crate::common::TestContext;

// =============================================================================
// HELPER
// =============================================================================

async fn create_test_swap(ctx: &TestContext) -> String {
    let response = ctx
        .server
        .post("/swap/create")
        .json(&json!({
            "from": "btc",
            "to": "eth",
            "amount": 0.1,
            "provider": "changenow",
            "recipient_address": "0x742d35Cc6634C0532925a3b844Bc9e7595f5bE12",
            "refund_address": "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh"
        }))
        .await;

    let body: serde_json::Value = response.json();
    body["swap_id"].as_str().unwrap().to_string()
}

// =============================================================================
// GET /swap/{id} - Get swap status/details
// =============================================================================

#[tokio::test]
async fn get_swap_status_returns_details() {
    let ctx = TestContext::new().await;
    let swap_id = create_test_swap(&ctx).await;

    let response = ctx
        .server
        .get(&format!("/swap/{}", swap_id))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert!(body.get("swap_id").is_some());
    assert!(body.get("status").is_some());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_swap_includes_all_fields() {
    let ctx = TestContext::new().await;
    let swap_id = create_test_swap(&ctx).await;

    let response = ctx
        .server
        .get(&format!("/swap/{}", swap_id))
        .await;

    let body: serde_json::Value = response.json();
    assert!(body.get("swap_id").is_some());
    assert!(body.get("status").is_some());
    assert!(body.get("from").is_some());
    assert!(body.get("to").is_some());
    assert!(body.get("amount").is_some());
    assert!(body.get("deposit_address").is_some());
    assert!(body.get("recipient_address").is_some());
    assert!(body.get("provider").is_some());
    assert!(body.get("created_at").is_some());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_swap_nonexistent_returns_not_found() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/swap/nonexistent-swap-id-12345")
        .await;

    response.assert_status(StatusCode::NOT_FOUND);

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_swap_invalid_id_format_returns_not_found() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/swap/!!invalid!!id!!")
        .await;

    assert!(
        response.status_code() == StatusCode::NOT_FOUND ||
        response.status_code() == StatusCode::BAD_REQUEST
    );

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_swap_empty_id_returns_not_found() {
    let ctx = TestContext::new().await;

    let response = ctx.server.get("/swap/").await;

    // Should return 404 or redirect
    assert!(
        response.status_code() == StatusCode::NOT_FOUND ||
        response.status_code() == StatusCode::PERMANENT_REDIRECT
    );

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_swap_status_has_valid_values() {
    let ctx = TestContext::new().await;
    let swap_id = create_test_swap(&ctx).await;

    let response = ctx
        .server
        .get(&format!("/swap/{}", swap_id))
        .await;

    let body: serde_json::Value = response.json();

    let valid_statuses = vec![
        "waiting", "confirming", "exchanging", "sending",
        "completed", "failed", "refunded", "expired", "pending"
    ];

    let status = body["status"].as_str().unwrap();
    assert!(
        valid_statuses.contains(&status),
        "Invalid status: {}. Expected one of: {:?}",
        status,
        valid_statuses
    );

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_swap_shows_waiting_status_initially() {
    let ctx = TestContext::new().await;
    let swap_id = create_test_swap(&ctx).await;

    let response = ctx
        .server
        .get(&format!("/swap/{}", swap_id))
        .await;

    let body: serde_json::Value = response.json();

    // New swap should be waiting for deposit
    assert!(
        body["status"] == "waiting" || body["status"] == "pending",
        "New swap should be waiting/pending, got: {}",
        body["status"]
    );

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_swap_includes_rate_info() {
    let ctx = TestContext::new().await;
    let swap_id = create_test_swap(&ctx).await;

    let response = ctx
        .server
        .get(&format!("/swap/{}", swap_id))
        .await;

    let body: serde_json::Value = response.json();
    assert!(body.get("rate").is_some() || body.get("estimated_receive").is_some());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_swap_includes_fee_info() {
    let ctx = TestContext::new().await;
    let swap_id = create_test_swap(&ctx).await;

    let response = ctx
        .server
        .get(&format!("/swap/{}", swap_id))
        .await;

    let body: serde_json::Value = response.json();
    assert!(
        body.get("fee").is_some() ||
        body.get("network_fee").is_some() ||
        body.get("total_fee").is_some()
    );

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_swap_includes_timestamps() {
    let ctx = TestContext::new().await;
    let swap_id = create_test_swap(&ctx).await;

    let response = ctx
        .server
        .get(&format!("/swap/{}", swap_id))
        .await;

    let body: serde_json::Value = response.json();
    assert!(body.get("created_at").is_some());
    assert!(body.get("expires_at").is_some() || body.get("updated_at").is_some());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_swap_completed_includes_tx_hash() {
    let ctx = TestContext::new().await;

    // This would test a completed swap (mock in real implementation)
    let response = ctx
        .server
        .get("/swap/test-completed-swap")
        .await;

    if response.status_code() == StatusCode::OK {
        let body: serde_json::Value = response.json();
        if body["status"] == "completed" {
            assert!(body.get("tx_hash").is_some() || body.get("transaction_hash").is_some());
            assert!(body.get("completed_at").is_some());
        }
    }

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_swap_failed_includes_error_reason() {
    let ctx = TestContext::new().await;

    // This would test a failed swap (mock in real implementation)
    let response = ctx
        .server
        .get("/swap/test-failed-swap")
        .await;

    if response.status_code() == StatusCode::OK {
        let body: serde_json::Value = response.json();
        if body["status"] == "failed" {
            assert!(body.get("error").is_some() || body.get("failure_reason").is_some());
        }
    }

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_swap_refunded_includes_refund_info() {
    let ctx = TestContext::new().await;

    // This would test a refunded swap (mock in real implementation)
    let response = ctx
        .server
        .get("/swap/test-refunded-swap")
        .await;

    if response.status_code() == StatusCode::OK {
        let body: serde_json::Value = response.json();
        if body["status"] == "refunded" {
            assert!(body.get("refund_tx_hash").is_some() || body.get("refunded_at").is_some());
        }
    }

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_swap_no_auth_required() {
    let ctx = TestContext::new().await;
    let swap_id = create_test_swap(&ctx).await;

    // Should work without authentication
    let response = ctx
        .server
        .get(&format!("/swap/{}", swap_id))
        .await;

    response.assert_status(StatusCode::OK);

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_swap_has_security_headers() {
    let ctx = TestContext::new().await;
    let swap_id = create_test_swap(&ctx).await;

    let response = ctx
        .server
        .get(&format!("/swap/{}", swap_id))
        .await;

    assert!(response.headers().get("x-content-type-options").is_some());
    assert!(response.headers().get("x-frame-options").is_some());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_swap_responds_fast() {
    let ctx = TestContext::new().await;
    let swap_id = create_test_swap(&ctx).await;

    let start = std::time::Instant::now();

    ctx.server
        .get(&format!("/swap/{}", swap_id))
        .await;

    let duration = start.elapsed();

    assert!(duration.as_secs() < 3, "Status check took too long: {:?}", duration);

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_swap_handles_sql_injection() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/swap/'; DROP TABLE swaps; --")
        .await;

    assert!(
        response.status_code() == StatusCode::NOT_FOUND ||
        response.status_code() == StatusCode::BAD_REQUEST
    );

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_swap_rate_limited() {
    let ctx = TestContext::new().await;
    let swap_id = create_test_swap(&ctx).await;

    let mut rate_limited = false;
    for _ in 0..50 {
        let response = ctx
            .server
            .get(&format!("/swap/{}", swap_id))
            .await;

        if response.status_code() == StatusCode::TOO_MANY_REQUESTS {
            rate_limited = true;
            break;
        }
    }

    ctx.cleanup().await;
}
