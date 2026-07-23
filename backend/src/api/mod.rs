//! HTTP API surface. Each submodule owns a slice of routes; [`router`] wires
//! them together with the shared [`AppState`].

pub mod auth;
pub mod captures;
pub mod dto;
pub mod highlights;
pub mod media;
pub mod pages;
pub mod search;
pub mod stats;

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::middleware;
use axum::routing::{get, patch, post};

use crate::state::AppState;
use crate::web;

/// Upper bound on a capture request body (payload JSON + image blobs).
const CAPTURE_BODY_LIMIT: usize = 64 * 1024 * 1024;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(web::collection::page))
        .route("/assets/app.css", get(web::assets::app_css))
        .route("/assets/reader.js", get(web::assets::reader_js))
        .route(
            "/api/captures",
            post(captures::create_capture).layer(DefaultBodyLimit::max(CAPTURE_BODY_LIMIT)),
        )
        .route("/api/pages", get(pages::list))
        .route("/api/pages/{id}", get(pages::get).delete(pages::delete))
        .route("/api/pages/{id}/reader", get(web::reader::reader))
        .route("/api/pages/{id}/snapshot", get(web::reader::snapshot_raw))
        .route("/api/highlights", get(highlights::list))
        .route(
            "/api/highlights/{id}",
            patch(highlights::patch).delete(highlights::delete),
        )
        .route("/api/search", get(search::search))
        .route("/api/stats", get(stats::get))
        .route("/search", get(web::search::page))
        .route("/media/{hash}", get(media::serve))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_auth,
        ))
        .layer(auth::cors_layer(&state.config.allowed_origins))
        .with_state(state)
}
