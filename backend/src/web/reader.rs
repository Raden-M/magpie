//! The reader view: `GET /api/pages/{id}/reader` (chrome + baked highlights +
//! rail) and `GET /api/pages/{id}/snapshot` (raw sanitized content).

use axum::extract::{Path, Query, State};
use axum::http::header;
use axum::response::{Html, IntoResponse, Response};
use maud::{DOCTYPE, Markup, PreEscaped, html};
use serde::Deserialize;

use std::collections::HashMap;

use crate::db::models::{Highlight, Page, Snapshot};
use crate::db::{highlights, pages, snapshots};
use crate::error::{AppError, AppResult};
use crate::reader::anchor::Selectors;
use crate::reader::bake::{self, BakeHighlight};
use crate::reader::snippet::{self, Snippet};
use crate::state::AppState;
use crate::url_canon;
use crate::util;
use crate::web::{PAGE_CSP, SNAPSHOT_CSP};

#[derive(Debug, Deserialize)]
pub struct ReaderParams {
    /// View an older snapshot version; defaults to the latest.
    pub v: Option<i64>,
    /// `full` shows the whole captured snapshot (the backup); anything else -
    /// the default - shows the highlight snippets.
    pub view: Option<String>,
}

struct Loaded {
    page: Page,
    snapshot: Snapshot,
    highlights: Vec<Highlight>,
    content_html: String,
}

async fn load(state: &AppState, id: String, version: Option<i64>) -> AppResult<Loaded> {
    let data_dir = state.db.data_dir().to_path_buf();
    state
        .db
        .call(move |conn| {
            let page =
                pages::get_page(conn, &id)?.ok_or_else(|| AppError::not_found("page not found"))?;
            let snapshot = match version {
                Some(v) => snapshots::snapshot_by_version(conn, &page.id, v)?,
                None => snapshots::latest_snapshot(conn, &page.id)?,
            }
            .ok_or_else(|| AppError::not_found("this page has no snapshot"))?;
            let highlights = highlights::highlights_for_page(conn, &page.id)?;
            let content_html = std::fs::read_to_string(data_dir.join(&snapshot.content_path))
                .map_err(|e| AppError::Internal(e.into()))?;
            Ok(Loaded {
                page,
                snapshot,
                highlights,
                content_html,
            })
        })
        .await
}

pub async fn reader(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<ReaderParams>,
) -> AppResult<Response> {
    let loaded = load(&state, id, params.v).await?;
    let full = params.view.as_deref() == Some("full");

    let bake_inputs: Vec<BakeHighlight<'_>> = loaded
        .highlights
        .iter()
        .map(|h| BakeHighlight {
            id: &h.id,
            color: &h.color,
            selectors: Selectors {
                quote_exact: &h.quote_exact,
                quote_prefix: &h.quote_prefix,
                quote_suffix: &h.quote_suffix,
                pos_start: h.pos_start,
                pos_end: h.pos_end,
            },
        })
        .collect();

    let (article, located) = if full {
        let baked = bake::bake(&loaded.content_html, &bake_inputs);
        (full_article(&loaded, &baked.html), baked.located)
    } else {
        let snips = snippet::snippets(&loaded.content_html, &bake_inputs);
        let located = snips
            .iter()
            .flat_map(|s| s.hl_ids.iter().cloned())
            .collect();
        (snippet_article(&loaded, &snips), located)
    };

    let markup = page_shell(&loaded, &located, full, params.v, article);
    Ok((
        [(header::CONTENT_SECURITY_POLICY, PAGE_CSP)],
        Html(markup.into_string()),
    )
        .into_response())
}

pub async fn snapshot_raw(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<ReaderParams>,
) -> AppResult<Response> {
    let loaded = load(&state, id, params.v).await?;
    Ok((
        [(header::CONTENT_SECURITY_POLICY, SNAPSHOT_CSP)],
        Html(loaded.content_html),
    )
        .into_response())
}

fn snippet(quote: &str) -> String {
    const LIMIT: usize = 140;
    let trimmed = quote.trim();
    if trimmed.chars().count() <= LIMIT {
        return trimmed.to_string();
    }
    let mut out: String = trimmed.chars().take(LIMIT).collect();
    out.push('…');
    out
}

/// The origin URL without its scheme - the host and path are the identifying
/// part, and dropping `https://` buys back room in a 17rem rail.
fn without_scheme(url: &str) -> &str {
    url.strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url)
}

fn nonempty(value: Option<&String>) -> Option<&str> {
    value.map(|s| s.trim()).filter(|s| !s.is_empty())
}

/// Where this came from and what was kept: above the article on a phone, at the
/// head of the rail on a wide screen. The article stays pure content - everything
/// *about* the capture lives here, so a snapshot never has to imitate the page it
/// was taken from.
fn source_card(page: &Page, snapshot: &Snapshot, full: bool, v: Option<i64>) -> Markup {
    let bare = without_scheme(&page.original_url).to_string();
    // Prefer the publisher's own name for the heading, as the collection does.
    // Falling back to the host would make the line below start with that same
    // host, so that case drops to the path alone rather than saying it twice.
    let (site, url_line) = match (
        nonempty(page.site_name.as_ref()),
        url_canon::split_authority(&page.original_url),
    ) {
        (Some(name), _) => (name.to_string(), bare),
        (None, Some((authority, rest))) => (authority, rest),
        (None, None) => ("Source".to_string(), bare),
    };
    let reading = snapshot.reading_time.unwrap_or(0);

    html! {
        section class="source" {
            h2 class="rail-title" { "Source" }
            a class="source-link" href=(page.original_url) title=(page.original_url)
              rel="noopener noreferrer" target="_blank" {
                span class="source-site" { (site) }
                span class="source-url" { (url_line) }
            }
            @if let Some(byline) = nonempty(snapshot.byline.as_ref()) {
                p class="source-byline" { (byline) }
            }
            // Each pair is wrapped so it can never break across a line when the
            // facts run inline on a narrow screen; in the rail the wrappers go
            // `display: contents` and the pairs align as two columns.
            dl class="source-facts" {
                div class="fact" {
                    dt { "Kept" }
                    dd { (util::short_date(&snapshot.captured_at)) }
                }
                @if let Some(words) = snapshot.word_count.filter(|w| *w > 0) {
                    div class="fact" {
                        dt { "Words" }
                        dd { (util::thousands(words)) }
                    }
                }
                @if reading > 0 {
                    div class="fact" {
                        dt { "Reading" }
                        dd { (reading) " min" }
                    }
                }
                @if snapshot.version > 1 {
                    div class="fact" {
                        dt { "Snapshot" }
                        dd { "v" (snapshot.version) }
                    }
                }
            }
            @if snapshot.partial {
                p class="source-partial" {
                    "Partial capture - this page wasn't article-shaped, so only the region "
                    "around the highlight was kept."
                }
            }
            // Snippets are the default; the full snapshot is the backup, one
            // click away. Both are server-rendered, so the toggle needs no JS.
            nav class="view-toggle" aria-label="Reader view" {
                a class=({ if full { "view-tab" } else { "view-tab is-current" } })
                  href=(reader_query(page, v, false))
                  aria-current=[(!full).then_some("page")] { "Snippets" }
                a class=({ if full { "view-tab is-current" } else { "view-tab" } })
                  href=(reader_query(page, v, true))
                  aria-current=[full.then_some("page")] { "Full snapshot" }
            }
        }
    }
}

/// A link back to this same reader, choosing a view and preserving the version.
fn reader_query(page: &Page, v: Option<i64>, full: bool) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(v) = v {
        parts.push(format!("v={v}"));
    }
    parts.push(format!("view={}", if full { "full" } else { "snippets" }));
    format!("/api/pages/{}/reader?{}", page.id, parts.join("&"))
}

/// A deep link into the *live* page at the highlighted text, using a URL Text
/// Fragment (`#:~:text=`). Browsers that support it scroll to and flash the
/// exact words; the rest simply open the page. When the quote is long we fall
/// back to the fragment's `start,end` form, which matches on the first and last
/// few words - a whole-quote directive that long tends not to match at all.
fn live_url(original_url: &str, quote: &str) -> String {
    const MAX_EXACT: usize = 300;
    let quote = collapse_ws(quote);
    let core = if quote.chars().count() <= MAX_EXACT {
        pct(&quote)
    } else {
        let words: Vec<&str> = quote.split(' ').collect();
        let start = words.iter().take(6).copied().collect::<Vec<_>>().join(" ");
        let end = words
            .iter()
            .rev()
            .take(6)
            .rev()
            .copied()
            .collect::<Vec<_>>()
            .join(" ");
        format!("{},{}", pct(&start), pct(&end))
    };
    let directive = format!("text={core}");
    if original_url.contains('#') {
        format!("{original_url}:~:{directive}")
    } else {
        format!("{original_url}#:~:{directive}")
    }
}

fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Percent-encode conservatively: everything but ASCII alphanumerics. Over-
/// encoding is harmless (browsers decode `%XX`) and sidesteps the text-fragment
/// rule that `-`, `,`, `&` and `%` must themselves be escaped inside a directive.
fn pct(s: &str) -> String {
    use std::fmt::Write;
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() {
            out.push(b as char);
        } else {
            let _ = write!(out, "%{b:02X}");
        }
    }
    out
}

/// The full captured snapshot with highlights baked in - the backup view.
/// The article's own summary, set as an italic standfirst under the title. Uses
/// the captured excerpt verbatim - never fabricated - and simply doesn't render
/// when the snapshot carries none.
fn standfirst(snapshot: &Snapshot) -> Markup {
    html! {
        @if let Some(excerpt) = nonempty(snapshot.excerpt.as_ref()) {
            p class="article-standfirst" { (excerpt) }
        }
    }
}

fn full_article(loaded: &Loaded, baked_html: &str) -> Markup {
    html! {
        article class="article" {
            h1 class="article-title" { (loaded.page.title) }
            (standfirst(&loaded.snapshot))
            div class="article-body" { (PreEscaped(baked_html)) }
        }
    }
}

/// The default view: each highlight shown inside its surrounding passage.
fn snippet_article(loaded: &Loaded, snips: &[Snippet]) -> Markup {
    let by_id: HashMap<&str, &Highlight> = loaded
        .highlights
        .iter()
        .map(|h| (h.id.as_str(), h))
        .collect();
    let located: std::collections::HashSet<&str> = snips
        .iter()
        .flat_map(|s| s.hl_ids.iter().map(String::as_str))
        .collect();
    let orphans: Vec<&Highlight> = loaded
        .highlights
        .iter()
        .filter(|h| !located.contains(h.id.as_str()))
        .collect();

    html! {
        article class="article article--snippets" {
            h1 class="article-title" { (loaded.page.title) }
            (standfirst(&loaded.snapshot))
            @if snips.is_empty() && orphans.is_empty() {
                p class="snippets-empty" {
                    "No highlights kept on this page yet. The "
                    "full snapshot is still here as a backup."
                }
            }
            @for snip in snips {
                section class="snippet" {
                    div class="snippet-passage" { (PreEscaped(&snip.html)) }
                    (snippet_meta(&loaded.page, snip.hl_ids.iter().filter_map(|id| by_id.get(id.as_str()).copied())))
                }
            }
            @if !orphans.is_empty() {
                @for h in &orphans {
                    section class="snippet snippet--orphan" {
                        p class="snippet-orphan-tag" { "Couldn't re-locate this in the snapshot" }
                        blockquote class="snippet-orphan-quote" { (h.quote_exact) }
                        (snippet_meta(&loaded.page, std::iter::once(*h)))
                    }
                }
            }
        }
    }
}

/// The note(s) and live-page link(s) that sit under a passage. A merged snippet
/// can hold more than one highlight, so this takes an iterator.
fn snippet_meta<'a>(page: &Page, hls: impl Iterator<Item = &'a Highlight>) -> Markup {
    html! {
        footer class="snippet-meta" {
            @for h in hls {
                div class="snippet-action" {
                    @if let Some(note) = nonempty(h.note.as_ref()) {
                        p class="snippet-note" { (note) }
                    }
                    a class="snippet-jump" href=(live_url(&page.original_url, &h.quote_exact))
                      rel="noopener noreferrer" target="_blank" {
                        "Open on page"
                        span class="snippet-jump-arrow" aria-hidden="true" { " ↗" }
                    }
                }
            }
        }
    }
}

fn page_shell(
    loaded: &Loaded,
    located: &std::collections::HashSet<String>,
    full: bool,
    v: Option<i64>,
    article: Markup,
) -> Markup {
    let Loaded {
        page,
        snapshot,
        highlights,
        ..
    } = loaded;

    html! {
        (DOCTYPE)
        html lang=(page.lang.as_deref().unwrap_or("en")) {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (page.title) " · Magpie" }
                // Ahead of the stylesheet so a stored theme lands before paint.
                script src=(super::assets::theme_js_url()) {}
                link rel="stylesheet" href=(super::assets::app_css_url());
            }
            body class="reader-body" {
                header id="top" class="topbar" {
                    (super::layout::brand())
                    (super::layout::back_to_collection())
                }
                // Source first in the DOM: on a phone the grid is one column, so
                // provenance reads above the article and the rail falls below it.
                // The wide layout places all three by grid area instead.
                div class="reader-grid" {
                    (source_card(page, snapshot, full, v))
                    (article)
                    aside class="rail" {
                        h2 class="rail-title" { "Shiny things" }
                        @if highlights.is_empty() {
                            p class="rail-empty" { "No highlights kept on this page yet." }
                        }
                        ul class="rail-list" {
                            @for h in highlights {
                                @let is_located = located.contains(&h.id);
                                li class=({ if is_located { "rail-item" } else { "rail-item is-orphaned" } })
                                   data-hl=(h.id) {
                                    @if is_located {
                                        a class="rail-quote" href=(format!("#hl-{}", h.id)) {
                                            (snippet(&h.quote_exact))
                                        }
                                    } @else {
                                        span class="rail-quote" { (snippet(&h.quote_exact)) }
                                        span class="rail-orphan" { "couldn't re-locate" }
                                    }
                                    @if let Some(note) = &h.note {
                                        @if !note.trim().is_empty() {
                                            p class="rail-note" { (note) }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                (super::layout::dock())
                script src=(super::assets::reader_js_url()) {}
                script src=(super::assets::dock_js_url()) {}
            }
        }
    }
}
