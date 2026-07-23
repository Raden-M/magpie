//! Bearer-token auth, CORS, and the constant-time token compare.
//!
//! Auth is enforced whenever a token is set or the bind is non-loopback (see
//! [`crate::config::Config::auth_required`]). When enforced it covers every
//! route except liveness and static assets - so a remote instance never leaks
//! the hoard by default. Browsers cannot send a bearer header on navigation, so
//! remote *human* access to the web UI is expected to be fronted by the reverse
//! proxy (docs/08-deployment.md); the token guards the extension's API path.

use axum::extract::{Request, State};
use axum::http::header::{AUTHORIZATION, CONTENT_TYPE};
use axum::http::{HeaderValue, Method};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use tower_http::cors::{AllowOrigin, CorsLayer};

use crate::error::AppError;
use crate::state::AppState;

/// Paths reachable without a token even when auth is on.
fn is_exempt(path: &str) -> bool {
    path == "/healthz" || path.starts_with("/assets/")
}

/// Length-checked constant-time byte comparison.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b) {
        diff |= x ^ y;
    }
    diff == 0
}

pub async fn require_auth(State(state): State<AppState>, req: Request, next: Next) -> Response {
    if !state.config.auth_required() || is_exempt(req.uri().path()) {
        return next.run(req).await;
    }

    let Some(expected) = state.config.token.as_deref() else {
        // auth_required() is true on non-loopback even without a token, but
        // startup refuses that combination - so reaching here means misconfig.
        return AppError::Unauthorized.into_response();
    };

    let presented = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    match presented {
        Some(token) if constant_time_eq(token.as_bytes(), expected.as_bytes()) => {
            next.run(req).await
        }
        _ => AppError::Unauthorized.into_response(),
    }
}

/// CORS for the extension's cross-origin calls. Only the configured extension
/// origins are allowed; there is no wildcard. With no origins configured the
/// layer adds no cross-origin permission (the extension can instead use
/// `host_permissions`), while same-origin web-UI requests are unaffected.
pub fn cors_layer(origins: &[String]) -> CorsLayer {
    let layer = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_headers([AUTHORIZATION, CONTENT_TYPE]);

    let parsed: Vec<HeaderValue> =
        origins.iter().filter_map(|o| o.trim().parse::<HeaderValue>().ok()).collect();

    if parsed.is_empty() {
        layer
    } else {
        layer.allow_origin(AllowOrigin::list(parsed))
    }
}
