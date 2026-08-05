//! `Host` header allowlist.
//!
//! Binding to loopback is not the same as being unreachable: every page you
//! visit can issue requests to `127.0.0.1`. CORS covers the ordinary case by
//! withholding the response, but DNS rebinding sidesteps it entirely - the
//! attacker points their own hostname at `127.0.0.1`, and the browser then
//! considers the response same-origin, so no CORS check ever runs.
//!
//! Pinning the hostnames we answer to is what closes that, because a rebound
//! request still arrives carrying the attacker's hostname rather than ours.
//!
//! This does *not* stop a page from firing a cross-origin `POST /api/captures`
//! at a genuinely local address: `multipart/form-data` is a CORS-safelisted
//! content type, so the write is a simple request that needs no preflight and
//! carries a legitimate `Host`. Only the response is hidden. See the security
//! posture in `docs/usage.md`.

use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::http::header::HOST;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::state::AppState;

/// Names that always mean "this machine".
const LOOPBACK_HOSTS: [&str; 3] = ["localhost", "127.0.0.1", "[::1]"];

/// Take the hostname out of a `Host` value, dropping any port. IPv6 literals
/// are bracketed (`[::1]:8787`), so the bracket - not the first colon - is what
/// separates the address from the port.
fn hostname_of(host: &str) -> &str {
    if let Some(close) = host.rfind(']') {
        return &host[..=close];
    }
    match host.split_once(':') {
        Some((name, _port)) => name,
        None => host,
    }
}

/// Whether we answer to `name`: loopback, whatever we are bound to, or anything
/// the operator listed for a reverse proxy.
fn is_known(name: &str, state: &AppState) -> bool {
    if LOOPBACK_HOSTS.iter().any(|h| h.eq_ignore_ascii_case(name)) {
        return true;
    }

    let bound = state.config.bind.ip();
    // A wildcard bind answers to whatever name reaches it, so it can't
    // contribute one of its own - the operator lists those explicitly.
    if !bound.is_unspecified() {
        let literal = match bound {
            std::net::IpAddr::V4(v4) => v4.to_string(),
            std::net::IpAddr::V6(v6) => format!("[{v6}]"),
        };
        if literal.eq_ignore_ascii_case(name) {
            return true;
        }
    }

    state
        .config
        .allowed_hosts
        .iter()
        .any(|h| hostname_of(h.trim()).eq_ignore_ascii_case(name))
}

/// Reject requests carrying a `Host` we don't recognise.
///
/// A request with no host information at all is allowed through: browsers
/// always send `Host`, so its absence means the caller isn't one, and a raw
/// client could present any value it liked anyway. The header is what a
/// *browser* can be tricked into sending, and that is what this guards.
pub async fn require_known_host(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    let presented: Option<String> = req
        .headers()
        .get(HOST)
        .and_then(|v| v.to_str().ok())
        .or_else(|| req.uri().host())
        .map(|h| hostname_of(h).to_owned());

    match presented {
        Some(name) if !is_known(&name, &state) => {
            tracing::warn!(host = %name, "refused a request for an unrecognised Host");
            (
                StatusCode::MISDIRECTED_REQUEST,
                "This Magpie does not answer to that hostname. If you reach it through a \
                 reverse proxy, add the public name to MAGPIE_ALLOWED_HOSTS.\n",
            )
                .into_response()
        }
        _ => next.run(req).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_the_port_off_a_host_value() {
        assert_eq!(hostname_of("127.0.0.1:8787"), "127.0.0.1");
        assert_eq!(hostname_of("magpie.example.com"), "magpie.example.com");
        assert_eq!(hostname_of("[::1]:8787"), "[::1]");
        assert_eq!(hostname_of("[::1]"), "[::1]");
        assert_eq!(hostname_of("[2001:db8::1]:443"), "[2001:db8::1]");
    }
}
