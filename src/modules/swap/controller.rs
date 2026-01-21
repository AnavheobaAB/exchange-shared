use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;

use crate::AppState;
use super::crud::{SwapCrud};
use super::schema::{
    CurrenciesQuery, CurrencyResponse, ProvidersQuery, ProviderResponse, SwapErrorResponse,
};
use crate::services::trocador::TrocadorClient;

// =============================================================================
// GET /swap/currencies - List all currencies
// =============================================================================

pub async fn get_currencies(
    State(state): State<Arc<AppState>>,
    Query(query): Query<CurrenciesQuery>,
) -> Result<Json<Vec<CurrencyResponse>>, (StatusCode, Json<SwapErrorResponse>)> {
    let crud = SwapCrud::new(state.db.clone());

    // Check if we need to sync from Trocador
    let should_sync = crud.should_sync_currencies().await.unwrap_or(true);

    if should_sync {
        // Get API key from environment
        let api_key = std::env::var("TROCADOR_API_KEY").unwrap_or_default();
        
        if !api_key.is_empty() {
            let trocador_client = TrocadorClient::new(api_key);
            
            // Sync in background, don't fail request if sync fails
            if let Err(e) = crud.sync_currencies_from_trocador(&trocador_client).await {
                tracing::warn!("Failed to sync currencies from Trocador: {}", e);
            }
        }
    }

    // Get currencies from database (cache)
    let currencies = crud.get_currencies(query).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(SwapErrorResponse::new(e.to_string())),
        )
    })?;

    // Convert to response format
    let responses: Vec<CurrencyResponse> = currencies
        .into_iter()
        .map(|c| c.into())
        .collect();

    Ok(Json(responses))
}

// =============================================================================
// GET /swap/providers - List all exchange providers
// =============================================================================

pub async fn get_providers(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ProvidersQuery>,
) -> Result<Json<Vec<ProviderResponse>>, (StatusCode, Json<SwapErrorResponse>)> {
    let crud = SwapCrud::new(state.db.clone());

    // Check if we need to sync from Trocador
    let should_sync = crud.should_sync_providers().await.unwrap_or(true);

    if should_sync {
        let api_key = std::env::var("TROCADOR_API_KEY").unwrap_or_default();
        
        if !api_key.is_empty() {
            let trocador_client = TrocadorClient::new(api_key);
            
            if let Err(e) = crud.sync_providers_from_trocador(&trocador_client).await {
                tracing::warn!("Failed to sync providers from Trocador: {}", e);
            }
        }
    }

    // Get providers from database (cache)
    let providers = crud.get_providers(query).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(SwapErrorResponse::new(e.to_string())),
        )
    })?;

    // Convert to response format
    let responses: Vec<ProviderResponse> = providers
        .into_iter()
        .map(|p| p.into())
        .collect();

    Ok(Json(responses))
}
