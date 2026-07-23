//! Cross-page highlight listing and per-highlight edit/delete.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::api::dto::HighlightJson;
use crate::api::pages::clamp_limit;
use crate::cursor;
use crate::db::highlights;
use crate::db::highlights::HighlightListQuery;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ListParams {
    pub q: Option<String>,
    pub page_id: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct HighlightListResponse {
    pub highlights: Vec<HighlightJson>,
    pub next_cursor: Option<String>,
}

pub async fn list(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> AppResult<Json<HighlightListResponse>> {
    let limit = clamp_limit(params.limit);
    let after = match &params.cursor {
        Some(c) => Some(cursor::decode(c)?),
        None => None,
    };
    let (q, page_id) = (params.q.clone(), params.page_id.clone());

    let mut rows = state
        .db
        .call(move |conn| {
            Ok(highlights::list_highlights(
                conn,
                &HighlightListQuery { page_id, q, limit: limit + 1, after },
            )?)
        })
        .await?;

    let next_cursor = if rows.len() > limit {
        rows.truncate(limit);
        rows.last().map(|r| cursor::encode(&r.cursor_value, &r.highlight.id))
    } else {
        None
    };

    Ok(Json(HighlightListResponse {
        highlights: rows.into_iter().map(|r| r.highlight.into()).collect(),
        next_cursor,
    }))
}

#[derive(Debug, Deserialize)]
pub struct PatchBody {
    /// Absent = leave note unchanged; `null` = clear it; a string = set it.
    #[serde(default)]
    pub note: Option<Option<String>>,
    #[serde(default)]
    pub color: Option<String>,
}

pub async fn patch(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<PatchBody>,
) -> AppResult<Json<HighlightJson>> {
    let updated = state
        .db
        .call(move |conn| {
            let note = body.note.as_ref().map(|inner| inner.as_deref());
            let color = body.color.as_deref();
            Ok(highlights::update_highlight(conn, &id, note, color)?)
        })
        .await?
        .ok_or_else(|| AppError::not_found("highlight not found"))?;

    Ok(Json(updated.into()))
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<StatusCode> {
    let removed = state.db.call(move |conn| Ok(highlights::delete_highlight(conn, &id)?)).await?;
    if removed {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::not_found("highlight not found"))
    }
}
