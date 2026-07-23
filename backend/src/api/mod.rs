//! HTTP API surface. Each submodule owns a slice of routes; [`router`] wires
//! them together with the shared [`AppState`].

pub mod captures;

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::post;

use crate::state::AppState;

/// Upper bound on a capture request body (payload JSON + image blobs).
const CAPTURE_BODY_LIMIT: usize = 64 * 1024 * 1024;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route(
            "/api/captures",
            post(captures::create_capture).layer(DefaultBodyLimit::max(CAPTURE_BODY_LIMIT)),
        )
        .with_state(state)
}
