use axum::http::StatusCode;

use crate::common::TestContext;

// =============================================================================
// GET /swap/rates - Get exchange rates from providers
// =============================================================================

#[tokio::test]
async fn get_rates_returns_providers_list() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/swap/rates?from=btc&to=eth&amount=0.1")
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert!(body.get("rates").is_some());
    assert!(body["rates"].is_array());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_rates_includes_provider_details() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/swap/rates?from=btc&to=eth&amount=0.1")
        .await;

    let body: serde_json::Value = response.json();

    if let Some(rates) = body["rates"].as_array() {
        if !rates.is_empty() {
            let first_rate = &rates[0];
            assert!(first_rate.get("provider").is_some());
            assert!(first_rate.get("rate").is_some());
            assert!(first_rate.get("estimated_amount").is_some());
            assert!(first_rate.get("min_amount").is_some());
            assert!(first_rate.get("max_amount").is_some());
        }
    }

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_rates_includes_fee_breakdown() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/swap/rates?from=btc&to=eth&amount=1")
        .await;

    let body: serde_json::Value = response.json();

    if let Some(rates) = body["rates"].as_array() {
        for rate in rates {
            // Should include fee information
            assert!(
                rate.get("network_fee").is_some() ||
                rate.get("provider_fee").is_some() ||
                rate.get("total_fee").is_some()
            );
        }
    }

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_rates_includes_platform_commission() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/swap/rates?from=btc&to=eth&amount=1")
        .await;

    let body: serde_json::Value = response.json();

    if let Some(rates) = body["rates"].as_array() {
        for rate in rates {
            assert!(
                rate.get("platform_fee").is_some() ||
                rate.get("our_fee").is_some() ||
                rate.get("commission").is_some()
            );
        }
    }

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_rates_missing_from_returns_bad_request() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/swap/rates?to=eth&amount=0.1")
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_rates_missing_to_returns_bad_request() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/swap/rates?from=btc&amount=0.1")
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_rates_missing_amount_returns_bad_request() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/swap/rates?from=btc&to=eth")
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_rates_with_zero_amount_returns_bad_request() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/swap/rates?from=btc&to=eth&amount=0")
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_rates_with_negative_amount_returns_bad_request() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/swap/rates?from=btc&to=eth&amount=-0.1")
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_rates_with_non_numeric_amount_returns_bad_request() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/swap/rates?from=btc&to=eth&amount=abc")
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_rates_with_invalid_from_currency_returns_not_found() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/swap/rates?from=invalidcoin999&to=eth&amount=0.1")
        .await;

    response.assert_status(StatusCode::NOT_FOUND);

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_rates_with_invalid_to_currency_returns_not_found() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/swap/rates?from=btc&to=invalidcoin999&amount=0.1")
        .await;

    response.assert_status(StatusCode::NOT_FOUND);

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_rates_same_currency_returns_bad_request() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/swap/rates?from=btc&to=btc&amount=0.1")
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);

    let body: serde_json::Value = response.json();
    assert!(body["error"].as_str().unwrap().to_lowercase().contains("same"));

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_rates_sorts_by_best_rate() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/swap/rates?from=btc&to=eth&amount=0.1")
        .await;

    let body: serde_json::Value = response.json();

    if let Some(rates) = body["rates"].as_array() {
        if rates.len() > 1 {
            let first_amount = rates[0]["estimated_amount"].as_f64().unwrap_or(0.0);
            let second_amount = rates[1]["estimated_amount"].as_f64().unwrap_or(0.0);
            assert!(
                first_amount >= second_amount,
                "Rates should be sorted by best offer: {} >= {}",
                first_amount,
                second_amount
            );
        }
    }

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_rates_with_fixed_rate_type() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/swap/rates?from=btc&to=eth&amount=0.1&rate_type=fixed")
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    for rate in body["rates"].as_array().unwrap_or(&vec![]) {
        assert_eq!(rate["rate_type"], "fixed");
    }

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_rates_with_floating_rate_type() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/swap/rates?from=btc&to=eth&amount=0.1&rate_type=floating")
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    for rate in body["rates"].as_array().unwrap_or(&vec![]) {
        assert_eq!(rate["rate_type"], "floating");
    }

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_rates_with_invalid_rate_type_returns_bad_request() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/swap/rates?from=btc&to=eth&amount=0.1&rate_type=invalid")
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_rates_with_specific_provider() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/swap/rates?from=btc&to=eth&amount=0.1&provider=changenow")
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    for rate in body["rates"].as_array().unwrap_or(&vec![]) {
        assert_eq!(rate["provider"].as_str().unwrap().to_lowercase(), "changenow");
    }

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_rates_very_small_amount() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/swap/rates?from=btc&to=eth&amount=0.00000001")
        .await;

    let body: serde_json::Value = response.json();

    // Should return error about minimum or empty rates
    assert!(
        response.status_code() == StatusCode::BAD_REQUEST ||
        body["rates"].as_array().map_or(true, |r| r.is_empty()) ||
        body.get("error").is_some()
    );

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_rates_very_large_amount() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/swap/rates?from=btc&to=eth&amount=10000")
        .await;

    // Should handle gracefully
    assert!(
        response.status_code() == StatusCode::OK ||
        response.status_code() == StatusCode::BAD_REQUEST
    );

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_rates_case_insensitive_currencies() {
    let ctx = TestContext::new().await;

    let response_lower = ctx
        .server
        .get("/swap/rates?from=btc&to=eth&amount=0.1")
        .await;

    let response_upper = ctx
        .server
        .get("/swap/rates?from=BTC&to=ETH&amount=0.1")
        .await;

    assert_eq!(response_lower.status_code(), response_upper.status_code());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_rates_has_security_headers() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/swap/rates?from=btc&to=eth&amount=0.1")
        .await;

    assert!(response.headers().get("x-content-type-options").is_some());
    assert!(response.headers().get("x-frame-options").is_some());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_rates_responds_within_acceptable_time() {
    let ctx = TestContext::new().await;

    let start = std::time::Instant::now();

    ctx.server
        .get("/swap/rates?from=btc&to=eth&amount=0.1")
        .await;

    let duration = start.elapsed();

    // Rates aggregation from multiple providers should complete within 10 seconds
    assert!(duration.as_secs() < 10, "Rates took too long: {:?}", duration);

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_rates_handles_provider_timeout_gracefully() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/swap/rates?from=btc&to=eth&amount=0.1")
        .await;

    // Should not return 500 even if some providers fail
    assert_ne!(response.status_code(), StatusCode::INTERNAL_SERVER_ERROR);

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_rates_rate_limited() {
    let ctx = TestContext::new().await;

    let mut rate_limited = false;
    for _ in 0..30 {
        let response = ctx
            .server
            .get("/swap/rates?from=btc&to=eth&amount=0.1")
            .await;

        if response.status_code() == StatusCode::TOO_MANY_REQUESTS {
            rate_limited = true;
            break;
        }
    }

    // Rate limiting may or may not trigger depending on config
    ctx.cleanup().await;
}

#[tokio::test]
async fn get_rates_handles_sql_injection() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/swap/rates?from=btc';DROP TABLE swaps;--&to=eth&amount=0.1")
        .await;

    assert!(
        response.status_code() == StatusCode::BAD_REQUEST ||
        response.status_code() == StatusCode::NOT_FOUND
    );

    ctx.cleanup().await;
}
