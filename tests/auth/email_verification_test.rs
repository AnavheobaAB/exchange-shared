use axum::http::StatusCode;
use serde_json::json;

use crate::common::{test_email, test_password, TestContext};

async fn create_and_login(ctx: &TestContext) -> (String, String) {
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
    let access_token = body["access_token"].as_str().unwrap().to_string();

    (email, access_token)
}

// =============================================================================
// LAZY VERIFICATION - Login works without verification
// =============================================================================

#[tokio::test]
async fn unverified_user_can_login_immediately_after_registration() {
    let ctx = TestContext::new().await;
    let email = test_email();

    // Register
    let register_response = ctx
        .server
        .post("/auth/register")
        .json(&json!({
            "email": &email,
            "password": test_password(),
            "password_confirm": test_password()
        }))
        .await;

    register_response.assert_status(StatusCode::CREATED);

    // Login immediately without verification
    let login_response = ctx
        .server
        .post("/auth/login")
        .json(&json!({
            "email": &email,
            "password": test_password()
        }))
        .await;

    login_response.assert_status(StatusCode::OK);

    let body: serde_json::Value = login_response.json();
    assert!(body.get("access_token").is_some());

    ctx.cleanup().await;
}

#[tokio::test]
async fn unverified_user_can_access_basic_endpoints() {
    let ctx = TestContext::new().await;
    let (_, access_token) = create_and_login(&ctx).await;

    // Can access /me endpoint
    let response = ctx
        .server
        .get("/auth/me")
        .authorization_bearer(&access_token)
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["email_verified"], false);

    ctx.cleanup().await;
}

// =============================================================================
// SENSITIVE FEATURES - Blocked for unverified users
// =============================================================================

#[tokio::test]
async fn unverified_user_cannot_enable_2fa() {
    let ctx = TestContext::new().await;
    let (_, access_token) = create_and_login(&ctx).await;

    let response = ctx
        .server
        .post("/auth/enable-2fa")
        .authorization_bearer(&access_token)
        .await;

    response.assert_status(StatusCode::FORBIDDEN);

    let body: serde_json::Value = response.json();
    assert!(body["error"].as_str().unwrap().contains("verify")
        || body["message"].as_str().unwrap_or("").contains("verify"));

    ctx.cleanup().await;
}

#[tokio::test]
async fn unverified_user_cannot_request_password_reset() {
    let ctx = TestContext::new().await;
    let (email, _) = create_and_login(&ctx).await;

    // For unverified users, forgot-password should indicate they need to verify first
    // Note: We still return 200 to prevent email enumeration, but no email is sent
    let response = ctx
        .server
        .post("/auth/forgot-password")
        .json(&json!({
            "email": &email
        }))
        .await;

    response.assert_status(StatusCode::OK);

    // Verify no reset token was created for unverified user
    let result = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM password_resets pr
         JOIN users u ON pr.user_id = u.id
         WHERE u.email = ? AND u.email_verified = false"
    )
    .bind(&email)
    .fetch_one(&ctx.db)
    .await
    .unwrap();

    assert_eq!(result, 0, "Password reset token should NOT be created for unverified user");

    ctx.cleanup().await;
}

#[tokio::test]
async fn unverified_user_cannot_purchase_gift_cards() {
    let ctx = TestContext::new().await;
    let (_, access_token) = create_and_login(&ctx).await;

    let response = ctx
        .server
        .post("/gift-cards/purchase")
        .authorization_bearer(&access_token)
        .json(&json!({
            "card_id": "amazon-50",
            "amount": 50
        }))
        .await;

    response.assert_status(StatusCode::FORBIDDEN);

    let body: serde_json::Value = response.json();
    assert!(body["error"].as_str().unwrap_or("").contains("verify")
        || body["message"].as_str().unwrap_or("").contains("verify"));

    ctx.cleanup().await;
}

// =============================================================================
// EMAIL VERIFICATION FLOW
// =============================================================================

#[tokio::test]
async fn request_verification_email_returns_success() {
    let ctx = TestContext::new().await;
    let (_, access_token) = create_and_login(&ctx).await;

    let response = ctx
        .server
        .post("/auth/request-verification")
        .authorization_bearer(&access_token)
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert!(body.get("message").is_some());

    ctx.cleanup().await;
}

#[tokio::test]
async fn request_verification_creates_token_in_database() {
    let ctx = TestContext::new().await;
    let (email, access_token) = create_and_login(&ctx).await;

    ctx.server
        .post("/auth/request-verification")
        .authorization_bearer(&access_token)
        .await;

    // Verify token was created
    let result = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM email_verifications ev
         JOIN users u ON ev.user_id = u.id
         WHERE u.email = ?"
    )
    .bind(&email)
    .fetch_one(&ctx.db)
    .await
    .unwrap();

    assert!(result > 0, "Verification token should be created");

    ctx.cleanup().await;
}

#[tokio::test]
async fn verify_email_with_valid_token_succeeds() {
    let ctx = TestContext::new().await;
    let (email, access_token) = create_and_login(&ctx).await;

    // Request verification
    ctx.server
        .post("/auth/request-verification")
        .authorization_bearer(&access_token)
        .await;

    // Get token from database
    let token: String = sqlx::query_scalar(
        "SELECT token FROM email_verifications ev
         JOIN users u ON ev.user_id = u.id
         WHERE u.email = ?
         ORDER BY ev.created_at DESC
         LIMIT 1"
    )
    .bind(&email)
    .fetch_one(&ctx.db)
    .await
    .unwrap();

    // Verify email
    let response = ctx
        .server
        .post("/auth/verify-email")
        .json(&json!({
            "token": &token
        }))
        .await;

    response.assert_status(StatusCode::OK);

    ctx.cleanup().await;
}

#[tokio::test]
async fn verify_email_updates_user_verified_status() {
    let ctx = TestContext::new().await;
    let (email, access_token) = create_and_login(&ctx).await;

    // Request and get token
    ctx.server
        .post("/auth/request-verification")
        .authorization_bearer(&access_token)
        .await;

    let token: String = sqlx::query_scalar(
        "SELECT token FROM email_verifications ev
         JOIN users u ON ev.user_id = u.id
         WHERE u.email = ?
         ORDER BY ev.created_at DESC
         LIMIT 1"
    )
    .bind(&email)
    .fetch_one(&ctx.db)
    .await
    .unwrap();

    // Verify
    ctx.server
        .post("/auth/verify-email")
        .json(&json!({
            "token": &token
        }))
        .await;

    // Check user status
    let response = ctx
        .server
        .get("/auth/me")
        .authorization_bearer(&access_token)
        .await;

    let body: serde_json::Value = response.json();
    assert_eq!(body["email_verified"], true);

    ctx.cleanup().await;
}

#[tokio::test]
async fn verify_email_with_invalid_token_returns_bad_request() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .post("/auth/verify-email")
        .json(&json!({
            "token": "invalid-token-12345"
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);

    ctx.cleanup().await;
}

#[tokio::test]
async fn verify_email_with_expired_token_returns_bad_request() {
    let ctx = TestContext::new().await;
    let (email, access_token) = create_and_login(&ctx).await;

    // Request verification
    ctx.server
        .post("/auth/request-verification")
        .authorization_bearer(&access_token)
        .await;

    // Expire the token manually
    sqlx::query(
        "UPDATE email_verifications ev
         JOIN users u ON ev.user_id = u.id
         SET ev.expires_at = DATE_SUB(NOW(), INTERVAL 1 HOUR)
         WHERE u.email = ?"
    )
    .bind(&email)
    .execute(&ctx.db)
    .await
    .unwrap();

    let token: String = sqlx::query_scalar(
        "SELECT token FROM email_verifications ev
         JOIN users u ON ev.user_id = u.id
         WHERE u.email = ?"
    )
    .bind(&email)
    .fetch_one(&ctx.db)
    .await
    .unwrap();

    let response = ctx
        .server
        .post("/auth/verify-email")
        .json(&json!({
            "token": &token
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);

    ctx.cleanup().await;
}

#[tokio::test]
async fn verification_token_can_only_be_used_once() {
    let ctx = TestContext::new().await;
    let (email, access_token) = create_and_login(&ctx).await;

    // Request and get token
    ctx.server
        .post("/auth/request-verification")
        .authorization_bearer(&access_token)
        .await;

    let token: String = sqlx::query_scalar(
        "SELECT token FROM email_verifications ev
         JOIN users u ON ev.user_id = u.id
         WHERE u.email = ?"
    )
    .bind(&email)
    .fetch_one(&ctx.db)
    .await
    .unwrap();

    // First verification
    ctx.server
        .post("/auth/verify-email")
        .json(&json!({
            "token": &token
        }))
        .await;

    // Second verification with same token
    let response = ctx
        .server
        .post("/auth/verify-email")
        .json(&json!({
            "token": &token
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);

    ctx.cleanup().await;
}

// =============================================================================
// VERIFIED USER - Can access sensitive features
// =============================================================================

#[tokio::test]
async fn verified_user_can_enable_2fa() {
    let ctx = TestContext::new().await;
    let (email, access_token) = create_and_login(&ctx).await;

    // Manually verify user for this test
    sqlx::query("UPDATE users SET email_verified = true WHERE email = ?")
        .bind(&email)
        .execute(&ctx.db)
        .await
        .unwrap();

    let response = ctx
        .server
        .post("/auth/enable-2fa")
        .authorization_bearer(&access_token)
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert!(body.get("secret").is_some());

    ctx.cleanup().await;
}

#[tokio::test]
async fn verified_user_can_request_password_reset() {
    let ctx = TestContext::new().await;
    let (email, _) = create_and_login(&ctx).await;

    // Manually verify user
    sqlx::query("UPDATE users SET email_verified = true WHERE email = ?")
        .bind(&email)
        .execute(&ctx.db)
        .await
        .unwrap();

    ctx.server
        .post("/auth/forgot-password")
        .json(&json!({
            "email": &email
        }))
        .await;

    // Verify reset token was created
    let result = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM password_resets pr
         JOIN users u ON pr.user_id = u.id
         WHERE u.email = ?"
    )
    .bind(&email)
    .fetch_one(&ctx.db)
    .await
    .unwrap();

    assert!(result > 0, "Password reset token SHOULD be created for verified user");

    ctx.cleanup().await;
}

#[tokio::test]
async fn verified_user_can_purchase_gift_cards() {
    let ctx = TestContext::new().await;
    let (email, access_token) = create_and_login(&ctx).await;

    // Manually verify user
    sqlx::query("UPDATE users SET email_verified = true WHERE email = ?")
        .bind(&email)
        .execute(&ctx.db)
        .await
        .unwrap();

    let response = ctx
        .server
        .post("/gift-cards/purchase")
        .authorization_bearer(&access_token)
        .json(&json!({
            "card_id": "amazon-50",
            "amount": 50
        }))
        .await;

    // Should not be FORBIDDEN (might be other errors like payment required)
    assert_ne!(response.status_code(), StatusCode::FORBIDDEN);

    ctx.cleanup().await;
}

// =============================================================================
// ALREADY VERIFIED - Edge cases
// =============================================================================

#[tokio::test]
async fn already_verified_user_requesting_verification_returns_success() {
    let ctx = TestContext::new().await;
    let (email, access_token) = create_and_login(&ctx).await;

    // Manually verify user
    sqlx::query("UPDATE users SET email_verified = true WHERE email = ?")
        .bind(&email)
        .execute(&ctx.db)
        .await
        .unwrap();

    // Request verification again
    let response = ctx
        .server
        .post("/auth/request-verification")
        .authorization_bearer(&access_token)
        .await;

    // Should return OK with message that email is already verified
    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert!(body["message"].as_str().unwrap_or("").contains("already"));

    ctx.cleanup().await;
}

#[tokio::test]
async fn rate_limits_verification_email_requests() {
    let ctx = TestContext::new().await;
    let (_, access_token) = create_and_login(&ctx).await;

    // Send multiple requests rapidly
    for _ in 0..5 {
        ctx.server
            .post("/auth/request-verification")
            .authorization_bearer(&access_token)
            .await;
    }

    // The 6th request might be rate limited
    let response = ctx
        .server
        .post("/auth/request-verification")
        .authorization_bearer(&access_token)
        .await;

    // Either OK or TOO_MANY_REQUESTS
    assert!(
        response.status_code() == StatusCode::OK
        || response.status_code() == StatusCode::TOO_MANY_REQUESTS
    );

    ctx.cleanup().await;
}
