//! Static assets, embedded in the binary (single-binary story) and served with
//! the right content type and a short cache.

use axum::http::header;
use axum::response::IntoResponse;

const APP_CSS: &str = include_str!("../assets/app.css");
const READER_JS: &str = include_str!("../assets/reader.js");

pub async fn app_css() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "text/css; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        APP_CSS,
    )
}

pub async fn reader_js() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "text/javascript; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        READER_JS,
    )
}
