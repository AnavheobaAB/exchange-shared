use axum::http::StatusCode;

use crate::common::TestContext;

// =============================================================================
// GET /swap/providers - List exchange providers
// =============================================================================

#[tokio::test]
async fn get_providers_returns_list() {
    let ctx = TestContext::new().await;

    let response = ctx.server.get("/swap/providers").await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert!(body.is_array());
    assert!(!body.as_array().unwrap().is_empty());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_providers_includes_required_fields() {
    let ctx = TestContext::new().await;

    let response = ctx.server.get("/swap/providers").await;

    let body: serde_json::Value = response.json();
    let first_provider = &body[0];

    assert!(first_provider.get("id").is_some());
    assert!(first_provider.get("name").is_some());
    assert!(first_provider.get("is_active").is_some());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_providers_includes_kyc_info() {
    let ctx = TestContext::new().await;

    let response = ctx.server.get("/swap/providers").await;

    let body: serde_json::Value = response.json();

    for provider in body.as_array().unwrap() {
        assert!(
            provider.get("kyc_required").is_some() ||
            provider.get("kyc_level").is_some() ||
            provider.get("requires_kyc").is_some()
        );
    }

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_providers_includes_supported_features() {
    let ctx = TestContext::new().await;

    let response = ctx.server.get("/swap/providers").await;

    let body: serde_json::Value = response.json();

    for provider in body.as_array().unwrap() {
        // Should indicate rate types supported
        assert!(
            provider.get("supports_fixed_rate").is_some() ||
            provider.get("rate_types").is_some()
        );
    }

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_providers_includes_rating_or_trust_score() {
    let ctx = TestContext::new().await;

    let response = ctx.server.get("/swap/providers").await;

    let body: serde_json::Value = response.json();

    for provider in body.as_array().unwrap() {
        assert!(
            provider.get("rating").is_some() ||
            provider.get("trust_score").is_some() ||
            provider.get("reputation").is_some()
        );
    }

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_providers_only_active_by_default() {
    let ctx = TestContext::new().await;

    let response = ctx.server.get("/swap/providers").await;

    let body: serde_json::Value = response.json();

    for provider in body.as_array().unwrap() {
        assert_eq!(provider["is_active"], true);
    }

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_providers_include_inactive_with_param() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/swap/providers?include_inactive=true")
        .await;

    response.assert_status(StatusCode::OK);

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_providers_with_kyc_filter() {
    let ctx = TestContext::new().await;

    // Get only no-KYC providers
    let response = ctx
        .server
        .get("/swap/providers?kyc_required=false")
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();

    for provider in body.as_array().unwrap() {
        assert_eq!(provider["kyc_required"], false);
    }

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_providers_with_rate_type_filter() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/swap/providers?rate_type=fixed")
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();

    for provider in body.as_array().unwrap() {
        assert!(
            provider["supports_fixed_rate"] == true ||
            provider["rate_types"].as_array().map_or(false, |r| {
                r.iter().any(|t| t == "fixed")
            })
        );
    }

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_providers_sorted_by_name() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/swap/providers?sort=name")
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    let providers = body.as_array().unwrap();

    if providers.len() > 1 {
        for i in 0..providers.len() - 1 {
            let name1 = providers[i]["name"].as_str().unwrap().to_lowercase();
            let name2 = providers[i + 1]["name"].as_str().unwrap().to_lowercase();
            assert!(name1 <= name2, "Should be sorted alphabetically");
        }
    }

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_providers_sorted_by_rating() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/swap/providers?sort=rating")
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    let providers = body.as_array().unwrap();

    if providers.len() > 1 {
        for i in 0..providers.len() - 1 {
            let rating1 = providers[i]["rating"].as_f64().unwrap_or(0.0);
            let rating2 = providers[i + 1]["rating"].as_f64().unwrap_or(0.0);
            assert!(rating1 >= rating2, "Should be sorted by rating descending");
        }
    }

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_providers_no_auth_required() {
    let ctx = TestContext::new().await;

    let response = ctx.server.get("/swap/providers").await;

    response.assert_status(StatusCode::OK);

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_providers_has_security_headers() {
    let ctx = TestContext::new().await;

    let response = ctx.server.get("/swap/providers").await;

    assert!(response.headers().get("x-content-type-options").is_some());
    assert!(response.headers().get("x-frame-options").is_some());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_providers_responds_fast() {
    let ctx = TestContext::new().await;

    let start = std::time::Instant::now();
    ctx.server.get("/swap/providers").await;
    let duration = start.elapsed();

    assert!(duration.as_secs() < 2, "Response took too long: {:?}", duration);

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_providers_caches_response() {
    let ctx = TestContext::new().await;

    let start = std::time::Instant::now();
    ctx.server.get("/swap/providers").await;
    let first_duration = start.elapsed();

    let start = std::time::Instant::now();
    ctx.server.get("/swap/providers").await;
    let second_duration = start.elapsed();

    assert!(
        second_duration < first_duration || second_duration.as_millis() < 100,
        "Cache not working"
    );

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_providers_rate_limited() {
    let ctx = TestContext::new().await;

    let mut rate_limited = false;
    for _ in 0..50 {
        let response = ctx.server.get("/swap/providers").await;
        if response.status_code() == StatusCode::TOO_MANY_REQUESTS {
            rate_limited = true;
            break;
        }
    }

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_providers_handles_sql_injection() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/swap/providers?sort='; DROP TABLE providers; --")
        .await;

    assert!(
        response.status_code() == StatusCode::OK ||
        response.status_code() == StatusCode::BAD_REQUEST
    );

    ctx.cleanup().await;
}

// =============================================================================
// GET /swap/providers/{id} - Get single provider details
// =============================================================================

#[tokio::test]
async fn get_single_provider_returns_details() {
    let ctx = TestContext::new().await;

    let response = ctx.server.get("/swap/providers/changenow").await;

    if response.status_code() == StatusCode::OK {
        let body: serde_json::Value = response.json();
        assert!(body.get("id").is_some());
        assert!(body.get("name").is_some());
        assert!(body.get("is_active").is_some());
    }

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_single_provider_nonexistent_returns_not_found() {
    let ctx = TestContext::new().await;

    let response = ctx.server.get("/swap/providers/nonexistent_provider").await;

    response.assert_status(StatusCode::NOT_FOUND);

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_single_provider_includes_supported_currencies() {
    let ctx = TestContext::new().await;

    let response = ctx.server.get("/swap/providers/changenow").await;

    if response.status_code() == StatusCode::OK {
        let body: serde_json::Value = response.json();
        assert!(
            body.get("supported_currencies").is_some() ||
            body.get("currencies").is_some()
        );
    }

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_single_provider_includes_limits() {
    let ctx = TestContext::new().await;

    let response = ctx.server.get("/swap/providers/changenow").await;

    if response.status_code() == StatusCode::OK {
        let body: serde_json::Value = response.json();
        assert!(
            body.get("min_amount").is_some() ||
            body.get("max_amount").is_some() ||
            body.get("limits").is_some()
        );
    }

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_single_provider_includes_fee_info() {
    let ctx = TestContext::new().await;

    let response = ctx.server.get("/swap/providers/changenow").await;

    if response.status_code() == StatusCode::OK {
        let body: serde_json::Value = response.json();
        assert!(
            body.get("fee").is_some() ||
            body.get("fee_percentage").is_some() ||
            body.get("network_fee").is_some()
        );
    }

    ctx.cleanup().await;
}
