//! `GET /api/stats` - collection counts for the web UI footer.

use axum::Json;
use axum::extract::State;
use serde::Serialize;

use crate::db::stats;
use crate::error::AppResult;
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub pages: i64,
    pub highlights: i64,
    pub media: i64,
    pub media_bytes: i64,
}

pub async fn get(State(state): State<AppState>) -> AppResult<Json<StatsResponse>> {
    let s = state.db.call(move |conn| Ok(stats::stats(conn)?)).await?;
    Ok(Json(StatsResponse {
        pages: s.pages,
        highlights: s.highlights,
        media: s.media,
        media_bytes: s.media_bytes,
    }))
}
