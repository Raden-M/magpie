//! `POST /api/captures` - the main write path. Parses a multipart capture,
//! re-validates the canonical URL, then commits page + snapshot + highlight +
//! media + FTS in a single transaction, keyed for idempotent retries.

use std::collections::HashMap;
use std::path::Path;

use axum::extract::{Multipart, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::db::models::{Media, NewHighlight, NewPage, NewSnapshot};
use crate::db::{highlights, media, pages, snapshots};
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::url_canon;
use crate::util::{now_iso, sha256_hex};

const MAX_IMAGES: usize = 200;
const MAX_QUOTE_CHARS: usize = 100_000;
const MAX_NOTE_CHARS: usize = 50_000;
const MAX_CONTENT_BYTES: usize = 8_000_000;

#[derive(Debug, Deserialize)]
pub struct CapturePayload {
    pub capture_id: String,
    pub url: String,
    pub canonical_url: String,
    pub page: PagePart,
    pub snapshot: SnapshotPart,
    #[serde(default)]
    pub highlight: Option<HighlightPart>,
    #[serde(default)]
    pub media: Vec<MediaPart>,
}

#[derive(Debug, Deserialize)]
pub struct PagePart {
    pub title: String,
    #[serde(default)]
    pub site_name: Option<String>,
    #[serde(default)]
    pub lang: Option<String>,
    #[serde(default)]
    pub favicon_hash: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SnapshotPart {
    pub content_html: String,
    pub content_text: String,
    #[serde(default)]
    pub byline: Option<String>,
    #[serde(default)]
    pub excerpt: Option<String>,
    #[serde(default)]
    pub word_count: Option<i64>,
    pub content_hash: String,
    #[serde(default)]
    pub partial: bool,
}

#[derive(Debug, Deserialize)]
pub struct HighlightPart {
    pub quote_exact: String,
    #[serde(default)]
    pub quote_prefix: String,
    #[serde(default)]
    pub quote_suffix: String,
    #[serde(default)]
    pub pos_start: Option<i64>,
    #[serde(default)]
    pub pos_end: Option<i64>,
    #[serde(default)]
    pub section_anchor: Option<String>,
    #[serde(default = "default_color")]
    pub color: String,
    #[serde(default)]
    pub note: Option<String>,
}

fn default_color() -> String {
    "iridescent".to_string()
}

#[derive(Debug, Deserialize)]
pub struct MediaPart {
    pub hash: String,
    pub mime: String,
    pub byte_size: i64,
}

#[derive(Debug, Serialize)]
pub struct CaptureResponse {
    pub page: PageResult,
    pub snapshot: SnapshotResult,
    pub highlight: Option<HighlightResult>,
}

#[derive(Debug, Serialize)]
pub struct PageResult {
    pub id: String,
    pub canonical_url: String,
    pub title: String,
    pub highlight_count: i64,
}

#[derive(Debug, Serialize)]
pub struct SnapshotResult {
    pub id: String,
    pub version: i64,
}

#[derive(Debug, Serialize)]
pub struct HighlightResult {
    pub id: String,
}

/// Outcome carried out of the transaction, then rendered as JSON.
struct Outcome {
    response: CaptureResponse,
}

pub async fn create_capture(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> AppResult<impl IntoResponse> {
    let max_image_bytes = state.config.max_image_bytes;

    let mut payload_bytes: Option<Vec<u8>> = None;
    let mut images: HashMap<String, Vec<u8>> = HashMap::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::bad_request(format!("malformed multipart body: {e}")))?
    {
        let name = field.name().unwrap_or_default().to_string();
        if name == "payload" {
            let bytes = field
                .bytes()
                .await
                .map_err(|e| AppError::bad_request(format!("unreadable payload part: {e}")))?;
            payload_bytes = Some(bytes.to_vec());
        } else if let Some(hash) = name.strip_prefix("image/") {
            let hash = hash.to_string();
            let bytes = field
                .bytes()
                .await
                .map_err(|e| AppError::bad_request(format!("unreadable image part: {e}")))?;
            if bytes.len() as u64 > max_image_bytes {
                return Err(AppError::PayloadTooLarge(format!(
                    "image {hash} exceeds the {max_image_bytes}-byte limit"
                )));
            }
            if images.len() >= MAX_IMAGES {
                return Err(AppError::bad_request(format!(
                    "too many images in one capture (max {MAX_IMAGES})"
                )));
            }
            images.insert(hash, bytes.to_vec());
        }
        // Unknown parts are ignored.
    }

    let payload_bytes =
        payload_bytes.ok_or_else(|| AppError::bad_request("missing `payload` part"))?;
    let payload: CapturePayload = serde_json::from_slice(&payload_bytes)
        .map_err(|e| AppError::bad_request(format!("invalid payload JSON: {e}")))?;

    validate(&payload)?;

    // Content addressing is only meaningful if the bytes match their declared
    // hash - verify before we touch disk or the database.
    for (declared, bytes) in &images {
        if &sha256_hex(bytes) != declared {
            return Err(AppError::bad_request(format!(
                "image {declared} does not match its content hash"
            )));
        }
    }

    // Re-validate the canonical URL server-side; never merge across origins.
    let original = payload.url.trim().to_string();
    if url_canon::canonicalize(&original).is_none() {
        return Err(AppError::bad_request("`url` is not a valid http(s) URL"));
    }
    let origin_host = url_canon::host_of(&original);
    let canonical_url = match url_canon::canonicalize(&payload.canonical_url) {
        Some(c) if url_canon::host_of(&c) == origin_host => c,
        _ => url_canon::canonicalize(&original).expect("original already validated"),
    };

    let data_dir = state.db.data_dir().to_path_buf();
    let outcome = state
        .db
        .call(move |conn| commit_capture(conn, &data_dir, payload, canonical_url, original, images))
        .await?;

    Ok((StatusCode::CREATED, axum::Json(outcome.response)))
}

fn validate(payload: &CapturePayload) -> AppResult<()> {
    if payload.capture_id.trim().is_empty() {
        return Err(AppError::bad_request("`capture_id` is required"));
    }
    if payload.page.title.trim().is_empty() {
        return Err(AppError::bad_request("`page.title` is required"));
    }
    if payload.snapshot.content_html.len() > MAX_CONTENT_BYTES {
        return Err(AppError::PayloadTooLarge(
            "snapshot content is too large".into(),
        ));
    }
    if let Some(h) = &payload.highlight {
        if h.quote_exact.trim().is_empty() {
            return Err(AppError::bad_request(
                "a highlight must include `quote_exact`",
            ));
        }
        if h.quote_exact.chars().count() > MAX_QUOTE_CHARS {
            return Err(AppError::PayloadTooLarge(
                "highlight quote is too long".into(),
            ));
        }
        if h.note
            .as_ref()
            .is_some_and(|n| n.chars().count() > MAX_NOTE_CHARS)
        {
            return Err(AppError::PayloadTooLarge("note is too long".into()));
        }
    }
    Ok(())
}

fn commit_capture(
    conn: &mut Connection,
    data_dir: &Path,
    payload: CapturePayload,
    canonical_url: String,
    original_url: String,
    images: HashMap<String, Vec<u8>>,
) -> AppResult<Outcome> {
    let tx = conn.transaction()?;

    // Idempotent replay: a repeated capture_id returns the original result.
    if let Some(existing) = lookup_capture(&tx, &payload.capture_id)? {
        let response = build_response(&tx, &existing)?;
        return Ok(Outcome { response });
    }

    let favicon_path = payload
        .page
        .favicon_hash
        .as_ref()
        .filter(|h| !h.trim().is_empty())
        .map(|h| format!("/media/{h}"));

    let page = pages::upsert_page(
        &tx,
        &NewPage {
            canonical_url,
            original_url,
            title: payload.page.title.clone(),
            site_name: payload.page.site_name.clone(),
            lang: payload.page.lang.clone(),
            favicon_path,
        },
    )?;

    let decision = snapshots::store_or_reuse(
        &tx,
        &page.id,
        |id| format!("snapshots/{id}.html"),
        &NewSnapshot {
            content_hash: payload.snapshot.content_hash.clone(),
            byline: payload.snapshot.byline.clone(),
            excerpt: payload.snapshot.excerpt.clone(),
            word_count: payload.snapshot.word_count,
            partial: payload.snapshot.partial,
        },
    )?;

    if decision.created {
        std::fs::write(
            data_dir.join(&decision.snapshot.content_path),
            payload.snapshot.content_html.as_bytes(),
        )?;
        snapshots::index_fts(
            &tx,
            &decision.snapshot,
            &payload.snapshot.content_text,
            &payload.page.title,
        )?;

        for m in &payload.media {
            let have_bytes = images.get(&m.hash);
            let referenced = if let Some(bytes) = have_bytes {
                media::insert_media(
                    &tx,
                    &Media {
                        hash: m.hash.clone(),
                        mime: m.mime.clone(),
                        byte_size: m.byte_size,
                        created_at: String::new(),
                    },
                )?;
                std::fs::write(data_dir.join("media").join(&m.hash), bytes)?;
                true
            } else {
                media::media_exists(&tx, &m.hash)?
            };
            if referenced {
                snapshots::link_media(&tx, &decision.snapshot.id, &m.hash)?;
            }
        }

        // Persist a favicon blob if it arrived and is new.
        let favicon = payload
            .page
            .favicon_hash
            .as_ref()
            .filter(|h| !h.trim().is_empty());
        if let (Some(hash), Some(bytes)) = (favicon, favicon.and_then(|h| images.get(h))) {
            media::insert_media(
                &tx,
                &Media {
                    hash: hash.clone(),
                    mime: "image/x-icon".into(),
                    byte_size: bytes.len() as i64,
                    created_at: String::new(),
                },
            )?;
            std::fs::write(data_dir.join("media").join(hash), bytes)?;
        }
    }

    let highlight_id = if let Some(h) = &payload.highlight {
        let inserted = highlights::insert_highlight(
            &tx,
            &NewHighlight {
                page_id: page.id.clone(),
                snapshot_id: Some(decision.snapshot.id.clone()),
                quote_exact: h.quote_exact.clone(),
                quote_prefix: h.quote_prefix.clone(),
                quote_suffix: h.quote_suffix.clone(),
                pos_start: h.pos_start,
                pos_end: h.pos_end,
                section_anchor: h.section_anchor.clone(),
                color: h.color.clone(),
                note: h.note.clone(),
            },
        )?;
        Some(inserted.id)
    } else {
        None
    };

    tx.execute(
        "INSERT INTO capture_log(capture_id, page_id, snapshot_id, highlight_id, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            payload.capture_id,
            page.id,
            decision.snapshot.id,
            highlight_id,
            now_iso()
        ],
    )?;

    let highlight_count = pages::highlight_count(&tx, &page.id)?;
    let response = CaptureResponse {
        page: PageResult {
            id: page.id.clone(),
            canonical_url: page.canonical_url.clone(),
            title: page.title.clone(),
            highlight_count,
        },
        snapshot: SnapshotResult {
            id: decision.snapshot.id.clone(),
            version: decision.snapshot.version,
        },
        highlight: highlight_id.map(|id| HighlightResult { id }),
    };

    tx.commit()?;
    Ok(Outcome { response })
}

struct ExistingCapture {
    page_id: String,
    snapshot_id: String,
    highlight_id: Option<String>,
}

fn lookup_capture(
    conn: &Connection,
    capture_id: &str,
) -> rusqlite::Result<Option<ExistingCapture>> {
    conn.query_row(
        "SELECT page_id, snapshot_id, highlight_id FROM capture_log WHERE capture_id = ?1",
        params![capture_id],
        |r| {
            Ok(ExistingCapture {
                page_id: r.get(0)?,
                snapshot_id: r.get(1)?,
                highlight_id: r.get(2)?,
            })
        },
    )
    .optional()
}

fn build_response(conn: &Connection, existing: &ExistingCapture) -> AppResult<CaptureResponse> {
    let page = pages::get_page(conn, &existing.page_id)?
        .ok_or_else(|| AppError::not_found("captured page no longer exists"))?;
    let version: i64 = conn.query_row(
        "SELECT version FROM snapshot WHERE id = ?1",
        params![existing.snapshot_id],
        |r| r.get(0),
    )?;
    let highlight_count = pages::highlight_count(conn, &page.id)?;
    Ok(CaptureResponse {
        page: PageResult {
            id: page.id,
            canonical_url: page.canonical_url,
            title: page.title,
            highlight_count,
        },
        snapshot: SnapshotResult {
            id: existing.snapshot_id.clone(),
            version,
        },
        highlight: existing
            .highlight_id
            .clone()
            .map(|id| HighlightResult { id }),
    })
}
