//! The reader view: `GET /api/pages/{id}/reader` (chrome + baked highlights +
//! rail) and `GET /api/pages/{id}/snapshot` (raw sanitized content).

use axum::extract::{Path, Query, State};
use axum::http::header;
use axum::response::{Html, IntoResponse, Response};
use maud::{DOCTYPE, Markup, PreEscaped, html};
use serde::Deserialize;

use crate::db::models::{Highlight, Page, Snapshot};
use crate::db::{highlights, pages, snapshots};
use crate::error::{AppError, AppResult};
use crate::reader::anchor::Selectors;
use crate::reader::bake::{self, BakeHighlight};
use crate::state::AppState;
use crate::web::{PAGE_CSP, SNAPSHOT_CSP};

#[derive(Debug, Deserialize)]
pub struct ReaderParams {
    /// View an older snapshot version; defaults to the latest.
    pub v: Option<i64>,
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
    let baked = bake::bake(&loaded.content_html, &bake_inputs);

    let markup = render_reader(&loaded, &baked.html, &baked.located);
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

fn render_reader(
    loaded: &Loaded,
    baked_html: &str,
    located: &std::collections::HashSet<String>,
) -> Markup {
    let Loaded {
        page,
        snapshot,
        highlights,
        ..
    } = loaded;
    let reading = snapshot.reading_time.unwrap_or(0);

    html! {
        (DOCTYPE)
        html lang=(page.lang.as_deref().unwrap_or("en")) {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (page.title) " · Magpie" }
                link rel="stylesheet" href="/assets/app.css";
            }
            body class="reader-body" {
                header class="topbar" {
                    a class="brand" href="/" { "Mag" span class="brand-pie" { "pie" } }
                    div class="topbar-meta" {
                        a class="source-link" href=(page.original_url) rel="noopener noreferrer" target="_blank" {
                            (page.site_name.as_deref().unwrap_or("source"))
                        }
                        @if reading > 0 { span class="reading-time" { (reading) " min" } }
                        @if snapshot.version > 1 { span class="snap-version" { "v" (snapshot.version) } }
                    }
                }
                div class="reader-grid" {
                    article class="article" {
                        h1 class="article-title" { (page.title) }
                        @if let Some(byline) = &snapshot.byline {
                            p class="article-byline" { (byline) }
                        }
                        div class="article-body" { (PreEscaped(baked_html)) }
                    }
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
                script src="/assets/reader.js" {}
            }
        }
    }
}
