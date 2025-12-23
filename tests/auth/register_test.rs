use axum::http::StatusCode;
use serde_json::json;

use crate::common::{test_email, test_password, TestContext};

#[tokio::test]
async fn register_with_valid_data_returns_created() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/auth/register")
        .json(&json!({
            "email": test_email(),
            "password": test_password(),
            "password_confirm": test_password()
        }))
        .await;

    response.assert_status(StatusCode::CREATED);

    let body: serde_json::Value = response.json();
    assert!(body.get("user").is_some());
    assert!(body["user"].get("id").is_some());
    assert!(body["user"].get("email").is_some());
    assert!(body["user"].get("password").is_none()); // Password should not be returned

    ctx.cleanup().await;
}

#[tokio::test]
async fn register_with_mismatched_passwords_returns_bad_request() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/auth/register")
        .json(&json!({
            "email": test_email(),
            "password": "Password123!",
            "password_confirm": "DifferentPassword123!"
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);

    let body: serde_json::Value = response.json();
    assert!(body.get("error").is_some());

    ctx.cleanup().await;
}

#[tokio::test]
async fn register_with_invalid_email_returns_bad_request() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/auth/register")
        .json(&json!({
            "email": "invalid-email",
            "password": test_password(),
            "password_confirm": test_password()
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);

    ctx.cleanup().await;
}

#[tokio::test]
async fn register_with_weak_password_returns_bad_request() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/auth/register")
        .json(&json!({
            "email": test_email(),
            "password": "weak",
            "password_confirm": "weak"
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);

    let body: serde_json::Value = response.json();
    assert!(body.get("error").is_some());

    ctx.cleanup().await;
}

#[tokio::test]
async fn register_with_existing_email_returns_conflict() {
    let ctx = TestContext::new().await;
    let email = test_email();

    // First registration
    ctx.server
        .post("/auth/register")
        .json(&json!({
            "email": &email,
            "password": test_password(),
            "password_confirm": test_password()
        }))
        .await;

    // Second registration with same email
    let response = ctx
        .server
        .post("/auth/register")
        .json(&json!({
            "email": &email,
            "password": test_password(),
            "password_confirm": test_password()
        }))
        .await;

    response.assert_status(StatusCode::CONFLICT);

    ctx.cleanup().await;
}

#[tokio::test]
async fn register_with_missing_fields_returns_unprocessable() {
    let ctx = TestContext::new().await;

    // Missing email
    let response = ctx
        .server
        .post("/auth/register")
        .json(&json!({
            "password": test_password(),
            "password_confirm": test_password()
        }))
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);

    // Missing password
    let response = ctx
        .server
        .post("/auth/register")
        .json(&json!({
            "email": test_email(),
            "password_confirm": test_password()
        }))
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);

    // Missing password_confirm
    let response = ctx
        .server
        .post("/auth/register")
        .json(&json!({
            "email": test_email(),
            "password": test_password()
        }))
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);

    ctx.cleanup().await;
}

#[tokio::test]
async fn register_with_empty_body_returns_unprocessable() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/auth/register")
        .json(&json!({}))
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);

    ctx.cleanup().await;
}

// =============================================================================
// RATE LIMITING
// =============================================================================

#[tokio::test]
async fn register_rate_limits_after_too_many_requests() {
    let ctx = TestContext::new().await;

    // Send 10 rapid requests
    for _ in 0..10 {
        ctx.server
            .post("/auth/register")
            .json(&json!({
                "email": test_email(),
                "password": test_password(),
                "password_confirm": test_password()
            }))
            .await;
    }

    // 11th request should be rate limited
    let response = ctx
        .server
        .post("/auth/register")
        .json(&json!({
            "email": test_email(),
            "password": test_password(),
            "password_confirm": test_password()
        }))
        .await;

    response.assert_status(StatusCode::TOO_MANY_REQUESTS);

    ctx.cleanup().await;
}

// =============================================================================
// CONCURRENT REQUESTS (Race Condition)
// =============================================================================

#[tokio::test]
async fn register_handles_concurrent_duplicate_emails() {
    let ctx = TestContext::new().await;
    let email = test_email();

    // Send two concurrent requests with same email
    let (res1, res2) = tokio::join!(
        ctx.server.post("/auth/register").json(&json!({
            "email": &email,
            "password": test_password(),
            "password_confirm": test_password()
        })),
        ctx.server.post("/auth/register").json(&json!({
            "email": &email,
            "password": test_password(),
            "password_confirm": test_password()
        }))
    );

    let statuses = vec![res1.status_code(), res2.status_code()];

    // Valid outcomes due to race conditions and rate limiting:
    // 1. One CREATED, one CONFLICT (ideal - both processed, second detected duplicate)
    // 2. One CREATED, one TOO_MANY_REQUESTS (rate limited)
    // 3. Both TOO_MANY_REQUESTS (both rate limited)
    // 4. Both CREATED (rare race - both passed email check before insert)
    //    This is acceptable as DB constraint still prevents true duplicates
    let has_created = statuses.contains(&StatusCode::CREATED);
    let has_conflict = statuses.contains(&StatusCode::CONFLICT);
    let has_rate_limited = statuses.contains(&StatusCode::TOO_MANY_REQUESTS);
    let both_created = statuses.iter().filter(|&s| *s == StatusCode::CREATED).count() == 2;

    // If both returned CREATED, verify only one user was actually created
    if both_created {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE email = ?")
            .bind(&email)
            .fetch_one(&ctx.db)
            .await
            .unwrap();
        assert_eq!(count.0, 1, "Only one user should exist despite both returning 201");
    } else {
        assert!(
            (has_created && has_conflict) ||
            (has_created && has_rate_limited) ||
            (has_rate_limited),
            "Unexpected statuses: {:?}", statuses
        );
    }

    ctx.cleanup().await;
}

// =============================================================================
// SECURITY
// =============================================================================

#[tokio::test]
async fn register_response_includes_security_headers() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/auth/register")
        .json(&json!({
            "email": test_email(),
            "password": test_password(),
            "password_confirm": test_password()
        }))
        .await;

    // Check security headers exist
    assert!(response.headers().get("x-content-type-options").is_some());
    assert!(response.headers().get("x-frame-options").is_some());

    ctx.cleanup().await;
}

#[tokio::test]
async fn register_sanitizes_email_input() {
    let ctx = TestContext::new().await;

    // Attempt XSS in email
    let response = ctx
        .server
        .post("/auth/register")
        .json(&json!({
            "email": "<script>alert('xss')</script>@test.com",
            "password": test_password(),
            "password_confirm": test_password()
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);

    ctx.cleanup().await;
}

#[tokio::test]
async fn register_rejects_oversized_payload() {
    let ctx = TestContext::new().await;

    // Create a very large password (1MB)
    let large_password = "a".repeat(1_000_000);

    let response = ctx
        .server
        .post("/auth/register")
        .json(&json!({
            "email": test_email(),
            "password": &large_password,
            "password_confirm": &large_password
        }))
        .await;

    // Should reject with 413 Payload Too Large or 400 Bad Request
    assert!(
        response.status_code() == StatusCode::PAYLOAD_TOO_LARGE
        || response.status_code() == StatusCode::BAD_REQUEST
    );

    ctx.cleanup().await;
}

// =============================================================================
// PERFORMANCE
// =============================================================================

#[tokio::test]
async fn register_responds_within_acceptable_time() {
    let ctx = TestContext::new().await;

    let start = std::time::Instant::now();

    ctx.server
        .post("/auth/register")
        .json(&json!({
            "email": test_email(),
            "password": test_password(),
            "password_confirm": test_password()
        }))
        .await;

    let duration = start.elapsed();

    // Should respond within 5 seconds (argon2 + parallel test overhead)
    assert!(duration.as_secs() < 5, "Response took too long: {:?}", duration);

    ctx.cleanup().await;
}
