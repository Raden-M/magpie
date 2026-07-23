//! `GET /api/search` - unified FTS across highlights and content, grouped by page.

use axum::Json;
use axum::extract::{Query, State};
use serde::{Deserialize, Serialize};

use crate::api::pages::clamp_limit;
use crate::cursor;
use crate::db::search::{self, HitKind};
use crate::error::AppResult;
use crate::state::AppState;

/// How many raw hits of each kind to consider before grouping.
const HIT_CAP: usize = 400;
/// How many hits to show per page group.
const HITS_PER_PAGE: usize = 6;

#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pub q: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub results: Vec<PageResult>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PageResult {
    pub page: PageRef,
    pub hits: Vec<HitJson>,
}

#[derive(Debug, Serialize)]
pub struct PageRef {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Serialize)]
pub struct HitJson {
    #[serde(rename = "type")]
    pub kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_id: Option<String>,
    pub snippet: String,
}

pub async fn search(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> AppResult<Json<SearchResponse>> {
    let limit = clamp_limit(params.limit);
    let offset = match &params.cursor {
        Some(c) => cursor::decode(c)?
            .1
            .parse::<usize>()
            .map_err(|_| crate::error::AppError::bad_request("invalid cursor"))?,
        None => 0,
    };

    let Some(match_query) = params.q.as_deref().and_then(search::to_match_query) else {
        return Ok(Json(SearchResponse {
            results: Vec::new(),
            next_cursor: None,
        }));
    };

    let groups = state
        .db
        .call(move |conn| Ok(search::search(conn, &match_query, HIT_CAP)?))
        .await?;

    let total = groups.len();
    let results = groups
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|g| PageResult {
            page: PageRef {
                id: g.page_id,
                title: g.page_title,
            },
            hits: g
                .hits
                .into_iter()
                .take(HITS_PER_PAGE)
                .map(|h| match h.kind {
                    HitKind::Highlight => HitJson {
                        kind: "highlight",
                        id: Some(h.id),
                        snapshot_id: None,
                        snippet: h.snippet,
                    },
                    HitKind::Content => HitJson {
                        kind: "content",
                        id: None,
                        snapshot_id: Some(h.id),
                        snippet: h.snippet,
                    },
                })
                .collect(),
        })
        .collect();

    let next_cursor =
        (offset + limit < total).then(|| cursor::encode("s", &(offset + limit).to_string()));

    Ok(Json(SearchResponse {
        results,
        next_cursor,
    }))
}
