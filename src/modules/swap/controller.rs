use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;

use crate::AppState;
use super::crud::{SwapCrud, SwapError};
use super::schema::{
    ProvidersQuery, ProviderResponse, ProviderDetailResponse, SwapErrorResponse,
};

// =============================================================================
// GET /swap/providers - List exchange providers
// =============================================================================

pub async fn get_providers(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ProvidersQuery>,
) -> Result<Json<Vec<ProviderResponse>>, (StatusCode, Json<SwapErrorResponse>)> {
    let crud = SwapCrud::new(state.db.clone());

    let providers = crud.get_providers(query).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(SwapErrorResponse::new(e.to_string())),
        )
    })?;

    Ok(Json(providers))
}

// =============================================================================
// GET /swap/providers/:id - Get single provider details
// =============================================================================

pub async fn get_provider(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ProviderDetailResponse>, (StatusCode, Json<SwapErrorResponse>)> {
    let crud = SwapCrud::new(state.db.clone());

    let provider = crud.get_provider_by_id(&id).await.map_err(|e| {
        match e {
            SwapError::ProviderNotFound => (
                StatusCode::NOT_FOUND,
                Json(SwapErrorResponse::with_code("Provider not found", "PROVIDER_NOT_FOUND")),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(SwapErrorResponse::new(e.to_string())),
            ),
        }
    })?;

    Ok(Json(provider))
}
