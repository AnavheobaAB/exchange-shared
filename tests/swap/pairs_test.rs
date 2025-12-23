use axum::http::StatusCode;

use crate::common::TestContext;

// =============================================================================
// GET /swap/pairs - Get available trading pairs
// =============================================================================

#[tokio::test]
async fn get_pairs_returns_list() {
    let ctx = TestContext::new().await;

    let response = ctx.server.get("/swap/pairs").await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert!(body.is_array());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_pairs_includes_required_fields() {
    let ctx = TestContext::new().await;

    let response = ctx.server.get("/swap/pairs").await;
    let body: serde_json::Value = response.json();

    if !body.as_array().unwrap().is_empty() {
        let first_pair = &body[0];
        assert!(first_pair.get("from").is_some());
        assert!(first_pair.get("to").is_some());
        assert!(first_pair.get("is_active").is_some());
    }

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_pairs_with_from_currency_filter() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/swap/pairs?from=btc")
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    for pair in body.as_array().unwrap() {
        assert_eq!(pair["from"].as_str().unwrap().to_lowercase(), "btc");
    }

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_pairs_with_to_currency_filter() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/swap/pairs?to=usdt")
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    for pair in body.as_array().unwrap() {
        assert_eq!(pair["to"].as_str().unwrap().to_lowercase(), "usdt");
    }

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_pairs_with_both_filters() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/swap/pairs?from=btc&to=eth")
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    for pair in body.as_array().unwrap() {
        assert_eq!(pair["from"].as_str().unwrap().to_lowercase(), "btc");
        assert_eq!(pair["to"].as_str().unwrap().to_lowercase(), "eth");
    }

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_pairs_invalid_from_returns_empty() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/swap/pairs?from=invalidcoin999")
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert!(body.as_array().unwrap().is_empty());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_pairs_invalid_to_returns_empty() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/swap/pairs?to=invalidcoin999")
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert!(body.as_array().unwrap().is_empty());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_pairs_case_insensitive() {
    let ctx = TestContext::new().await;

    let response_lower = ctx.server.get("/swap/pairs?from=btc").await;
    let response_upper = ctx.server.get("/swap/pairs?from=BTC").await;

    let body_lower: serde_json::Value = response_lower.json();
    let body_upper: serde_json::Value = response_upper.json();

    assert_eq!(
        body_lower.as_array().unwrap().len(),
        body_upper.as_array().unwrap().len()
    );

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_pairs_with_network_filter() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/swap/pairs?from_network=ethereum")
        .await;

    response.assert_status(StatusCode::OK);

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_pairs_only_active_by_default() {
    let ctx = TestContext::new().await;

    let response = ctx.server.get("/swap/pairs").await;

    let body: serde_json::Value = response.json();
    for pair in body.as_array().unwrap() {
        assert_eq!(pair["is_active"], true);
    }

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_pairs_has_security_headers() {
    let ctx = TestContext::new().await;

    let response = ctx.server.get("/swap/pairs").await;

    assert!(response.headers().get("x-content-type-options").is_some());
    assert!(response.headers().get("x-frame-options").is_some());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_pairs_responds_fast() {
    let ctx = TestContext::new().await;

    let start = std::time::Instant::now();
    ctx.server.get("/swap/pairs").await;
    let duration = start.elapsed();

    assert!(duration.as_secs() < 2, "Response took too long: {:?}", duration);

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_pairs_handles_special_characters() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/swap/pairs?from=btc%00&to=eth")
        .await;

    // Should not crash, should return error or empty
    assert!(
        response.status_code() == StatusCode::OK ||
        response.status_code() == StatusCode::BAD_REQUEST
    );

    ctx.cleanup().await;
}
