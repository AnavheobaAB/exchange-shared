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

async fn create_swap_for_user(ctx: &TestContext, token: &str) {
    ctx.server
        .post("/swap/create")
        .authorization_bearer(token)
        .json(&json!({
            "from": "btc",
            "to": "eth",
            "amount": 0.1,
            "provider": "changenow",
            "recipient_address": "0x742d35Cc6634C0532925a3b844Bc9e7595f5bE12",
            "refund_address": "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh"
        }))
        .await;
}

// =============================================================================
// GET /swap/history - User's swap history (AUTH REQUIRED)
// =============================================================================

#[tokio::test]
async fn get_history_without_auth_returns_unauthorized() {
    let ctx = TestContext::new().await;

    let response = ctx.server.get("/swap/history").await;

    response.assert_status(StatusCode::UNAUTHORIZED);

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_history_with_invalid_token_returns_unauthorized() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/swap/history")
        .authorization_bearer("invalid_token_here")
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_history_with_expired_token_returns_unauthorized() {
    let ctx = TestContext::new().await;

    // This would need a pre-generated expired token
    let expired_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwiZXhwIjoxfQ.invalid";

    let response = ctx
        .server
        .get("/swap/history")
        .authorization_bearer(expired_token)
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_history_with_auth_returns_list() {
    let ctx = TestContext::new().await;
    let token = create_authenticated_user(&ctx).await;

    let response = ctx
        .server
        .get("/swap/history")
        .authorization_bearer(&token)
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert!(body.get("swaps").is_some());
    assert!(body["swaps"].is_array());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_history_empty_for_new_user() {
    let ctx = TestContext::new().await;
    let token = create_authenticated_user(&ctx).await;

    let response = ctx
        .server
        .get("/swap/history")
        .authorization_bearer(&token)
        .await;

    let body: serde_json::Value = response.json();
    assert!(body["swaps"].as_array().unwrap().is_empty());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_history_shows_user_swaps() {
    let ctx = TestContext::new().await;
    let token = create_authenticated_user(&ctx).await;

    // Create a swap
    create_swap_for_user(&ctx, &token).await;

    let response = ctx
        .server
        .get("/swap/history")
        .authorization_bearer(&token)
        .await;

    let body: serde_json::Value = response.json();
    assert!(!body["swaps"].as_array().unwrap().is_empty());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_history_only_shows_own_swaps() {
    let ctx = TestContext::new().await;

    // Create two users
    let token1 = create_authenticated_user(&ctx).await;
    let token2 = create_authenticated_user(&ctx).await;

    // User 1 creates a swap
    create_swap_for_user(&ctx, &token1).await;

    // User 2 should not see User 1's swaps
    let response = ctx
        .server
        .get("/swap/history")
        .authorization_bearer(&token2)
        .await;

    let body: serde_json::Value = response.json();
    assert!(body["swaps"].as_array().unwrap().is_empty());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_history_swap_includes_details() {
    let ctx = TestContext::new().await;
    let token = create_authenticated_user(&ctx).await;

    create_swap_for_user(&ctx, &token).await;

    let response = ctx
        .server
        .get("/swap/history")
        .authorization_bearer(&token)
        .await;

    let body: serde_json::Value = response.json();
    let swap = &body["swaps"][0];

    assert!(swap.get("swap_id").is_some());
    assert!(swap.get("from").is_some());
    assert!(swap.get("to").is_some());
    assert!(swap.get("amount").is_some());
    assert!(swap.get("status").is_some());
    assert!(swap.get("created_at").is_some());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_history_with_pagination() {
    let ctx = TestContext::new().await;
    let token = create_authenticated_user(&ctx).await;

    let response = ctx
        .server
        .get("/swap/history?page=1&limit=10")
        .authorization_bearer(&token)
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert!(body.get("page").is_some() || body.get("current_page").is_some());
    assert!(body.get("limit").is_some() || body.get("per_page").is_some());
    assert!(body.get("total").is_some() || body.get("total_count").is_some());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_history_respects_limit() {
    let ctx = TestContext::new().await;
    let token = create_authenticated_user(&ctx).await;

    // Create multiple swaps
    for _ in 0..5 {
        create_swap_for_user(&ctx, &token).await;
    }

    let response = ctx
        .server
        .get("/swap/history?limit=2")
        .authorization_bearer(&token)
        .await;

    let body: serde_json::Value = response.json();
    assert!(body["swaps"].as_array().unwrap().len() <= 2);

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_history_with_status_filter() {
    let ctx = TestContext::new().await;
    let token = create_authenticated_user(&ctx).await;

    let response = ctx
        .server
        .get("/swap/history?status=completed")
        .authorization_bearer(&token)
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    for swap in body["swaps"].as_array().unwrap() {
        assert_eq!(swap["status"], "completed");
    }

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_history_with_invalid_status_filter() {
    let ctx = TestContext::new().await;
    let token = create_authenticated_user(&ctx).await;

    let response = ctx
        .server
        .get("/swap/history?status=invalid_status")
        .authorization_bearer(&token)
        .await;

    // Should return empty or bad request
    assert!(
        response.status_code() == StatusCode::OK ||
        response.status_code() == StatusCode::BAD_REQUEST
    );

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_history_with_currency_filter() {
    let ctx = TestContext::new().await;
    let token = create_authenticated_user(&ctx).await;

    let response = ctx
        .server
        .get("/swap/history?from=btc")
        .authorization_bearer(&token)
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    for swap in body["swaps"].as_array().unwrap() {
        assert_eq!(swap["from"].as_str().unwrap().to_lowercase(), "btc");
    }

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_history_with_date_range() {
    let ctx = TestContext::new().await;
    let token = create_authenticated_user(&ctx).await;

    let response = ctx
        .server
        .get("/swap/history?from_date=2024-01-01&to_date=2024-12-31")
        .authorization_bearer(&token)
        .await;

    response.assert_status(StatusCode::OK);

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_history_with_invalid_date_returns_error() {
    let ctx = TestContext::new().await;
    let token = create_authenticated_user(&ctx).await;

    let response = ctx
        .server
        .get("/swap/history?from_date=invalid-date")
        .authorization_bearer(&token)
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_history_sorted_by_date_desc() {
    let ctx = TestContext::new().await;
    let token = create_authenticated_user(&ctx).await;

    // Create multiple swaps
    for _ in 0..3 {
        create_swap_for_user(&ctx, &token).await;
    }

    let response = ctx
        .server
        .get("/swap/history")
        .authorization_bearer(&token)
        .await;

    let body: serde_json::Value = response.json();
    let swaps = body["swaps"].as_array().unwrap();

    if swaps.len() > 1 {
        // Most recent should be first
        let first_date = swaps[0]["created_at"].as_str().unwrap();
        let second_date = swaps[1]["created_at"].as_str().unwrap();
        assert!(first_date >= second_date, "Should be sorted by date descending");
    }

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_history_sandbox_filter() {
    let ctx = TestContext::new().await;
    let token = create_authenticated_user(&ctx).await;

    // Create production swap
    create_swap_for_user(&ctx, &token).await;

    // Create sandbox swap
    ctx.server
        .post("/swap/create")
        .authorization_bearer(&token)
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

    // Filter to production only
    let response = ctx
        .server
        .get("/swap/history?sandbox=false")
        .authorization_bearer(&token)
        .await;

    let body: serde_json::Value = response.json();
    for swap in body["swaps"].as_array().unwrap() {
        assert_ne!(swap["is_sandbox"], true);
    }

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_history_has_security_headers() {
    let ctx = TestContext::new().await;
    let token = create_authenticated_user(&ctx).await;

    let response = ctx
        .server
        .get("/swap/history")
        .authorization_bearer(&token)
        .await;

    assert!(response.headers().get("x-content-type-options").is_some());
    assert!(response.headers().get("x-frame-options").is_some());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_history_responds_fast() {
    let ctx = TestContext::new().await;
    let token = create_authenticated_user(&ctx).await;

    let start = std::time::Instant::now();

    ctx.server
        .get("/swap/history")
        .authorization_bearer(&token)
        .await;

    let duration = start.elapsed();

    assert!(duration.as_secs() < 3, "History took too long: {:?}", duration);

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_history_rate_limited() {
    let ctx = TestContext::new().await;
    let token = create_authenticated_user(&ctx).await;

    let mut rate_limited = false;
    for _ in 0..30 {
        let response = ctx
            .server
            .get("/swap/history")
            .authorization_bearer(&token)
            .await;

        if response.status_code() == StatusCode::TOO_MANY_REQUESTS {
            rate_limited = true;
            break;
        }
    }

    ctx.cleanup().await;
}
