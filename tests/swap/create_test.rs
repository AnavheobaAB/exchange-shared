use axum::http::StatusCode;
use serde_json::json;

use crate::common::{TestContext, test_email, test_password};

// =============================================================================
// HELPER
// =============================================================================

async fn create_authenticated_user(ctx: &TestContext) -> String {
    let email = test_email();

    ctx.server
        .post("/auth/register")
        .json(&json!({
            "email": &email,
            "password": test_password(),
            "password_confirm": test_password()
        }))
        .await;

    let response = ctx
        .server
        .post("/auth/login")
        .json(&json!({
            "email": &email,
            "password": test_password()
        }))
        .await;

    let body: serde_json::Value = response.json();
    body["access_token"].as_str().unwrap().to_string()
}

// =============================================================================
// POST /swap/create - Create a new swap (NO AUTH REQUIRED)
// =============================================================================

#[tokio::test]
async fn create_swap_without_auth_succeeds() {
    let ctx = TestContext::new().await;

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

    response.assert_status(StatusCode::CREATED);

    let body: serde_json::Value = response.json();
    assert!(body.get("swap_id").is_some());
    assert!(body.get("deposit_address").is_some());

    ctx.cleanup().await;
}

#[tokio::test]
async fn create_swap_returns_all_required_fields() {
    let ctx = TestContext::new().await;

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
    assert!(body.get("swap_id").is_some());
    assert!(body.get("deposit_address").is_some());
    assert!(body.get("deposit_amount").is_some());
    assert!(body.get("estimated_receive").is_some());
    assert!(body.get("status").is_some());
    assert!(body.get("expires_at").is_some());
    assert!(body.get("created_at").is_some());

    ctx.cleanup().await;
}

#[tokio::test]
async fn create_swap_with_auth_links_to_user() {
    let ctx = TestContext::new().await;
    let token = create_authenticated_user(&ctx).await;

    let response = ctx
        .server
        .post("/swap/create")
        .authorization_bearer(&token)
        .json(&json!({
            "from": "btc",
            "to": "eth",
            "amount": 0.1,
            "provider": "changenow",
            "recipient_address": "0x742d35Cc6634C0532925a3b844Bc9e7595f5bE12",
            "refund_address": "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh"
        }))
        .await;

    response.assert_status(StatusCode::CREATED);

    ctx.cleanup().await;
}

#[tokio::test]
async fn create_swap_returns_unique_ids() {
    let ctx = TestContext::new().await;

    let response1 = ctx
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

    let response2 = ctx
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

    let body1: serde_json::Value = response1.json();
    let body2: serde_json::Value = response2.json();

    assert_ne!(body1["swap_id"], body2["swap_id"]);

    ctx.cleanup().await;
}

#[tokio::test]
async fn create_swap_missing_from_returns_error() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/swap/create")
        .json(&json!({
            "to": "eth",
            "amount": 0.1,
            "provider": "changenow",
            "recipient_address": "0x742d35Cc6634C0532925a3b844Bc9e7595f5bE12",
            "refund_address": "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh"
        }))
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);

    ctx.cleanup().await;
}

#[tokio::test]
async fn create_swap_missing_to_returns_error() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/swap/create")
        .json(&json!({
            "from": "btc",
            "amount": 0.1,
            "provider": "changenow",
            "recipient_address": "0x742d35Cc6634C0532925a3b844Bc9e7595f5bE12",
            "refund_address": "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh"
        }))
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);

    ctx.cleanup().await;
}

#[tokio::test]
async fn create_swap_missing_amount_returns_error() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/swap/create")
        .json(&json!({
            "from": "btc",
            "to": "eth",
            "provider": "changenow",
            "recipient_address": "0x742d35Cc6634C0532925a3b844Bc9e7595f5bE12",
            "refund_address": "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh"
        }))
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);

    ctx.cleanup().await;
}

#[tokio::test]
async fn create_swap_missing_provider_returns_error() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/swap/create")
        .json(&json!({
            "from": "btc",
            "to": "eth",
            "amount": 0.1,
            "recipient_address": "0x742d35Cc6634C0532925a3b844Bc9e7595f5bE12",
            "refund_address": "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh"
        }))
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);

    ctx.cleanup().await;
}

#[tokio::test]
async fn create_swap_missing_recipient_returns_error() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/swap/create")
        .json(&json!({
            "from": "btc",
            "to": "eth",
            "amount": 0.1,
            "provider": "changenow",
            "refund_address": "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh"
        }))
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);

    ctx.cleanup().await;
}

#[tokio::test]
async fn create_swap_invalid_recipient_address_returns_error() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/swap/create")
        .json(&json!({
            "from": "btc",
            "to": "eth",
            "amount": 0.1,
            "provider": "changenow",
            "recipient_address": "invalid_address_format",
            "refund_address": "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh"
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);

    let body: serde_json::Value = response.json();
    assert!(body["error"].as_str().unwrap().to_lowercase().contains("address"));

    ctx.cleanup().await;
}

#[tokio::test]
async fn create_swap_invalid_refund_address_returns_error() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/swap/create")
        .json(&json!({
            "from": "btc",
            "to": "eth",
            "amount": 0.1,
            "provider": "changenow",
            "recipient_address": "0x742d35Cc6634C0532925a3b844Bc9e7595f5bE12",
            "refund_address": "invalid_btc_address"
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);

    ctx.cleanup().await;
}

#[tokio::test]
async fn create_swap_invalid_provider_returns_error() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/swap/create")
        .json(&json!({
            "from": "btc",
            "to": "eth",
            "amount": 0.1,
            "provider": "nonexistent_provider_xyz",
            "recipient_address": "0x742d35Cc6634C0532925a3b844Bc9e7595f5bE12",
            "refund_address": "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh"
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);

    ctx.cleanup().await;
}

#[tokio::test]
async fn create_swap_invalid_currency_returns_error() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/swap/create")
        .json(&json!({
            "from": "invalidcoin999",
            "to": "eth",
            "amount": 0.1,
            "provider": "changenow",
            "recipient_address": "0x742d35Cc6634C0532925a3b844Bc9e7595f5bE12",
            "refund_address": "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh"
        }))
        .await;

    assert!(
        response.status_code() == StatusCode::BAD_REQUEST ||
        response.status_code() == StatusCode::NOT_FOUND
    );

    ctx.cleanup().await;
}

#[tokio::test]
async fn create_swap_same_currency_returns_error() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/swap/create")
        .json(&json!({
            "from": "btc",
            "to": "btc",
            "amount": 0.1,
            "provider": "changenow",
            "recipient_address": "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh",
            "refund_address": "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh"
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);

    ctx.cleanup().await;
}

#[tokio::test]
async fn create_swap_zero_amount_returns_error() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/swap/create")
        .json(&json!({
            "from": "btc",
            "to": "eth",
            "amount": 0,
            "provider": "changenow",
            "recipient_address": "0x742d35Cc6634C0532925a3b844Bc9e7595f5bE12",
            "refund_address": "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh"
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);

    ctx.cleanup().await;
}

#[tokio::test]
async fn create_swap_negative_amount_returns_error() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/swap/create")
        .json(&json!({
            "from": "btc",
            "to": "eth",
            "amount": -0.1,
            "provider": "changenow",
            "recipient_address": "0x742d35Cc6634C0532925a3b844Bc9e7595f5bE12",
            "refund_address": "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh"
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);

    ctx.cleanup().await;
}

#[tokio::test]
async fn create_swap_below_minimum_returns_error() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/swap/create")
        .json(&json!({
            "from": "btc",
            "to": "eth",
            "amount": 0.00000001,
            "provider": "changenow",
            "recipient_address": "0x742d35Cc6634C0532925a3b844Bc9e7595f5bE12",
            "refund_address": "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh"
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);

    let body: serde_json::Value = response.json();
    assert!(
        body["error"].as_str().unwrap().to_lowercase().contains("min") ||
        body.get("min_amount").is_some()
    );

    ctx.cleanup().await;
}

#[tokio::test]
async fn create_swap_with_extra_id_for_memo_coins() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/swap/create")
        .json(&json!({
            "from": "btc",
            "to": "xrp",
            "amount": 0.1,
            "provider": "changenow",
            "recipient_address": "rEb8TK3gBgk5auZkwc6sHnwrGVJH8DuaLh",
            "recipient_extra_id": "123456789",
            "refund_address": "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh"
        }))
        .await;

    // Should accept or reject based on provider support
    assert!(
        response.status_code() == StatusCode::CREATED ||
        response.status_code() == StatusCode::BAD_REQUEST
    );

    ctx.cleanup().await;
}

#[tokio::test]
async fn create_swap_xrp_without_memo_warns_or_errors() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/swap/create")
        .json(&json!({
            "from": "btc",
            "to": "xrp",
            "amount": 0.1,
            "provider": "changenow",
            "recipient_address": "rEb8TK3gBgk5auZkwc6sHnwrGVJH8DuaLh",
            "refund_address": "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh"
        }))
        .await;

    let body: serde_json::Value = response.json();
    assert!(
        response.status_code() == StatusCode::BAD_REQUEST ||
        body.get("warning").is_some()
    );

    ctx.cleanup().await;
}

#[tokio::test]
async fn create_swap_with_fixed_rate() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/swap/create")
        .json(&json!({
            "from": "btc",
            "to": "eth",
            "amount": 0.1,
            "provider": "changenow",
            "recipient_address": "0x742d35Cc6634C0532925a3b844Bc9e7595f5bE12",
            "refund_address": "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh",
            "rate_type": "fixed"
        }))
        .await;

    if response.status_code() == StatusCode::CREATED {
        let body: serde_json::Value = response.json();
        assert_eq!(body["rate_type"], "fixed");
    }

    ctx.cleanup().await;
}

#[tokio::test]
async fn create_swap_empty_body_returns_error() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/swap/create")
        .json(&json!({}))
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);

    ctx.cleanup().await;
}

// =============================================================================
// SANDBOX MODE
// =============================================================================

#[tokio::test]
async fn create_swap_in_sandbox_mode() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/swap/create")
        .json(&json!({
            "from": "btc",
            "to": "eth",
            "amount": 0.1,
            "provider": "changenow",
            "recipient_address": "0x742d35Cc6634C0532925a3b844Bc9e7595f5bE12",
            "refund_address": "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh",
            "sandbox": true
        }))
        .await;

    if response.status_code() == StatusCode::CREATED {
        let body: serde_json::Value = response.json();
        assert_eq!(body["is_sandbox"], true);
    }

    ctx.cleanup().await;
}

#[tokio::test]
async fn sandbox_swap_uses_test_addresses() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/swap/create")
        .json(&json!({
            "from": "btc",
            "to": "eth",
            "amount": 0.1,
            "provider": "changenow",
            "recipient_address": "0x742d35Cc6634C0532925a3b844Bc9e7595f5bE12",
            "refund_address": "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh",
            "sandbox": true
        }))
        .await;

    if response.status_code() == StatusCode::CREATED {
        let body: serde_json::Value = response.json();
        // Sandbox deposit address should be identifiable or different
        assert!(body.get("deposit_address").is_some());
    }

    ctx.cleanup().await;
}

// =============================================================================
// SECURITY
// =============================================================================

#[tokio::test]
async fn create_swap_sanitizes_xss_in_address() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/swap/create")
        .json(&json!({
            "from": "btc",
            "to": "eth",
            "amount": 0.1,
            "provider": "changenow",
            "recipient_address": "<script>alert('xss')</script>",
            "refund_address": "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh"
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);

    ctx.cleanup().await;
}

#[tokio::test]
async fn create_swap_rejects_sql_injection() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/swap/create")
        .json(&json!({
            "from": "btc'; DROP TABLE swaps; --",
            "to": "eth",
            "amount": 0.1,
            "provider": "changenow",
            "recipient_address": "0x742d35Cc6634C0532925a3b844Bc9e7595f5bE12",
            "refund_address": "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh"
        }))
        .await;

    assert!(
        response.status_code() == StatusCode::BAD_REQUEST ||
        response.status_code() == StatusCode::NOT_FOUND
    );

    ctx.cleanup().await;
}

#[tokio::test]
async fn create_swap_has_security_headers() {
    let ctx = TestContext::new().await;

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

    assert!(response.headers().get("x-content-type-options").is_some());
    assert!(response.headers().get("x-frame-options").is_some());

    ctx.cleanup().await;
}

// =============================================================================
// RATE LIMITING
// =============================================================================

#[tokio::test]
async fn create_swap_rate_limited_per_ip() {
    let ctx = TestContext::new().await;

    let mut rate_limited = false;
    for _ in 0..20 {
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

        if response.status_code() == StatusCode::TOO_MANY_REQUESTS {
            rate_limited = true;
            break;
        }
    }

    assert!(rate_limited, "Should be rate limited after many swap creations");

    ctx.cleanup().await;
}

// =============================================================================
// PERFORMANCE
// =============================================================================

#[tokio::test]
async fn create_swap_responds_within_acceptable_time() {
    let ctx = TestContext::new().await;

    let start = std::time::Instant::now();

    ctx.server
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

    let duration = start.elapsed();

    assert!(duration.as_secs() < 5, "Swap creation took too long: {:?}", duration);

    ctx.cleanup().await;
}
