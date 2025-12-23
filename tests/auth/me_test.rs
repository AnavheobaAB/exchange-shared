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

#[tokio::test]
async fn me_with_valid_token_returns_user_data() {
    let ctx = TestContext::new().await;
    let (email, access_token) = create_and_login(&ctx).await;

    let response = ctx
        .server
        .get("/auth/me")
        .authorization_bearer(&access_token)
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["email"], email);
    assert!(body.get("id").is_some());
    assert!(body.get("created_at").is_some());
    assert!(body.get("password").is_none()); // Password should never be returned

    ctx.cleanup().await;
}

#[tokio::test]
async fn me_without_auth_header_returns_unauthorized() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/auth/me")
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);

    ctx.cleanup().await;
}

#[tokio::test]
async fn me_with_invalid_token_returns_unauthorized() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/auth/me")
        .authorization_bearer("invalid-token")
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);

    ctx.cleanup().await;
}

#[tokio::test]
async fn me_with_malformed_auth_header_returns_unauthorized() {
    let ctx = TestContext::new().await;

    // Test with empty token
    let response = ctx
        .server
        .get("/auth/me")
        .authorization_bearer("")
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);

    ctx.cleanup().await;
}

#[tokio::test]
async fn me_returns_2fa_status() {
    let ctx = TestContext::new().await;
    let (_, access_token) = create_and_login(&ctx).await;

    let response = ctx
        .server
        .get("/auth/me")
        .authorization_bearer(&access_token)
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert!(body.get("two_factor_enabled").is_some());
    assert_eq!(body["two_factor_enabled"], false);

    ctx.cleanup().await;
}

#[tokio::test]
async fn me_does_not_return_sensitive_data() {
    let ctx = TestContext::new().await;
    let (_, access_token) = create_and_login(&ctx).await;

    let response = ctx
        .server
        .get("/auth/me")
        .authorization_bearer(&access_token)
        .await;

    let body: serde_json::Value = response.json();

    // Should NOT return these fields
    assert!(body.get("password").is_none());
    assert!(body.get("password_hash").is_none());
    assert!(body.get("two_factor_secret").is_none());

    ctx.cleanup().await;
}
