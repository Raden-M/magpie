//! `GET /media/{hash}` - serve a content-addressed blob from disk, on our origin
//! only, with a long immutable cache (the hash is the content).

use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};

use crate::db::media;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

fn is_valid_hash(hash: &str) -> bool {
    hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit())
}

pub async fn serve(
    State(state): State<AppState>,
    Path(hash): Path<String>,
) -> AppResult<Response> {
    if !is_valid_hash(&hash) {
        return Err(AppError::not_found("no such media"));
    }

    let path = state.db.media_path(&hash);
    let lookup_hash = hash.clone();
    let mime = state
        .db
        .call(move |conn| Ok(media::get_media(conn, &lookup_hash)?))
        .await?
        .map(|m| m.mime)
        .ok_or_else(|| AppError::not_found("no such media"))?;

    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|_| AppError::not_found("media blob missing on disk"))?;

    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, mime),
            (header::CACHE_CONTROL, "public, max-age=31536000, immutable".to_string()),
        ],
        Body::from(bytes),
    )
        .into_response())
}
