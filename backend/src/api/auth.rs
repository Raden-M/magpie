//! Bearer-token auth, CORS, and the constant-time token compare.
//!
//! Auth is enforced whenever a token is set or the bind is non-loopback (see
//! [`crate::config::Config::auth_required`]). When enforced it covers every
//! route except liveness, static assets and the login form - so a remote
//! instance never leaks the hoard by default.
//!
//! There are two credential shapes because there have to be. The extension
//! sends the configured token as a bearer header, which is the only thing it
//! can do. A browser cannot: nothing attaches an `Authorization` header to
//! ordinary navigation, so a token-protected instance would render its own
//! owner a JSON 401 on every page. Browsers therefore trade the token for a
//! session cookie at `/login` (see [`crate::web::login`]), and either
//! credential satisfies this middleware.

use axum::extract::{Request, State};
use axum::http::HeaderMap;
use axum::http::header::{AUTHORIZATION, CONTENT_TYPE, COOKIE};
use axum::http::{HeaderValue, Method};
use axum::middleware::Next;
use axum::response::{IntoResponse, Redirect, Response};
use tower_http::cors::{AllowOrigin, CorsLayer};

use crate::error::AppError;
use crate::state::AppState;

/// The browser session cookie set by `POST /login`.
pub const SESSION_COOKIE: &str = "magpie_session";

/// Paths reachable without a token even when auth is on.
fn is_exempt(path: &str) -> bool {
    path == "/healthz" || path == "/login" || path.starts_with("/assets/")
}

/// The value of our session cookie, if the request carries one.
fn session_cookie(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .filter_map(|crumb| crumb.trim().split_once('='))
        .find(|(name, _)| *name == SESSION_COOKIE)
        .map(|(_, value)| value)
}

/// Length-checked constant-time byte comparison.
pub(crate) fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b) {
        diff |= x ^ y;
    }
    diff == 0
}

/// A caller-supplied redirect target, reduced to something that can only point
/// back into this instance.
///
/// Anything that isn't a plain absolute path becomes `/`. A protocol-relative
/// `//evil.example.com` is a URL, not a path; a backslash is normalised to `/`
/// by several browsers, so `/\evil.example.com` is the same trick spelled
/// differently; and a control character could split the `Location` header.
pub(crate) fn safe_next_path(next: Option<&str>) -> String {
    const HOME: &str = "/";

    let Some(candidate) = next.map(str::trim) else {
        return HOME.to_string();
    };

    let is_local = candidate.starts_with('/')
        && !candidate.starts_with("//")
        && !candidate.contains('\\')
        && !candidate.chars().any(char::is_control);

    if is_local {
        candidate.to_string()
    } else {
        HOME.to_string()
    }
}

/// `/login`, carrying where the visitor was actually trying to go.
///
/// Without this, every arrival at a deep link is answered with the collection
/// root once signed in, which is worst exactly when it matters: a link straight
/// to one highlight.
fn login_target(req: &Request) -> String {
    if req.method() != Method::GET {
        return "/login".to_string();
    }

    let wanted = req
        .uri()
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/");
    if wanted == "/" || safe_next_path(Some(wanted)) != wanted {
        return "/login".to_string();
    }

    let query = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("next", wanted)
        .finish();
    format!("/login?{query}")
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

    let headers = req.headers();
    let bearer_ok = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .is_some_and(|token| constant_time_eq(token.as_bytes(), expected.as_bytes()));
    let session_ok = session_cookie(headers)
        .is_some_and(|value| constant_time_eq(value.as_bytes(), state.session.as_bytes()));

    if bearer_ok || session_ok {
        return next.run(req).await;
    }

    // A human who lands on a page gets the login form; the extension gets the
    // JSON 401 its error handling already reads (`shared/http.ts`). Sending an
    // HTML redirect to the API would turn a clear "check your token" into a
    // parse failure.
    if req.uri().path().starts_with("/api/") {
        AppError::Unauthorized.into_response()
    } else {
        Redirect::to(&login_target(&req)).into_response()
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

    let parsed: Vec<HeaderValue> = origins
        .iter()
        .filter_map(|o| o.trim().parse::<HeaderValue>().ok())
        .collect();

    if parsed.is_empty() {
        layer
    } else {
        layer.allow_origin(AllowOrigin::list(parsed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_next_path_can_only_point_back_at_us() {
        assert_eq!(
            safe_next_path(Some("/reader/abc?full=1")),
            "/reader/abc?full=1"
        );
        assert_eq!(safe_next_path(Some("/")), "/");

        for hostile in [
            "//evil.example.com",
            "/\\evil.example.com",
            "https://evil.example.com",
            "javascript:alert(1)",
            "reader/abc",
            "/reader\r\nSet-Cookie: x=y",
        ] {
            assert_eq!(
                safe_next_path(Some(hostile)),
                "/",
                "{hostile} must not survive"
            );
        }

        assert_eq!(safe_next_path(None), "/");
    }

    #[test]
    fn the_login_redirect_carries_a_deep_link_but_not_the_root() {
        let target = |uri: &str| {
            login_target(
                &Request::builder()
                    .uri(uri)
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
        };

        assert_eq!(
            target("/reader/abc?full=1"),
            "/login?next=%2Freader%2Fabc%3Ffull%3D1"
        );
        assert_eq!(
            target("/"),
            "/login",
            "the root is where login lands anyway"
        );
    }
}
