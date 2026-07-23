//! `GET /` - the collection: a quiet list grouped by page, newest first.

use axum::extract::{Query, State};
use axum::http::header;
use axum::response::{Html, IntoResponse, Response};
use maud::{Markup, html};
use serde::Deserialize;

use crate::cursor;
use crate::db::pages::{PageListQuery, PageListRow, PageSort};
use crate::db::stats::Stats;
use crate::db::{pages, stats};
use crate::error::AppResult;
use crate::state::AppState;
use crate::url_canon;
use crate::web::PAGE_CSP;
use crate::web::layout::shell;

const LIMIT: usize = 30;

#[derive(Debug, Deserialize)]
pub struct CollectionParams {
    pub q: Option<String>,
    pub cursor: Option<String>,
}

pub async fn page(
    State(state): State<AppState>,
    Query(params): Query<CollectionParams>,
) -> AppResult<Response> {
    let after = match &params.cursor {
        Some(c) => Some(cursor::decode(c)?),
        None => None,
    };
    let q = params.q.clone();

    let (mut rows, stats) = state
        .db
        .call(move |conn| {
            let rows = pages::list_pages(
                conn,
                &PageListQuery {
                    q,
                    sort: PageSort::Recent,
                    limit: LIMIT + 1,
                    after,
                },
            )?;
            let stats = stats::stats(conn)?;
            Ok((rows, stats))
        })
        .await?;

    let next_cursor = if rows.len() > LIMIT {
        rows.truncate(LIMIT);
        rows.last()
            .map(|r| cursor::encode(&r.cursor_value, &r.page.id))
    } else {
        None
    };

    let query = params.q.unwrap_or_default();
    let body = render_body(&query, &rows, &next_cursor, &stats);
    let markup = shell("Collection", &query, body);
    Ok((
        [(header::CONTENT_SECURITY_POLICY, PAGE_CSP)],
        Html(markup.into_string()),
    )
        .into_response())
}

fn source_label(row: &PageListRow) -> String {
    if let Some(site) = row
        .page
        .site_name
        .as_deref()
        .filter(|s| !s.trim().is_empty())
    {
        return site.to_string();
    }
    url_canon::host_of(&row.page.canonical_url).unwrap_or_else(|| row.page.canonical_url.clone())
}

fn shiny_count(n: i64) -> String {
    if n == 1 {
        "1 shiny thing".to_string()
    } else {
        format!("{n} shiny things")
    }
}

fn render_body(
    query: &str,
    rows: &[PageListRow],
    next_cursor: &Option<String>,
    stats: &Stats,
) -> Markup {
    html! {
        main class="ui-main" {
            div class="page-head" {
                span class="section-label" { "The hoard" }
                h1 { "Collection" }
            }

            @if rows.is_empty() {
                p class="ui-empty" {
                    @if query.trim().is_empty() {
                        "Nothing kept yet. Highlight something on the web and keep it in Magpie."
                    } @else {
                        "No pages match \u{201c}" (query) "\u{201d}."
                    }
                }
            } @else {
                ul class="collection" {
                    @for row in rows {
                        a class="collection-row" href=(format!("/api/pages/{}/reader", row.page.id)) {
                            div class="row-title" { (row.page.title) }
                            div class="row-source" { (source_label(row)) }
                            @if let Some(preview) = &row.top_highlight {
                                div class="row-preview" { "\u{201c}" (preview) "\u{201d}" }
                            }
                            div class="row-meta" {
                                span class="count" { (shiny_count(row.highlight_count)) }
                            }
                        }
                    }
                }

                @if let Some(cursor) = next_cursor {
                    @let href = if query.trim().is_empty() {
                        format!("/?cursor={cursor}")
                    } else {
                        format!("/?q={}&cursor={}", urlencode(query), cursor)
                    };
                    a class="load-more" href=(href) { "Load more" }
                }
            }
        }
        footer class="footer" {
            (stats.pages) " page" (plural(stats.pages)) " · "
            (stats.highlights) " shiny thing" (plural(stats.highlights)) " · "
            (stats.media) " image" (plural(stats.media))
        }
    }
}

fn plural(n: i64) -> &'static str {
    if n == 1 { "" } else { "s" }
}

/// Minimal percent-encoding for the query string in the load-more link.
fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}
