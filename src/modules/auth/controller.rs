use axum::{extract::State, http::StatusCode, Json};
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;
use validator::Validate;

use crate::AppState;
use crate::modules::auth::{
    crud::{AuthError, UserCrud},
    model::User,
    schema::{LoginRequest, LoginResponse, RegisterRequest, RegisterResponse, UserResponse, ErrorResponse},
};
use crate::services::hashing;

pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<RegisterResponse>), (StatusCode, Json<ErrorResponse>)> {
    if let Err(e) = req.validate() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(e.to_string())),
        ));
    }

    if req.password != req.password_confirm {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("Passwords do not match")),
        ));
    }

    if req.password.len() < 8 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("Password must be at least 8 characters")),
        ));
    }

    let crud = UserCrud::new(state.db.clone(), &state.jwt_service);

    if crud.email_exists(&req.email).await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new(e.to_string())))
    })? {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse::new("Email already exists")),
        ));
    }

    let password_hash = hashing::hash_password(&req.password).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new(e.to_string())))
    })?;

    let now = Utc::now();
    let user = User {
        id: Uuid::new_v4().to_string(),
        email: req.email.clone(),
        password_hash,
        email_verified: false,
        two_factor_enabled: false,
        two_factor_secret: None,
        created_at: now,
        updated_at: now,
    };

    if let Err(e) = crud.create(&user).await {
        // Check for duplicate key error (MySQL error 1062)
        let err_str = e.to_string();
        if err_str.contains("Duplicate entry") || err_str.contains("1062") {
            return Err((
                StatusCode::CONFLICT,
                Json(ErrorResponse::new("Email already exists")),
            ));
        }
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(err_str)),
        ));
    }

    Ok((
        StatusCode::CREATED,
        Json(RegisterResponse {
            user: UserResponse {
                id: user.id,
                email: user.email,
                email_verified: user.email_verified,
                two_factor_enabled: user.two_factor_enabled,
                created_at: user.created_at,
                updated_at: user.updated_at,
            },
        }),
    ))
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> Result<(StatusCode, Json<LoginResponse>), (StatusCode, Json<ErrorResponse>)> {
    let crud = UserCrud::new(state.db.clone(), &state.jwt_service);

    let result = crud.login(&req.email, &req.password).await.map_err(|e| {
        match e {
            AuthError::InvalidCredentials => (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new("Invalid email or password")),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(e.to_string())),
            ),
        }
    })?;

    Ok((
        StatusCode::OK,
        Json(LoginResponse {
            access_token: result.access_token,
            refresh_token: result.refresh_token,
            token_type: "Bearer",
            expires_in: result.expires_in,
        }),
    ))
}
