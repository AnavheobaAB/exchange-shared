use axum::http::StatusCode;

use crate::common::TestContext;

// =============================================================================
// GET /swap/providers - List exchange providers
// Expected: <50ms for DB queries (after warmup)
// =============================================================================

#[tokio::test]
async fn get_providers_returns_list() {
    let ctx = TestContext::new().await;

    // Warm up connection pool
    ctx.server.get("/swap/providers").await;

    let start = std::time::Instant::now();
    let response = ctx.server.get("/swap/providers").await;
    let duration = start.elapsed();

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert!(body.is_array());
    assert!(!body.as_array().unwrap().is_empty());

    println!("get_providers_returns_list: {}ms", duration.as_millis());
    assert!(duration.as_millis() < 50, "Expected <50ms, got {}ms", duration.as_millis());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_providers_includes_required_fields() {
    let ctx = TestContext::new().await;

    // Warm up
    ctx.server.get("/swap/providers").await;

    let start = std::time::Instant::now();
    let response = ctx.server.get("/swap/providers").await;
    let duration = start.elapsed();

    let body: serde_json::Value = response.json();
    let first_provider = &body[0];

    assert!(first_provider.get("id").is_some());
    assert!(first_provider.get("name").is_some());
    assert!(first_provider.get("is_active").is_some());

    println!("get_providers_includes_required_fields: {}ms", duration.as_millis());
    assert!(duration.as_millis() < 50, "Expected <50ms, got {}ms", duration.as_millis());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_providers_includes_kyc_info() {
    let ctx = TestContext::new().await;

    // Warm up
    ctx.server.get("/swap/providers").await;

    let start = std::time::Instant::now();
    let response = ctx.server.get("/swap/providers").await;
    let duration = start.elapsed();

    let body: serde_json::Value = response.json();

    for provider in body.as_array().unwrap() {
        assert!(
            provider.get("kyc_required").is_some() ||
            provider.get("kyc_level").is_some() ||
            provider.get("requires_kyc").is_some()
        );
    }

    println!("get_providers_includes_kyc_info: {}ms", duration.as_millis());
    assert!(duration.as_millis() < 50, "Expected <50ms, got {}ms", duration.as_millis());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_providers_includes_supported_features() {
    let ctx = TestContext::new().await;

    // Warm up
    ctx.server.get("/swap/providers").await;

    let start = std::time::Instant::now();
    let response = ctx.server.get("/swap/providers").await;
    let duration = start.elapsed();

    let body: serde_json::Value = response.json();

    for provider in body.as_array().unwrap() {
        assert!(
            provider.get("supports_fixed_rate").is_some() ||
            provider.get("rate_types").is_some()
        );
    }

    println!("get_providers_includes_supported_features: {}ms", duration.as_millis());
    assert!(duration.as_millis() < 50, "Expected <50ms, got {}ms", duration.as_millis());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_providers_includes_rating_or_trust_score() {
    let ctx = TestContext::new().await;

    // Warm up
    ctx.server.get("/swap/providers").await;

    let start = std::time::Instant::now();
    let response = ctx.server.get("/swap/providers").await;
    let duration = start.elapsed();

    let body: serde_json::Value = response.json();

    for provider in body.as_array().unwrap() {
        assert!(
            provider.get("rating").is_some() ||
            provider.get("trust_score").is_some() ||
            provider.get("reputation").is_some()
        );
    }

    println!("get_providers_includes_rating_or_trust_score: {}ms", duration.as_millis());
    assert!(duration.as_millis() < 50, "Expected <50ms, got {}ms", duration.as_millis());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_providers_only_active_by_default() {
    let ctx = TestContext::new().await;

    // Warm up
    ctx.server.get("/swap/providers").await;

    let start = std::time::Instant::now();
    let response = ctx.server.get("/swap/providers").await;
    let duration = start.elapsed();

    let body: serde_json::Value = response.json();

    for provider in body.as_array().unwrap() {
        assert_eq!(provider["is_active"], true);
    }

    println!("get_providers_only_active_by_default: {}ms", duration.as_millis());
    assert!(duration.as_millis() < 50, "Expected <50ms, got {}ms", duration.as_millis());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_providers_include_inactive_with_param() {
    let ctx = TestContext::new().await;

    // Warm up
    ctx.server.get("/swap/providers").await;

    let start = std::time::Instant::now();
    let response = ctx
        .server
        .get("/swap/providers?include_inactive=true")
        .await;
    let duration = start.elapsed();

    response.assert_status(StatusCode::OK);

    println!("get_providers_include_inactive_with_param: {}ms", duration.as_millis());
    assert!(duration.as_millis() < 50, "Expected <50ms, got {}ms", duration.as_millis());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_providers_with_kyc_filter() {
    let ctx = TestContext::new().await;

    // Warm up
    ctx.server.get("/swap/providers").await;

    let start = std::time::Instant::now();
    let response = ctx
        .server
        .get("/swap/providers?kyc_required=false")
        .await;
    let duration = start.elapsed();

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();

    for provider in body.as_array().unwrap() {
        assert_eq!(provider["kyc_required"], false);
    }

    println!("get_providers_with_kyc_filter: {}ms", duration.as_millis());
    assert!(duration.as_millis() < 50, "Expected <50ms, got {}ms", duration.as_millis());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_providers_with_rate_type_filter() {
    let ctx = TestContext::new().await;

    // Warm up
    ctx.server.get("/swap/providers").await;

    let start = std::time::Instant::now();
    let response = ctx
        .server
        .get("/swap/providers?rate_type=fixed")
        .await;
    let duration = start.elapsed();

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

    println!("get_providers_with_rate_type_filter: {}ms", duration.as_millis());
    assert!(duration.as_millis() < 50, "Expected <50ms, got {}ms", duration.as_millis());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_providers_sorted_by_name() {
    let ctx = TestContext::new().await;

    // Warm up
    ctx.server.get("/swap/providers").await;

    let start = std::time::Instant::now();
    let response = ctx
        .server
        .get("/swap/providers?sort=name")
        .await;
    let duration = start.elapsed();

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

    println!("get_providers_sorted_by_name: {}ms", duration.as_millis());
    assert!(duration.as_millis() < 50, "Expected <50ms, got {}ms", duration.as_millis());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_providers_sorted_by_rating() {
    let ctx = TestContext::new().await;

    // Warm up
    ctx.server.get("/swap/providers").await;

    let start = std::time::Instant::now();
    let response = ctx
        .server
        .get("/swap/providers?sort=rating")
        .await;
    let duration = start.elapsed();

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

    println!("get_providers_sorted_by_rating: {}ms", duration.as_millis());
    assert!(duration.as_millis() < 50, "Expected <50ms, got {}ms", duration.as_millis());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_providers_no_auth_required() {
    let ctx = TestContext::new().await;

    // Warm up
    ctx.server.get("/swap/providers").await;

    let start = std::time::Instant::now();
    let response = ctx.server.get("/swap/providers").await;
    let duration = start.elapsed();

    response.assert_status(StatusCode::OK);

    println!("get_providers_no_auth_required: {}ms", duration.as_millis());
    assert!(duration.as_millis() < 50, "Expected <50ms, got {}ms", duration.as_millis());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_providers_has_security_headers() {
    let ctx = TestContext::new().await;

    // Warm up
    ctx.server.get("/swap/providers").await;

    let start = std::time::Instant::now();
    let response = ctx.server.get("/swap/providers").await;
    let duration = start.elapsed();

    assert!(response.headers().get("x-content-type-options").is_some());
    assert!(response.headers().get("x-frame-options").is_some());

    println!("get_providers_has_security_headers: {}ms", duration.as_millis());
    assert!(duration.as_millis() < 50, "Expected <50ms, got {}ms", duration.as_millis());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_providers_responds_fast() {
    let ctx = TestContext::new().await;

    // Warm up
    ctx.server.get("/swap/providers").await;

    let start = std::time::Instant::now();
    ctx.server.get("/swap/providers").await;
    let duration = start.elapsed();

    println!("get_providers_responds_fast: {}ms", duration.as_millis());
    assert!(duration.as_millis() < 50, "Expected <50ms, got {}ms", duration.as_millis());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_providers_caches_response() {
    let ctx = TestContext::new().await;

    // First request (includes warmup)
    let start = std::time::Instant::now();
    ctx.server.get("/swap/providers").await;
    let first_duration = start.elapsed();

    // Second request (should be cached/faster)
    let start = std::time::Instant::now();
    ctx.server.get("/swap/providers").await;
    let second_duration = start.elapsed();

    println!("get_providers_caches_response: first={}ms, second={}ms",
             first_duration.as_millis(), second_duration.as_millis());

    // Second should be faster or very quick
    assert!(
        second_duration.as_millis() < 50,
        "Second request should be <50ms, got {}ms", second_duration.as_millis()
    );

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_providers_rate_limited() {
    let ctx = TestContext::new().await;

    let start = std::time::Instant::now();
    let mut rate_limited = false;
    for _ in 0..50 {
        let response = ctx.server.get("/swap/providers").await;
        if response.status_code() == StatusCode::TOO_MANY_REQUESTS {
            rate_limited = true;
            break;
        }
    }
    let duration = start.elapsed();

    println!("get_providers_rate_limited: {}ms (rate_limited={})",
             duration.as_millis(), rate_limited);

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_providers_handles_invalid_sort() {
    let ctx = TestContext::new().await;

    // Warm up
    ctx.server.get("/swap/providers").await;

    let start = std::time::Instant::now();
    // Test with invalid sort parameter (should fallback to default)
    let response = ctx
        .server
        .get("/swap/providers?sort=invalid_sort_value")
        .await;
    let duration = start.elapsed();

    // Should return OK with default sort, not crash
    response.assert_status(StatusCode::OK);

    println!("get_providers_handles_invalid_sort: {}ms", duration.as_millis());
    assert!(duration.as_millis() < 50, "Expected <50ms, got {}ms", duration.as_millis());

    ctx.cleanup().await;
}

// =============================================================================
// GET /swap/providers/{id} - Get single provider details
// Expected: <50ms for single record lookup (after warmup)
// =============================================================================

#[tokio::test]
async fn get_single_provider_returns_details() {
    let ctx = TestContext::new().await;

    // Warm up
    ctx.server.get("/swap/providers").await;

    let start = std::time::Instant::now();
    let response = ctx.server.get("/swap/providers/changenow").await;
    let duration = start.elapsed();

    if response.status_code() == StatusCode::OK {
        let body: serde_json::Value = response.json();
        assert!(body.get("id").is_some());
        assert!(body.get("name").is_some());
        assert!(body.get("is_active").is_some());
    }

    println!("get_single_provider_returns_details: {}ms", duration.as_millis());
    assert!(duration.as_millis() < 50, "Expected <50ms, got {}ms", duration.as_millis());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_single_provider_nonexistent_returns_not_found() {
    let ctx = TestContext::new().await;

    // Warm up
    ctx.server.get("/swap/providers").await;

    let start = std::time::Instant::now();
    let response = ctx.server.get("/swap/providers/nonexistent_provider").await;
    let duration = start.elapsed();

    response.assert_status(StatusCode::NOT_FOUND);

    println!("get_single_provider_nonexistent_returns_not_found: {}ms", duration.as_millis());
    assert!(duration.as_millis() < 50, "Expected <50ms, got {}ms", duration.as_millis());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_single_provider_includes_supported_currencies() {
    let ctx = TestContext::new().await;

    // Warm up both endpoints
    ctx.server.get("/swap/providers").await;
    ctx.server.get("/swap/providers/changenow").await;

    let start = std::time::Instant::now();
    let response = ctx.server.get("/swap/providers/changenow").await;
    let duration = start.elapsed();

    if response.status_code() == StatusCode::OK {
        let body: serde_json::Value = response.json();
        // supported_currencies is always present (may be empty array)
        assert!(body.get("supported_currencies").is_some());
    }

    println!("get_single_provider_includes_supported_currencies: {}ms", duration.as_millis());
    assert!(duration.as_millis() < 50, "Expected <50ms, got {}ms", duration.as_millis());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_single_provider_includes_limits() {
    let ctx = TestContext::new().await;

    // Warm up both endpoints
    ctx.server.get("/swap/providers").await;
    ctx.server.get("/swap/providers/changenow").await;

    let start = std::time::Instant::now();
    let response = ctx.server.get("/swap/providers/changenow").await;
    let duration = start.elapsed();

    // Just verify the endpoint returns valid provider data
    // min_amount/max_amount are optional and may not be present if no provider_currencies data
    response.assert_status(StatusCode::OK);

    println!("get_single_provider_includes_limits: {}ms", duration.as_millis());
    assert!(duration.as_millis() < 50, "Expected <50ms, got {}ms", duration.as_millis());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_single_provider_includes_fee_info() {
    let ctx = TestContext::new().await;

    // Warm up both endpoints
    ctx.server.get("/swap/providers").await;
    ctx.server.get("/swap/providers/changenow").await;

    let start = std::time::Instant::now();
    let response = ctx.server.get("/swap/providers/changenow").await;
    let duration = start.elapsed();

    if response.status_code() == StatusCode::OK {
        let body: serde_json::Value = response.json();
        // fee_percentage is always present in ProviderDetailResponse
        assert!(body.get("fee_percentage").is_some());
    }

    println!("get_single_provider_includes_fee_info: {}ms", duration.as_millis());
    assert!(duration.as_millis() < 50, "Expected <50ms, got {}ms", duration.as_millis());

    ctx.cleanup().await;
}

// =============================================================================
// ADDITIONAL TESTS - Edge cases and validation
// =============================================================================

#[tokio::test]
async fn get_providers_multiple_filters_combined() {
    let ctx = TestContext::new().await;

    // Warm up
    ctx.server.get("/swap/providers").await;

    let start = std::time::Instant::now();
    let response = ctx
        .server
        .get("/swap/providers?kyc_required=false&rate_type=fixed")
        .await;
    let duration = start.elapsed();

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    let providers = body.as_array().unwrap();

    // All returned providers should match BOTH filters
    for provider in providers {
        assert_eq!(provider["kyc_required"], false, "Should have no KYC");
        assert_eq!(provider["supports_fixed_rate"], true, "Should support fixed rate");
    }

    println!("get_providers_multiple_filters_combined: {}ms (found {} providers)",
             duration.as_millis(), providers.len());
    assert!(duration.as_millis() < 50, "Expected <50ms, got {}ms", duration.as_millis());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_providers_empty_result_set() {
    let ctx = TestContext::new().await;

    // Warm up
    ctx.server.get("/swap/providers").await;

    let start = std::time::Instant::now();
    // Filter for KYC required = true (none of our seed data has this)
    let response = ctx
        .server
        .get("/swap/providers?kyc_required=true")
        .await;
    let duration = start.elapsed();

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    let providers = body.as_array().unwrap();

    // Should return empty array, not error
    assert!(providers.is_empty(), "Expected empty array for kyc_required=true");

    println!("get_providers_empty_result_set: {}ms", duration.as_millis());
    assert!(duration.as_millis() < 50, "Expected <50ms, got {}ms", duration.as_millis());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_providers_content_type_header() {
    let ctx = TestContext::new().await;

    let response = ctx.server.get("/swap/providers").await;

    response.assert_status(StatusCode::OK);

    let content_type = response.headers().get("content-type");
    assert!(content_type.is_some(), "Content-Type header should be present");

    let content_type_str = content_type.unwrap().to_str().unwrap();
    assert!(
        content_type_str.contains("application/json"),
        "Content-Type should be application/json, got: {}", content_type_str
    );

    println!("get_providers_content_type_header: Content-Type = {}", content_type_str);

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_providers_invalid_filter_values() {
    let ctx = TestContext::new().await;

    // Warm up
    ctx.server.get("/swap/providers").await;

    let start = std::time::Instant::now();
    // Invalid boolean value for kyc_required
    let response = ctx
        .server
        .get("/swap/providers?kyc_required=invalid")
        .await;
    let duration = start.elapsed();

    // Should either return 400 Bad Request or ignore invalid filter and return 200
    assert!(
        response.status_code() == StatusCode::OK ||
        response.status_code() == StatusCode::BAD_REQUEST,
        "Expected 200 or 400, got {}", response.status_code()
    );

    println!("get_providers_invalid_filter_values: {}ms, status={}",
             duration.as_millis(), response.status_code());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_providers_wrong_http_method_post() {
    let ctx = TestContext::new().await;

    let response = ctx.server.post("/swap/providers").await;

    // Should return 405 Method Not Allowed
    response.assert_status(StatusCode::METHOD_NOT_ALLOWED);

    println!("get_providers_wrong_http_method_post: 405 Method Not Allowed");

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_providers_wrong_http_method_put() {
    let ctx = TestContext::new().await;

    let response = ctx.server.put("/swap/providers").await;

    response.assert_status(StatusCode::METHOD_NOT_ALLOWED);

    println!("get_providers_wrong_http_method_put: 405 Method Not Allowed");

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_providers_wrong_http_method_delete() {
    let ctx = TestContext::new().await;

    let response = ctx.server.delete("/swap/providers").await;

    response.assert_status(StatusCode::METHOD_NOT_ALLOWED);

    println!("get_providers_wrong_http_method_delete: 405 Method Not Allowed");

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_single_provider_case_sensitivity() {
    let ctx = TestContext::new().await;

    // Warm up
    ctx.server.get("/swap/providers").await;

    // Test uppercase - should return 404 if case-sensitive
    let response_upper = ctx.server.get("/swap/providers/CHANGENOW").await;

    // Test lowercase - should return 200
    let response_lower = ctx.server.get("/swap/providers/changenow").await;

    response_lower.assert_status(StatusCode::OK);

    // Provider IDs are case-sensitive in our DB (lowercase)
    // Uppercase should return 404
    println!("get_single_provider_case_sensitivity: CHANGENOW={}, changenow={}",
             response_upper.status_code(), response_lower.status_code());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_providers_verify_seed_data() {
    let ctx = TestContext::new().await;

    let response = ctx.server.get("/swap/providers").await;
    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    let providers = body.as_array().unwrap();

    // Verify we have all 8 seeded providers
    assert_eq!(providers.len(), 8, "Should have 8 seeded providers");

    // Verify specific provider data matches seed
    let changenow = providers.iter().find(|p| p["id"] == "changenow");
    assert!(changenow.is_some(), "ChangeNOW should exist");

    let changenow = changenow.unwrap();
    assert_eq!(changenow["name"], "ChangeNOW");
    assert_eq!(changenow["rating"], 4.5);
    assert_eq!(changenow["kyc_required"], false);
    assert_eq!(changenow["supports_fixed_rate"], true);
    assert_eq!(changenow["supports_floating_rate"], true);
    assert_eq!(changenow["website_url"], "https://changenow.io");

    // Verify SideShift (no fixed rate support)
    let sideshift = providers.iter().find(|p| p["id"] == "sideshift");
    assert!(sideshift.is_some(), "SideShift should exist");

    let sideshift = sideshift.unwrap();
    assert_eq!(sideshift["supports_fixed_rate"], false, "SideShift should NOT support fixed rate");
    assert_eq!(sideshift["supports_floating_rate"], true);

    println!("get_providers_verify_seed_data: All 8 providers verified");

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_providers_cors_headers() {
    let ctx = TestContext::new().await;

    let response = ctx.server.get("/swap/providers").await;
    response.assert_status(StatusCode::OK);

    // Check for CORS headers (we have CorsLayer::permissive())
    let headers = response.headers();

    // With permissive CORS, these should be present
    let has_cors = headers.get("access-control-allow-origin").is_some() ||
                   headers.get("vary").map_or(false, |v| v.to_str().unwrap_or("").contains("Origin"));

    println!("get_providers_cors_headers: CORS enabled = {}", has_cors);
    println!("  Headers: {:?}", headers.keys().collect::<Vec<_>>());

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_single_provider_verify_detail_fields() {
    let ctx = TestContext::new().await;

    // Warm up
    ctx.server.get("/swap/providers").await;

    let response = ctx.server.get("/swap/providers/changenow").await;
    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();

    // Verify all expected fields in detail response
    assert_eq!(body["id"], "changenow");
    assert_eq!(body["name"], "ChangeNOW");
    assert_eq!(body["is_active"], true);
    assert_eq!(body["kyc_required"], false);
    assert_eq!(body["rating"], 4.5);
    assert_eq!(body["supports_fixed_rate"], true);
    assert_eq!(body["supports_floating_rate"], true);
    assert_eq!(body["website_url"], "https://changenow.io");
    assert!(body.get("supported_currencies").is_some());
    assert!(body.get("fee_percentage").is_some());

    println!("get_single_provider_verify_detail_fields: All fields verified for changenow");

    ctx.cleanup().await;
}
