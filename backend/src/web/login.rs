//! `GET|POST /login` - the browser's way past the token.
//!
//! The extension authenticates with a bearer header, which a browser cannot
//! send on navigation. Without this page a token-protected instance answers its
//! own owner with a JSON 401 on every URL, so the collection is only readable
//! from the machine it runs on. Here the token is exchanged, once, for a
//! session cookie carrying [`AppState::session`] - never the token itself, so
//! nothing durable is left in the browser and a restart signs everyone out.

use axum::extract::{Form, State};
use axum::http::StatusCode;
use axum::http::header::{self, HeaderMap};
use axum::response::{Html, IntoResponse, Redirect, Response};
use maud::{DOCTYPE, Markup, html};
use serde::Deserialize;

use crate::api::auth::{SESSION_COOKIE, constant_time_eq};
use crate::state::AppState;
use crate::web::{PAGE_CSP, layout};

#[derive(Deserialize)]
pub struct LoginForm {
    token: String,
}

/// Whether the browser reached us over TLS - directly, or through a proxy that
/// said so. `Secure` must track the real scheme: set on a plain-HTTP LAN
/// instance it would silently discard the cookie and make login impossible.
fn is_secure_request(headers: &HeaderMap) -> bool {
    headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|proto| {
            proto
                .split(',')
                .next()
                .is_some_and(|first| first.trim().eq_ignore_ascii_case("https"))
        })
}

pub async fn form(State(state): State<AppState>) -> Response {
    if !state.config.auth_required() {
        return Redirect::to("/").into_response();
    }
    page(None).into_response()
}

pub async fn submit(
    State(state): State<AppState>,
    headers: HeaderMap,
    Form(input): Form<LoginForm>,
) -> Response {
    if !state.config.auth_required() {
        return Redirect::to("/").into_response();
    }

    let expected = state.config.token.as_deref().unwrap_or_default();
    let presented = input.token.trim();
    if expected.is_empty() || !constant_time_eq(presented.as_bytes(), expected.as_bytes()) {
        return (
            StatusCode::UNAUTHORIZED,
            page(Some(
                "That token doesn't match. Check the extension's options.",
            )),
        )
            .into_response();
    }

    let mut cookie = format!(
        "{SESSION_COOKIE}={}; HttpOnly; SameSite=Strict; Path=/",
        state.session
    );
    if is_secure_request(&headers) {
        cookie.push_str("; Secure");
    }

    ([(header::SET_COOKIE, cookie)], Redirect::to("/")).into_response()
}

fn page(error: Option<&str>) -> Response {
    (
        [(header::CONTENT_SECURITY_POLICY, PAGE_CSP)],
        Html(render(error).into_string()),
    )
        .into_response()
}

fn render(error: Option<&str>) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { "Sign in · Magpie" }
                script src=(super::assets::theme_js_url()) {}
                link rel="stylesheet" href=(super::assets::app_css_url());
            }
            body class="ui-body" {
                header id="top" class="topbar" { (layout::brand()) }
                main class="login" {
                    h1 class="login-title" { "One token stands between you and the hoard." }
                    p class="login-lede" {
                        "This Magpie is protected. Paste the same token the extension uses "
                        "and it'll remember this browser until the server restarts."
                    }
                    form class="login-form" method="post" action="/login" {
                        label class="login-label" for="token" { "Token" }
                        input class="login-input" id="token" name="token" type="password"
                              autocomplete="current-password" autofocus spellcheck="false"
                              required;
                        @if let Some(message) = error {
                            p class="login-error" role="alert" { (message) }
                        }
                        button class="login-submit" type="submit" { "Sign in" }
                    }
                }
                (layout::dock())
                script src=(super::assets::dock_js_url()) {}
            }
        }
    }
}
