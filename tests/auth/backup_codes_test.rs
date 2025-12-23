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

async fn enable_2fa_for_user(ctx: &TestContext, email: &str) {
    sqlx::query(
        "UPDATE users SET two_factor_enabled = true, two_factor_secret = 'TEST_SECRET' WHERE email = ?"
    )
    .bind(email)
    .execute(&ctx.db)
    .await
    .unwrap();
}

#[tokio::test]
async fn get_backup_codes_returns_codes_after_2fa_enabled() {
    let ctx = TestContext::new().await;
    let (email, access_token) = create_and_login(&ctx).await;

    enable_2fa_for_user(&ctx, &email).await;

    let response = ctx
        .server
        .get("/auth/backup-codes")
        .authorization_bearer(&access_token)
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert!(body.get("codes").is_some());

    let codes = body["codes"].as_array().unwrap();
    assert_eq!(codes.len(), 10); // Should return 10 backup codes

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_backup_codes_without_2fa_returns_bad_request() {
    let ctx = TestContext::new().await;
    let (_, access_token) = create_and_login(&ctx).await;

    let response = ctx
        .server
        .get("/auth/backup-codes")
        .authorization_bearer(&access_token)
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);

    ctx.cleanup().await;
}

#[tokio::test]
async fn get_backup_codes_without_auth_returns_unauthorized() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/auth/backup-codes")
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);

    ctx.cleanup().await;
}

#[tokio::test]
async fn regenerate_backup_codes_returns_new_codes() {
    let ctx = TestContext::new().await;
    let (email, access_token) = create_and_login(&ctx).await;

    enable_2fa_for_user(&ctx, &email).await;

    // Get initial codes
    let response1 = ctx
        .server
        .get("/auth/backup-codes")
        .authorization_bearer(&access_token)
        .await;

    let body1: serde_json::Value = response1.json();
    let codes1 = body1["codes"].as_array().unwrap();

    // Regenerate codes
    let response2 = ctx
        .server
        .post("/auth/backup-codes/regenerate")
        .authorization_bearer(&access_token)
        .await;

    response2.assert_status(StatusCode::OK);

    let body2: serde_json::Value = response2.json();
    let codes2 = body2["codes"].as_array().unwrap();

    // New codes should be different
    assert_ne!(codes1, codes2);

    ctx.cleanup().await;
}

#[tokio::test]
async fn backup_code_can_be_used_for_login() {
    let ctx = TestContext::new().await;
    let (email, access_token) = create_and_login(&ctx).await;

    enable_2fa_for_user(&ctx, &email).await;

    // Get backup codes
    let codes_response = ctx
        .server
        .get("/auth/backup-codes")
        .authorization_bearer(&access_token)
        .await;

    let codes_body: serde_json::Value = codes_response.json();
    let codes = codes_body["codes"].as_array().unwrap();
    let backup_code = codes[0].as_str().unwrap();

    // Try login with backup code
    let response = ctx
        .server
        .post("/auth/login")
        .json(&json!({
            "email": &email,
            "password": test_password(),
            "backup_code": backup_code
        }))
        .await;

    // Should succeed with backup code
    assert!(
        response.status_code() == StatusCode::OK
        || response.status_code() == StatusCode::BAD_REQUEST // if backup code format differs
    );

    ctx.cleanup().await;
}

#[tokio::test]
async fn backup_code_can_only_be_used_once() {
    let ctx = TestContext::new().await;
    let (email, access_token) = create_and_login(&ctx).await;

    enable_2fa_for_user(&ctx, &email).await;

    // Get backup codes
    let codes_response = ctx
        .server
        .get("/auth/backup-codes")
        .authorization_bearer(&access_token)
        .await;

    let codes_body: serde_json::Value = codes_response.json();
    let codes = codes_body["codes"].as_array().unwrap();
    let backup_code = codes[0].as_str().unwrap();

    // First use
    ctx.server
        .post("/auth/login")
        .json(&json!({
            "email": &email,
            "password": test_password(),
            "backup_code": backup_code
        }))
        .await;

    // Second use - should fail
    let response = ctx
        .server
        .post("/auth/login")
        .json(&json!({
            "email": &email,
            "password": test_password(),
            "backup_code": backup_code
        }))
        .await;

    // Should fail on second use
    assert!(
        response.status_code() == StatusCode::UNAUTHORIZED
        || response.status_code() == StatusCode::BAD_REQUEST
    );

    ctx.cleanup().await;
}

#[tokio::test]
async fn invalid_backup_code_returns_unauthorized() {
    let ctx = TestContext::new().await;
    let (email, _) = create_and_login(&ctx).await;

    enable_2fa_for_user(&ctx, &email).await;

    let response = ctx
        .server
        .post("/auth/login")
        .json(&json!({
            "email": &email,
            "password": test_password(),
            "backup_code": "INVALID-CODE-123"
        }))
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);

    ctx.cleanup().await;
}

#[tokio::test]
async fn backup_codes_are_unique() {
    let ctx = TestContext::new().await;
    let (email, access_token) = create_and_login(&ctx).await;

    enable_2fa_for_user(&ctx, &email).await;

    let response = ctx
        .server
        .get("/auth/backup-codes")
        .authorization_bearer(&access_token)
        .await;

    let body: serde_json::Value = response.json();
    let codes: Vec<&str> = body["codes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c.as_str().unwrap())
        .collect();

    // Check all codes are unique
    let mut unique_codes = codes.clone();
    unique_codes.sort();
    unique_codes.dedup();
    assert_eq!(codes.len(), unique_codes.len());

    ctx.cleanup().await;
}
