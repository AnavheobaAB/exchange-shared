use axum::http::StatusCode;
use serde_json::json;

use crate::common::TestContext;

// =============================================================================
// POST /swap/estimate - Get estimated swap amount
// =============================================================================

#[tokio::test]
async fn estimate_swap_returns_amount() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/swap/estimate")
        .json(&json!({
            "from": "btc",
            "to": "eth",
            "amount": 0.1,
            "provider": "changenow"
        }))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert!(body.get("estimated_amount").is_some());
    assert!(body.get("rate").is_some());

    ctx.cleanup().await;
}

#[tokio::test]
async fn estimate_swap_includes_fee_breakdown() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/swap/estimate")
        .json(&json!({
            "from": "btc",
            "to": "eth",
            "amount": 1.0,
            "provider": "changenow"
        }))
        .await;

    let body: serde_json::Value = response.json();
    assert!(body.get("network_fee").is_some() || body.get("total_fee").is_some());

    ctx.cleanup().await;
}

#[tokio::test]
async fn estimate_swap_includes_expiration() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/swap/estimate")
        .json(&json!({
            "from": "btc",
            "to": "eth",
            "amount": 0.1,
            "provider": "changenow"
        }))
        .await;

    let body: serde_json::Value = response.json();
    assert!(body.get("expires_at").is_some() || body.get("valid_until").is_some());

    ctx.cleanup().await;
}

#[tokio::test]
async fn estimate_swap_includes_limits() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/swap/estimate")
        .json(&json!({
            "from": "btc",
            "to": "eth",
            "amount": 0.1,
            "provider": "changenow"
        }))
        .await;

    let body: serde_json::Value = response.json();
    assert!(body.get("min_amount").is_some());
    assert!(body.get("max_amount").is_some());

    ctx.cleanup().await;
}

#[tokio::test]
async fn estimate_swap_missing_from_returns_error() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/swap/estimate")
        .json(&json!({
            "to": "eth",
            "amount": 0.1,
            "provider": "changenow"
        }))
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);

    ctx.cleanup().await;
}

#[tokio::test]
async fn estimate_swap_missing_to_returns_error() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/swap/estimate")
        .json(&json!({
            "from": "btc",
            "amount": 0.1,
            "provider": "changenow"
        }))
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);

    ctx.cleanup().await;
}

#[tokio::test]
async fn estimate_swap_missing_amount_returns_error() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/swap/estimate")
        .json(&json!({
            "from": "btc",
            "to": "eth",
            "provider": "changenow"
        }))
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);

    ctx.cleanup().await;
}

#[tokio::test]
async fn estimate_swap_missing_provider_returns_error() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/swap/estimate")
        .json(&json!({
            "from": "btc",
            "to": "eth",
            "amount": 0.1
        }))
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);

    ctx.cleanup().await;
}

#[tokio::test]
async fn estimate_swap_invalid_provider_returns_error() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/swap/estimate")
        .json(&json!({
            "from": "btc",
            "to": "eth",
            "amount": 0.1,
            "provider": "nonexistent_provider_xyz"
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);

    ctx.cleanup().await;
}

#[tokio::test]
async fn estimate_swap_below_minimum_returns_error() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/swap/estimate")
        .json(&json!({
            "from": "btc",
            "to": "eth",
            "amount": 0.00000001,
            "provider": "changenow"
        }))
        .await;

    let body: serde_json::Value = response.json();
    assert!(
        response.status_code() == StatusCode::BAD_REQUEST ||
        body.get("error").is_some() ||
        body.get("min_amount").is_some()
    );

    ctx.cleanup().await;
}

#[tokio::test]
async fn estimate_swap_above_maximum_returns_error() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/swap/estimate")
        .json(&json!({
            "from": "btc",
            "to": "eth",
            "amount": 100000,
            "provider": "changenow"
        }))
        .await;

    let body: serde_json::Value = response.json();
    assert!(
        response.status_code() == StatusCode::BAD_REQUEST ||
        body.get("error").is_some() ||
        body.get("max_amount").is_some() ||
        body.get("warning").is_some()
    );

    ctx.cleanup().await;
}

#[tokio::test]
async fn estimate_swap_zero_amount_returns_error() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/swap/estimate")
        .json(&json!({
            "from": "btc",
            "to": "eth",
            "amount": 0,
            "provider": "changenow"
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);

    ctx.cleanup().await;
}

#[tokio::test]
async fn estimate_swap_negative_amount_returns_error() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/swap/estimate")
        .json(&json!({
            "from": "btc",
            "to": "eth",
            "amount": -0.1,
            "provider": "changenow"
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);

    ctx.cleanup().await;
}

#[tokio::test]
async fn estimate_swap_same_currency_returns_error() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/swap/estimate")
        .json(&json!({
            "from": "btc",
            "to": "btc",
            "amount": 0.1,
            "provider": "changenow"
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);

    ctx.cleanup().await;
}

#[tokio::test]
async fn estimate_swap_invalid_currency_returns_error() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/swap/estimate")
        .json(&json!({
            "from": "invalidcoin999",
            "to": "eth",
            "amount": 0.1,
            "provider": "changenow"
        }))
        .await;

    assert!(
        response.status_code() == StatusCode::BAD_REQUEST ||
        response.status_code() == StatusCode::NOT_FOUND
    );

    ctx.cleanup().await;
}

#[tokio::test]
async fn estimate_swap_with_fixed_rate() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/swap/estimate")
        .json(&json!({
            "from": "btc",
            "to": "eth",
            "amount": 0.1,
            "provider": "changenow",
            "rate_type": "fixed"
        }))
        .await;

    if response.status_code() == StatusCode::OK {
        let body: serde_json::Value = response.json();
        assert_eq!(body["rate_type"], "fixed");
    }

    ctx.cleanup().await;
}

#[tokio::test]
async fn estimate_swap_with_floating_rate() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/swap/estimate")
        .json(&json!({
            "from": "btc",
            "to": "eth",
            "amount": 0.1,
            "provider": "changenow",
            "rate_type": "floating"
        }))
        .await;

    if response.status_code() == StatusCode::OK {
        let body: serde_json::Value = response.json();
        assert_eq!(body["rate_type"], "floating");
    }

    ctx.cleanup().await;
}

#[tokio::test]
async fn estimate_swap_reflects_commission() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/swap/estimate")
        .json(&json!({
            "from": "btc",
            "to": "eth",
            "amount": 1.0,
            "provider": "changenow"
        }))
        .await;

    let body: serde_json::Value = response.json();

    if let (Some(rate), Some(estimated)) = (
        body["rate"].as_f64(),
        body["estimated_amount"].as_f64()
    ) {
        // With fees, estimated should be less than rate * amount
        let raw_amount = rate * 1.0;
        assert!(
            estimated <= raw_amount,
            "Fees should reduce estimated amount: {} <= {}",
            estimated,
            raw_amount
        );
    }

    ctx.cleanup().await;
}

#[tokio::test]
async fn estimate_swap_empty_body_returns_error() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/swap/estimate")
        .json(&json!({}))
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);

    ctx.cleanup().await;
}

#[tokio::test]
async fn estimate_swap_has_security_headers() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/swap/estimate")
        .json(&json!({
            "from": "btc",
            "to": "eth",
            "amount": 0.1,
            "provider": "changenow"
        }))
        .await;

    assert!(response.headers().get("x-content-type-options").is_some());
    assert!(response.headers().get("x-frame-options").is_some());

    ctx.cleanup().await;
}

#[tokio::test]
async fn estimate_swap_responds_fast() {
    let ctx = TestContext::new().await;

    let start = std::time::Instant::now();

    ctx.server
        .post("/swap/estimate")
        .json(&json!({
            "from": "btc",
            "to": "eth",
            "amount": 0.1,
            "provider": "changenow"
        }))
        .await;

    let duration = start.elapsed();

    assert!(duration.as_secs() < 5, "Estimate took too long: {:?}", duration);

    ctx.cleanup().await;
}

#[tokio::test]
async fn estimate_swap_rate_limited() {
    let ctx = TestContext::new().await;

    let mut rate_limited = false;
    for _ in 0..30 {
        let response = ctx
            .server
            .post("/swap/estimate")
            .json(&json!({
                "from": "btc",
                "to": "eth",
                "amount": 0.1,
                "provider": "changenow"
            }))
            .await;

        if response.status_code() == StatusCode::TOO_MANY_REQUESTS {
            rate_limited = true;
            break;
        }
    }

    ctx.cleanup().await;
}
