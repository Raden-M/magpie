//! `GET /` - the collection: a quiet list grouped by page, newest first, with a
//! live search box above it. Typing filters the hoard in place - the
//! `collection.js` upgrade fetches the `/partials/collection` fragment; without
//! script the same box is a GET form that reloads `/` with the results. A query
//! runs the BM25 full-text engine (`db::search`) across highlight quotes, notes
//! and captured content - the same search `/api/search` exposes - and each row's
//! preview becomes the best-matching snippet.

use axum::extract::{Query, State};
use axum::http::header;
use axum::response::{Html, IntoResponse, Response};
use maud::{Markup, PreEscaped, html};
use serde::Deserialize;

use crate::cursor;
use crate::db::models::Page;
use crate::db::pages::{PageListQuery, PageListRow, PageSort};
use crate::db::stats::Stats;
use crate::db::{pages, search, stats};
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::url_canon;
use crate::web::PAGE_CSP;
use crate::web::layout::shell;

const LIMIT: usize = 30;
/// How many raw hits per FTS table to weigh before grouping - the ceiling on how
/// many matching pages a query can surface.
const HIT_CAP: usize = 400;

const ICON_SEARCH: &str = r##"<svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><circle cx="11" cy="11" r="7"/><path d="M21 21l-4.3-4.3"/></svg>"##;
const ICON_CLEAR: &str = r##"<svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>"##;

#[derive(Debug, Deserialize)]
pub struct CollectionParams {
    pub q: Option<String>,
    pub cursor: Option<String>,
}

/// A single rendered row. Both the recency listing and search produce these, so
/// the list markup has one shape regardless of what filled it.
struct RowView {
    href: String,
    title: String,
    source: String,
    preview: Option<Markup>,
    count: i64,
}

/// One page of rows plus the cursor for the next page, if any.
struct Listing {
    rows: Vec<RowView>,
    next_cursor: Option<String>,
}

/// The full collection page: chrome, the search box, the list, and the footer.
pub async fn page(
    State(state): State<AppState>,
    Query(params): Query<CollectionParams>,
) -> AppResult<Response> {
    let query = params.q.clone().unwrap_or_default();
    let listing = build_listing(&state, &query, params.cursor.clone()).await?;
    let stats = state.db.call(|conn| Ok(stats::stats(conn)?)).await?;

    let body = render_body(&query, &listing, &stats);
    let markup = shell("Collection", body);
    Ok((
        [(header::CONTENT_SECURITY_POLICY, PAGE_CSP)],
        Html(markup.into_string()),
    )
        .into_response())
}

/// `GET /partials/collection` - just the list markup, for live search. Returns
/// exactly what sits inside `#collection-list` on the full page.
pub async fn fragment(
    State(state): State<AppState>,
    Query(params): Query<CollectionParams>,
) -> AppResult<Response> {
    let query = params.q.clone().unwrap_or_default();
    let listing = build_listing(&state, &query, params.cursor.clone()).await?;
    let markup = render_list(&query, &listing);
    Ok((
        [(header::CONTENT_SECURITY_POLICY, PAGE_CSP)],
        Html(markup.into_string()),
    )
        .into_response())
}

/// Recency listing when the query is blank, full-text search otherwise. The two
/// paginate differently - recency by keyset over `last_captured_at`, search by
/// offset into the ranked results - so each decodes its own cursor.
async fn build_listing(
    state: &AppState,
    query: &str,
    cursor: Option<String>,
) -> AppResult<Listing> {
    if query.trim().is_empty() {
        let after = match &cursor {
            Some(c) => Some(cursor::decode(c)?),
            None => None,
        };
        let mut rows = state
            .db
            .call(move |conn| {
                Ok(pages::list_pages(
                    conn,
                    &PageListQuery {
                        q: None,
                        sort: PageSort::Recent,
                        limit: LIMIT + 1,
                        after,
                    },
                )?)
            })
            .await?;

        let next_cursor = if rows.len() > LIMIT {
            rows.truncate(LIMIT);
            rows.last()
                .map(|r| cursor::encode(&r.cursor_value, &r.page.id))
        } else {
            None
        };
        Ok(Listing {
            rows: rows.iter().map(recency_row).collect(),
            next_cursor,
        })
    } else {
        let offset = match &cursor {
            Some(c) => cursor::decode(c)?
                .1
                .parse::<usize>()
                .map_err(|_| AppError::bad_request("invalid cursor"))?,
            None => 0,
        };
        let (rows, total) = match search::to_match_query(query) {
            Some(match_query) => {
                state
                    .db
                    .call(move |conn| {
                        Ok(search::search_page_rows(
                            conn,
                            &match_query,
                            HIT_CAP,
                            LIMIT,
                            offset,
                        )?)
                    })
                    .await?
            }
            None => (Vec::new(), 0),
        };

        let next_cursor =
            (offset + LIMIT < total).then(|| cursor::encode("s", &(offset + LIMIT).to_string()));
        Ok(Listing {
            rows: rows.iter().map(search_row).collect(),
            next_cursor,
        })
    }
}

fn source_label(page: &Page) -> String {
    if let Some(site) = page.site_name.as_deref().filter(|s| !s.trim().is_empty()) {
        return site.to_string();
    }
    url_canon::host_of(&page.canonical_url).unwrap_or_else(|| page.canonical_url.clone())
}

fn recency_row(row: &PageListRow) -> RowView {
    RowView {
        href: format!("/api/pages/{}/reader", row.page.id),
        title: row.page.title.clone(),
        source: source_label(&row.page),
        // A caught quote, set as one.
        preview: row
            .top_highlight
            .as_ref()
            .map(|p| html! { "\u{201c}" (p) "\u{201d}" }),
        count: row.highlight_count,
    }
}

fn search_row(row: &search::SearchPageRow) -> RowView {
    RowView {
        href: format!("/api/pages/{}/reader", row.page.id),
        title: row.page.title.clone(),
        source: source_label(&row.page),
        // The FTS snippet already carries its own ellipses and `<b>` markers, so
        // it stands on its own rather than being wrapped in quotes.
        preview: (!row.snippet.is_empty()).then(|| safe_snippet(&row.snippet)),
        count: row.highlight_count,
    }
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Render an FTS snippet safely: escape everything, then re-allow only the
/// `<b>`/`</b>` match markers the snippet function inserted.
fn safe_snippet(s: &str) -> Markup {
    let escaped = escape_html(s);
    let bolded = escaped
        .replace("&lt;b&gt;", "<b>")
        .replace("&lt;/b&gt;", "</b>");
    PreEscaped(bolded)
}

fn shiny_count(n: i64) -> String {
    if n == 1 {
        "1 shiny thing".to_string()
    } else {
        format!("{n} shiny things")
    }
}

fn search_bar(query: &str) -> Markup {
    html! {
        form class="collection-search" method="get" action="/" role="search" {
            span class="collection-search-icon" { (PreEscaped(ICON_SEARCH)) }
            input class="collection-search-input" type="search" name="q" value=(query)
                  placeholder="Search your hoard" aria-label="Search your hoard"
                  autocomplete="off" spellcheck="false";
            a class="collection-search-clear" href="/" aria-label="Clear search"
              hidden[query.is_empty()] {
                (PreEscaped(ICON_CLEAR))
            }
        }
    }
}

/// The list itself - the part live search swaps. Shared by the full page and the
/// `/partials/collection` fragment so both render rows identically.
fn render_list(query: &str, listing: &Listing) -> Markup {
    html! {
        @if listing.rows.is_empty() {
            p class="ui-empty" {
                @if query.trim().is_empty() {
                    "Nothing kept yet. Highlight something on the web and keep it in Magpie."
                } @else {
                    "Nothing matches \u{201c}" (query) "\u{201d}."
                }
            }
        } @else {
            ul class="collection" {
                @for row in &listing.rows {
                    a class="collection-row" href=(row.href) {
                        div class="row-title" { (row.title) }
                        div class="row-source" { (row.source) }
                        @if let Some(preview) = &row.preview {
                            div class="row-preview" { (preview) }
                        }
                        div class="row-meta" {
                            span class="count" { (shiny_count(row.count)) }
                        }
                    }
                }
            }

            @if let Some(cursor) = &listing.next_cursor {
                @let href = if query.trim().is_empty() {
                    format!("/?cursor={cursor}")
                } else {
                    format!("/?q={}&cursor={}", urlencode(query), cursor)
                };
                a class="load-more" href=(href) { "Load more" }
            }
        }
    }
}

fn render_body(query: &str, listing: &Listing, stats: &Stats) -> Markup {
    html! {
        main class="ui-main" {
            div class="page-head" {
                span class="section-label" { "The hoard" }
                h1 { "Collection" }
            }

            (search_bar(query))

            div id="collection-list" {
                (render_list(query, listing))
            }
        }
        footer class="footer" {
            span class="footer-stats" {
                (stats.pages) " page" (plural(stats.pages)) " · "
                (stats.highlights) " shiny thing" (plural(stats.highlights)) " · "
                (stats.media) " image" (plural(stats.media))
            }
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
