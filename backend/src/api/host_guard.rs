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
//! Note what that argument does *not* cover: an IP address literal. Rebinding
//! needs a name, because the attack is entirely in what that name resolves to.
//! A browser puts a bare address in `Host` only when it was pointed straight at
//! that address, and then the origin *is* that address, so there is nothing to
//! rebind and CORS applies normally. Literals are therefore accepted, which is
//! also what lets a LAN instance be reached at `http://192.168.1.50:8787`
//! without having to name itself in the allowlist first.
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

/// The one name that means "this machine" without resolving anything. The
/// loopback *addresses* need no entry here; they are covered as IP literals.
const LOCALHOST: &str = "localhost";

/// Take the hostname out of a `Host` value, dropping any port. IPv6 literals
/// are bracketed (`[::1]:8787`), so the bracket - not the first colon - is what
/// separates the address from the port.
///
/// Anything else after the bracket is malformed, and is handed back whole
/// rather than trimmed to the part before it: `[::1]evil.example.com` must not
/// reduce to a loopback address that the allowlist would then wave through.
fn hostname_of(host: &str) -> &str {
    let Some(rest) = host.strip_prefix('[') else {
        return match host.split_once(':') {
            Some((name, _port)) => name,
            None => host,
        };
    };

    match rest.find(']') {
        Some(close) if rest[close + 1..].is_empty() || rest[close + 1..].starts_with(':') => {
            &host[..=close + 1]
        }
        _ => host,
    }
}

/// Whether `name` is an IP address rather than a hostname. Bracketed IPv6 is
/// how a `Host` header carries one, so it is unwrapped before parsing.
fn is_ip_literal(name: &str) -> bool {
    match name.strip_prefix('[').and_then(|n| n.strip_suffix(']')) {
        Some(inner) => inner.parse::<std::net::Ipv6Addr>().is_ok(),
        None => name.parse::<std::net::IpAddr>().is_ok(),
    }
}

/// Whether we answer to `name`: `localhost`, any address literal, the public
/// URL's host, or anything else the operator listed.
fn is_known(name: &str, state: &AppState) -> bool {
    if name.eq_ignore_ascii_case(LOCALHOST) || is_ip_literal(name) {
        return true;
    }

    if state
        .config
        .public_url_host()
        .is_some_and(|host| host.eq_ignore_ascii_case(name))
    {
        return true;
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
                 reverse proxy, set MAGPIE_PUBLIC_URL to the address you use, or list the \
                 name in MAGPIE_ALLOWED_HOSTS.\n",
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

    #[test]
    fn a_bracketed_prefix_cannot_shadow_the_real_hostname() {
        // Left trimmed to `[::1]` this would pass as loopback.
        assert_eq!(
            hostname_of("[::1]evil.example.com"),
            "[::1]evil.example.com"
        );
        assert_eq!(hostname_of("[::1"), "[::1");
    }

    #[test]
    fn address_literals_are_recognised_but_names_are_not() {
        for literal in [
            "127.0.0.1",
            "192.168.1.50",
            "10.0.0.8",
            "[::1]",
            "[fe80::1]",
        ] {
            assert!(is_ip_literal(literal), "{literal} is an address");
        }
        for name in ["localhost", "magpie.example.com", "evil.example.com", ""] {
            assert!(
                !is_ip_literal(name),
                "{name} is a name, so it can be rebound"
            );
        }
    }

    #[test]
    fn a_hostname_that_merely_looks_bracketed_is_not_an_address() {
        assert!(!is_ip_literal("[not-an-address]"));
        assert!(!is_ip_literal("[::1]evil.example.com"));
    }
}
