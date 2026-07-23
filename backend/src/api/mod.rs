//! HTTP API surface. Each submodule owns a slice of routes; [`router`] wires
//! them together with the shared [`AppState`].

pub mod captures;
pub mod dto;
pub mod highlights;
pub mod pages;
pub mod stats;

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::{get, patch, post};

use crate::state::AppState;

/// Upper bound on a capture request body (payload JSON + image blobs).
const CAPTURE_BODY_LIMIT: usize = 64 * 1024 * 1024;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route(
            "/api/captures",
            post(captures::create_capture).layer(DefaultBodyLimit::max(CAPTURE_BODY_LIMIT)),
        )
        .route("/api/pages", get(pages::list))
        .route("/api/pages/{id}", get(pages::get).delete(pages::delete))
        .route("/api/highlights", get(highlights::list))
        .route("/api/highlights/{id}", patch(highlights::patch).delete(highlights::delete))
        .route("/api/stats", get(stats::get))
        .with_state(state)
}
