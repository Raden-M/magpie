//! Magpie backend library: storage, search, reader rendering, and the HTTP
//! surface. The binary (`main.rs`) is a thin wrapper that parses config, runs
//! startup checks, and serves [`app`]. Exposing the router as a library lets the
//! integration tests drive it in-process via `tower::ServiceExt::oneshot`.

pub mod api;
pub mod config;
pub mod cursor;
pub mod db;
pub mod error;
pub mod reader;
pub mod state;
pub mod url_canon;
pub mod util;
pub mod web;

use axum::Router;
use axum::routing::get;

use crate::state::AppState;

/// Assemble the full application router: liveness plus the API and web UI.
pub fn app(state: AppState) -> Router {
    Router::new().route("/healthz", get(healthz)).merge(api::router(state))
}

async fn healthz() -> &'static str {
    "ok"
}
