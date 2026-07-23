//! Collection + item endpoints for pages.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::api::dto::{HighlightJson, PageJson, SnapshotJson};
use crate::cursor;
use crate::db::pages::{PageListQuery, PageSort};
use crate::db::{highlights, media, pages, snapshots};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ListParams {
    pub q: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<usize>,
    pub sort: Option<String>,
}

pub fn clamp_limit(limit: Option<usize>) -> usize {
    limit.unwrap_or(30).clamp(1, 100)
}

#[derive(Debug, Serialize)]
pub struct PageSummary {
    pub id: String,
    pub title: String,
    pub canonical_url: String,
    pub site_name: Option<String>,
    pub favicon_path: Option<String>,
    pub highlight_count: i64,
    pub last_captured_at: String,
    pub top_highlight: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PageListResponse {
    pub pages: Vec<PageSummary>,
    pub next_cursor: Option<String>,
}

pub async fn list(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> AppResult<Json<PageListResponse>> {
    let limit = clamp_limit(params.limit);
    let sort = match params.sort.as_deref() {
        Some("title") => PageSort::Title,
        _ => PageSort::Recent,
    };
    let after = match &params.cursor {
        Some(c) => Some(cursor::decode(c)?),
        None => None,
    };
    let q = params.q.clone();

    let mut rows = state
        .db
        .call(move |conn| {
            Ok(pages::list_pages(
                conn,
                &PageListQuery { q, sort, limit: limit + 1, after },
            )?)
        })
        .await?;

    let next_cursor = if rows.len() > limit {
        rows.truncate(limit);
        rows.last().map(|r| cursor::encode(&r.cursor_value, &r.page.id))
    } else {
        None
    };

    let pages = rows
        .into_iter()
        .map(|r| PageSummary {
            id: r.page.id,
            title: r.page.title,
            canonical_url: r.page.canonical_url,
            site_name: r.page.site_name,
            favicon_path: r.page.favicon_path,
            highlight_count: r.highlight_count,
            last_captured_at: r.page.last_captured_at,
            top_highlight: r.top_highlight,
        })
        .collect();

    Ok(Json(PageListResponse { pages, next_cursor }))
}

#[derive(Debug, Serialize)]
pub struct PageDetail {
    pub page: PageJson,
    pub snapshot: Option<SnapshotJson>,
    pub highlights: Vec<HighlightJson>,
}

pub async fn get(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<PageDetail>> {
    let detail = state
        .db
        .call(move |conn| {
            let page = pages::get_page(conn, &id)?
                .ok_or_else(|| AppError::not_found("page not found"))?;
            let snapshot = snapshots::latest_snapshot(conn, &page.id)?;
            let highlights = highlights::highlights_for_page(conn, &page.id)?;
            Ok((page, snapshot, highlights))
        })
        .await?;

    let (page, snapshot, highlights) = detail;
    Ok(Json(PageDetail {
        page: page.into(),
        snapshot: snapshot.map(Into::into),
        highlights: highlights.into_iter().map(Into::into).collect(),
    }))
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<StatusCode> {
    let data_dir = state.db.data_dir().to_path_buf();
    let existed = state
        .db
        .call(move |conn| {
            if pages::get_page(conn, &id)?.is_none() {
                return Ok(false);
            }

            let snapshot_paths: Vec<String> = {
                let mut stmt = conn.prepare("SELECT content_path FROM snapshot WHERE page_id = ?1")?;
                let rows = stmt.query_map([&id], |r| r.get::<_, String>(0))?;
                rows.collect::<rusqlite::Result<_>>()?
            };

            let tx = conn.transaction()?;
            pages::delete_page(&tx, &id)?;
            media::collect_orphans(&tx, &data_dir.join("media"))?;
            tx.commit()?;

            for path in snapshot_paths {
                let _ = std::fs::remove_file(data_dir.join(path));
            }
            Ok(true)
        })
        .await?;

    if existed {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::not_found("page not found"))
    }
}
