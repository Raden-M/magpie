//! `GET /search` - the server-rendered search page: a query box and results
//! grouped by page, with BM25-ranked snippets.

use axum::extract::{Query, State};
use axum::http::header;
use axum::response::{Html, IntoResponse, Response};
use maud::{Markup, PreEscaped, html};
use serde::Deserialize;

use crate::db::search::{self, HitKind, PageGroup};
use crate::error::AppResult;
use crate::state::AppState;
use crate::web::PAGE_CSP;
use crate::web::layout::shell;

const MAX_GROUPS: usize = 50;
const HITS_PER_PAGE: usize = 6;

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
}

pub async fn page(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> AppResult<Response> {
    let query = params.q.unwrap_or_default();
    let groups = match search::to_match_query(&query) {
        Some(match_query) => {
            state
                .db
                .call(move |conn| Ok(search::search(conn, &match_query, 400)?))
                .await?
        }
        None => Vec::new(),
    };

    let body = render_results(&query, &groups);
    let markup = shell("Search", &query, body);
    Ok((
        [(header::CONTENT_SECURITY_POLICY, PAGE_CSP)],
        Html(markup.into_string()),
    )
        .into_response())
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Render an FTS snippet safely: escape everything, then re-allow only the
/// `<b>`/`</b>` match markers the snippet function inserted.
fn safe_snippet(s: &str) -> PreEscaped<String> {
    let escaped = escape_html(s);
    let bolded = escaped
        .replace("&lt;b&gt;", "<b>")
        .replace("&lt;/b&gt;", "</b>");
    PreEscaped(bolded)
}

fn render_results(query: &str, groups: &[PageGroup]) -> Markup {
    html! {
        main class="ui-main" {
            @if query.trim().is_empty() {
                p class="ui-empty" { "Type to search across your highlights, notes, and every word you kept." }
            } @else if groups.is_empty() {
                p class="ui-empty" { "Nothing matched \u{201c}" (query) "\u{201d}." }
            } @else {
                p class="search-summary" {
                    (groups.len().min(MAX_GROUPS)) " page" (if groups.len() == 1 { "" } else { "s" })
                }
                ul class="search-results" {
                    @for group in groups.iter().take(MAX_GROUPS) {
                        li class="search-group" {
                            a class="search-page-title" href=(format!("/api/pages/{}/reader", group.page_id)) {
                                (group.page_title)
                            }
                            ul class="search-hits" {
                                @for hit in group.hits.iter().take(HITS_PER_PAGE) {
                                    @let href = match hit.kind {
                                        HitKind::Highlight => format!("/api/pages/{}/reader#hl-{}", group.page_id, hit.id),
                                        HitKind::Content => format!("/api/pages/{}/reader", group.page_id),
                                    };
                                    li class="search-hit" {
                                        span class=({ match hit.kind { HitKind::Highlight => "hit-kind hit-highlight", HitKind::Content => "hit-kind hit-content" } }) {
                                            (match hit.kind { HitKind::Highlight => "highlight", HitKind::Content => "in text" })
                                        }
                                        a class="hit-snippet" href=(href) { (safe_snippet(&hit.snippet)) }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
