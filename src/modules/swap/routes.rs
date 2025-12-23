use axum::{routing::get, Router};
use std::sync::Arc;

use crate::AppState;
use super::controller;

pub fn swap_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/providers", get(controller::get_providers))
        .route("/providers/{id}", get(controller::get_provider))
}
