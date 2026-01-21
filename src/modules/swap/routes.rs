use axum::{routing::get, Router};
use std::sync::Arc;

use crate::AppState;
use super::controller::{get_currencies, get_providers};

pub fn swap_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/currencies", get(get_currencies))
        .route("/providers", get(get_providers))
        // Other routes to be added later...
}
