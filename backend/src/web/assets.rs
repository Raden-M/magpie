//! Static assets, embedded in the binary (single-binary story) and served with
//! the right content type and a short cache.

use std::sync::OnceLock;

use axum::http::header;
use axum::response::IntoResponse;

const APP_CSS: &str = include_str!("../assets/app.css");
const READER_JS: &str = include_str!("../assets/reader.js");
const DOCK_JS: &str = include_str!("../assets/dock.js");
const THEME_JS: &str = include_str!("../assets/theme.js");
const COLLECTION_JS: &str = include_str!("../assets/collection.js");
/// The wordmark face: Bricolage Grotesque, subset to the six letters of "Magpie"
/// and pinned to one instance (~2.6 KB). Licensed under the SIL OFL 1.1; the
/// licence travels beside it as `magpie-wordmark.OFL.txt`. Served rather than
/// inlined as a `data:` URI so it satisfies the `font-src 'self'` in `PAGE_CSP`.
const WORDMARK: &[u8] = include_bytes!("../assets/magpie-wordmark.woff2");

/// The two editorial faces, self-hosted for the same reason the wordmark is (the
/// `font-src 'self'` in `PAGE_CSP`, and no CDN dependency). Both are the Google
/// Fonts `latin` subset as variable woff2, licensed under the SIL OFL 1.1 with
/// the licence beside each file. `Fraunces` sets display type (titles, headings);
/// `Newsreader` sets the reading column and everything in italic. The `latin`
/// subset covers Latin-1 (café, naïve, Zürich); text outside it falls back to
/// Georgia, so a Latin-Extended article degrades rather than breaks.
const FRAUNCES: &[u8] = include_bytes!("../assets/fraunces.woff2");
const NEWSREADER: &[u8] = include_bytes!("../assets/newsreader.woff2");
const NEWSREADER_ITALIC: &[u8] = include_bytes!("../assets/newsreader-italic.woff2");

/// A font is referenced from inside `app.css`, which can't carry a URL the way a
/// page's `<link>` can, so each `@font-face` names one of these tokens and the
/// served CSS has them replaced with the fingerprinted URLs. The stylesheet's own
/// fingerprint is taken from the raw source, so editing the CSS still busts it.
const WORDMARK_URL_TOKEN: &str = "__MAGPIE_WORDMARK_WOFF2__";
const FRAUNCES_URL_TOKEN: &str = "__FRAUNCES_WOFF2__";
const NEWSREADER_URL_TOKEN: &str = "__NEWSREADER_WOFF2__";
const NEWSREADER_ITALIC_URL_TOKEN: &str = "__NEWSREADER_ITALIC_WOFF2__";

/// Assets are `include_str!`-baked, so a rebuild is all it should take to see an
/// edit. A cached copy in the browser outlives the rebuild and hides it, so
/// debug builds opt out of caching entirely.
#[cfg(debug_assertions)]
const CACHE: &str = "no-cache";
#[cfg(not(debug_assertions))]
const CACHE: &str = "public, max-age=3600";

/// Cache-busting: pages link the assets by a URL carrying a fingerprint of the
/// bytes, so editing one changes where the browser is told to look. Headers
/// alone can't do this - a copy already held under an older `max-age` is still
/// fresh, and the browser serves it without asking.
pub fn app_css_url() -> &'static str {
    static URL: OnceLock<String> = OnceLock::new();
    URL.get_or_init(|| format!("/assets/app.css?v={:08x}", fingerprint(APP_CSS.as_bytes())))
}

pub fn reader_js_url() -> &'static str {
    static URL: OnceLock<String> = OnceLock::new();
    URL.get_or_init(|| {
        format!(
            "/assets/reader.js?v={:08x}",
            fingerprint(READER_JS.as_bytes())
        )
    })
}

pub fn dock_js_url() -> &'static str {
    static URL: OnceLock<String> = OnceLock::new();
    URL.get_or_init(|| format!("/assets/dock.js?v={:08x}", fingerprint(DOCK_JS.as_bytes())))
}

pub fn theme_js_url() -> &'static str {
    static URL: OnceLock<String> = OnceLock::new();
    URL.get_or_init(|| {
        format!(
            "/assets/theme.js?v={:08x}",
            fingerprint(THEME_JS.as_bytes())
        )
    })
}

pub fn collection_js_url() -> &'static str {
    static URL: OnceLock<String> = OnceLock::new();
    URL.get_or_init(|| {
        format!(
            "/assets/collection.js?v={:08x}",
            fingerprint(COLLECTION_JS.as_bytes())
        )
    })
}

pub fn wordmark_woff2_url() -> &'static str {
    static URL: OnceLock<String> = OnceLock::new();
    URL.get_or_init(|| {
        format!(
            "/assets/magpie-wordmark.woff2?v={:08x}",
            fingerprint(WORDMARK)
        )
    })
}

pub fn fraunces_woff2_url() -> &'static str {
    static URL: OnceLock<String> = OnceLock::new();
    URL.get_or_init(|| format!("/assets/fraunces.woff2?v={:08x}", fingerprint(FRAUNCES)))
}

pub fn newsreader_woff2_url() -> &'static str {
    static URL: OnceLock<String> = OnceLock::new();
    URL.get_or_init(|| format!("/assets/newsreader.woff2?v={:08x}", fingerprint(NEWSREADER)))
}

pub fn newsreader_italic_woff2_url() -> &'static str {
    static URL: OnceLock<String> = OnceLock::new();
    URL.get_or_init(|| {
        format!(
            "/assets/newsreader-italic.woff2?v={:08x}",
            fingerprint(NEWSREADER_ITALIC)
        )
    })
}

/// `app.css` with the wordmark token resolved to its fingerprinted URL, built
/// once. The `@font-face` `src` can't be a Rust-generated string on its own, so
/// the substitution happens here at serve time.
fn app_css_body() -> &'static str {
    static BODY: OnceLock<String> = OnceLock::new();
    BODY.get_or_init(|| {
        APP_CSS
            .replace(WORDMARK_URL_TOKEN, wordmark_woff2_url())
            .replace(FRAUNCES_URL_TOKEN, fraunces_woff2_url())
            .replace(NEWSREADER_URL_TOKEN, newsreader_woff2_url())
            .replace(NEWSREADER_ITALIC_URL_TOKEN, newsreader_italic_woff2_url())
    })
}

/// FNV-1a. Collisions only matter across successive edits of one file, where
/// any change of value is enough - this is not a checksum.
const fn fingerprint(bytes: &[u8]) -> u32 {
    let mut hash: u32 = 0x811c_9dc5;
    let mut i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u32;
        hash = hash.wrapping_mul(0x0100_0193);
        i += 1;
    }
    hash
}

pub async fn app_css() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "text/css; charset=utf-8"),
            (header::CACHE_CONTROL, CACHE),
        ],
        app_css_body(),
    )
}

pub async fn wordmark_woff2() -> impl IntoResponse {
    woff2(WORDMARK)
}

pub async fn fraunces_woff2() -> impl IntoResponse {
    woff2(FRAUNCES)
}

pub async fn newsreader_woff2() -> impl IntoResponse {
    woff2(NEWSREADER)
}

pub async fn newsreader_italic_woff2() -> impl IntoResponse {
    woff2(NEWSREADER_ITALIC)
}

fn woff2(bytes: &'static [u8]) -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "font/woff2"),
            (header::CACHE_CONTROL, CACHE),
        ],
        bytes,
    )
}

pub async fn theme_js() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "text/javascript; charset=utf-8"),
            (header::CACHE_CONTROL, CACHE),
        ],
        THEME_JS,
    )
}

pub async fn reader_js() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "text/javascript; charset=utf-8"),
            (header::CACHE_CONTROL, CACHE),
        ],
        READER_JS,
    )
}

pub async fn dock_js() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "text/javascript; charset=utf-8"),
            (header::CACHE_CONTROL, CACHE),
        ],
        DOCK_JS,
    )
}

pub async fn collection_js() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "text/javascript; charset=utf-8"),
            (header::CACHE_CONTROL, CACHE),
        ],
        COLLECTION_JS,
    )
}
