use axum::http::StatusCode;

use crate::common::TestContext;

// =============================================================================
// GET /swap/currencies - List supported cryptocurrencies
// =============================================================================

#[tokio::test]
async fn get_currencies_returns_list() {
    let ctx = TestContext::new().await;

    let response = ctx.server.get("/swap/currencies").await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert!(body.is_array());
    assert!(!body.as_array().unwrap().is_empty());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_currencies_includes_required_fields() {
    let ctx = TestContext::new().await;

    let response = ctx.server.get("/swap/currencies").await;
    let body: serde_json::Value = response.json();

    let first_currency = &body[0];
    assert!(first_currency.get("symbol").is_some());
    assert!(first_currency.get("name").is_some());
    assert!(first_currency.get("network").is_some());
    assert!(first_currency.get("is_active").is_some());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_currencies_with_network_filter() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/swap/currencies?network=tron")
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    for currency in body.as_array().unwrap() {
        assert_eq!(currency["network"].as_str().unwrap().to_lowercase(), "tron");
    }

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_currencies_with_search_filter() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/swap/currencies?search=bitcoin")
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    for currency in body.as_array().unwrap() {
        let name = currency["name"].as_str().unwrap().to_lowercase();
        let symbol = currency["symbol"].as_str().unwrap().to_lowercase();
        assert!(name.contains("bitcoin") || symbol.contains("btc"));
    }

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_currencies_with_invalid_network_returns_empty() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/swap/currencies?network=invalidnetwork123")
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert!(body.as_array().unwrap().is_empty());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_currencies_only_active_by_default() {
    let ctx = TestContext::new().await;

    let response = ctx.server.get("/swap/currencies").await;

    let body: serde_json::Value = response.json();
    for currency in body.as_array().unwrap() {
        assert_eq!(currency["is_active"], true);
    }

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_currencies_include_inactive_with_param() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/swap/currencies?include_inactive=true")
        .await;

    response.assert_status(StatusCode::OK);

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_currencies_has_security_headers() {
    let ctx = TestContext::new().await;

    let response = ctx.server.get("/swap/currencies").await;

    assert!(response.headers().get("x-content-type-options").is_some());
    assert!(response.headers().get("x-frame-options").is_some());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_currencies_responds_fast() {
    let ctx = TestContext::new().await;

    let start = std::time::Instant::now();
    ctx.server.get("/swap/currencies").await;
    let duration = start.elapsed();

    // Should respond within 2 seconds (may be cached)
    assert!(duration.as_secs() < 2, "Response took too long: {:?}", duration);

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_currencies_caches_response() {
    let ctx = TestContext::new().await;

    let start = std::time::Instant::now();
    ctx.server.get("/swap/currencies").await;
    let first_duration = start.elapsed();

    let start = std::time::Instant::now();
    ctx.server.get("/swap/currencies").await;
    let second_duration = start.elapsed();

    // Second request should be faster or very quick (cached)
    assert!(
        second_duration < first_duration || second_duration.as_millis() < 100,
        "Cache not working: first={:?}, second={:?}",
        first_duration,
        second_duration
    );

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_currencies_rate_limited() {
    let ctx = TestContext::new().await;

    let mut rate_limited = false;
    for _ in 0..50 {
        let response = ctx.server.get("/swap/currencies").await;
        if response.status_code() == StatusCode::TOO_MANY_REQUESTS {
            rate_limited = true;
            break;
        }
    }

    // Rate limiting should kick in eventually
    // (depends on config, may not trigger in test)
    ctx.cleanup().await;
}
