//! End-to-end capture -> read -> search, driven through the real router with
//! `oneshot`. Covers the normalization round-trip (a highlight captured with
//! position offsets must bake onto the exact quote at render) and idempotency.

use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use magpie::config::Config;
use magpie::db::Db;
use magpie::reader::normalize::normalize_str;
use magpie::state::AppState;
use serde_json::{Value, json};
use tower::ServiceExt;

const BOUNDARY: &str = "MAGPIEBOUNDARY";

fn test_state() -> (AppState, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let config = Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        data_dir: dir.path().to_path_buf(),
        token: None,
        allowed_origins: vec![],
        max_image_bytes: 5_000_000,
        log: "warn".into(),
    };
    let db = Db::open(dir.path()).unwrap();
    (
        AppState {
            db,
            config: Arc::new(config),
        },
        dir,
    )
}

fn multipart_capture(payload: &Value) -> Body {
    let json = serde_json::to_string(payload).unwrap();
    let body = format!(
        "--{BOUNDARY}\r\n\
         Content-Disposition: form-data; name=\"payload\"\r\n\
         Content-Type: application/json\r\n\r\n\
         {json}\r\n\
         --{BOUNDARY}--\r\n"
    );
    Body::from(body)
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn body_text(response: axum::response::Response) -> String {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

fn capture_payload() -> (Value, String) {
    // A deliberately simple article so the backend's normalized text equals the
    // content_text an extension would compute - the round-trip precondition.
    let content_html = "<article><p>The quick brown fox jumps over the lazy dog.</p></article>";
    let content_text = normalize_str("The quick brown fox jumps over the lazy dog.");

    // Position offsets exactly as a correct extension would compute them.
    let quote = "brown fox";
    let pos_start = content_text.find(quote).unwrap(); // ASCII: byte == char index
    let pos_end = pos_start + quote.len();

    let payload = json!({
        "capture_id": "cap-int-1",
        "url": "https://example.com/fox?utm_source=x",
        "canonical_url": "https://example.com/fox",
        "page": { "title": "The Fox", "site_name": "Example", "lang": "en" },
        "snapshot": {
            "content_html": content_html,
            "content_text": content_text,
            "excerpt": "A pangram.",
            "word_count": 9,
            "content_hash": "hash-fox-1",
            "partial": false
        },
        "highlight": {
            "quote_exact": quote,
            "quote_prefix": "quick ",
            "quote_suffix": " jumps",
            "pos_start": pos_start,
            "pos_end": pos_end,
            "color": "iridescent",
            "note": "a classic"
        },
        "media": []
    });
    (payload, quote.to_string())
}

#[tokio::test]
async fn capture_read_search_round_trip() {
    let (state, _dir) = test_state();
    let app = magpie::app(state);
    let (payload, quote) = capture_payload();

    // 1. Capture.
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/captures")
                .header(
                    header::CONTENT_TYPE,
                    format!("multipart/form-data; boundary={BOUNDARY}"),
                )
                .body(multipart_capture(&payload))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let created = body_json(response).await;
    let page_id = created["page"]["id"].as_str().unwrap().to_string();
    let highlight_id = created["highlight"]["id"].as_str().unwrap().to_string();
    assert_eq!(created["page"]["highlight_count"], 1);
    assert_eq!(created["snapshot"]["version"], 1);

    // 2. Page JSON: one highlight, a snapshot present.
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/pages/{page_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let detail = body_json(response).await;
    assert_eq!(detail["highlights"].as_array().unwrap().len(), 1);
    assert!(detail["snapshot"].is_object());

    // 3. Reader: the round-trip - the mark lands on the exact quote, and only
    //    the first mark carries the jump-to id.
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/pages/{page_id}/reader"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let html = body_text(response).await;
    let expected_mark = format!("id=\"hl-{highlight_id}\">{quote}</mark>");
    assert!(
        html.contains(&expected_mark),
        "reader HTML must bake <mark> on the exact quote"
    );

    // 4. Search: a content-only term groups under the page.
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/search?q=lazy")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let results = body_json(response).await;
    let groups = results["results"].as_array().unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0]["page"]["id"], page_id.as_str());
    assert!(!groups[0]["hits"].as_array().unwrap().is_empty());

    // 5. Idempotent replay: same capture_id, no duplicate.
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/captures")
                .header(
                    header::CONTENT_TYPE,
                    format!("multipart/form-data; boundary={BOUNDARY}"),
                )
                .body(multipart_capture(&payload))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let replayed = body_json(response).await;
    assert_eq!(replayed["highlight"]["id"], highlight_id.as_str());
    assert_eq!(
        replayed["page"]["highlight_count"], 1,
        "replay must not duplicate"
    );
}

#[tokio::test]
async fn page_only_capture_has_no_highlight() {
    let (state, _dir) = test_state();
    let app = magpie::app(state);

    let payload = json!({
        "capture_id": "cap-page-only",
        "url": "https://example.com/whole",
        "canonical_url": "https://example.com/whole",
        "page": { "title": "Whole Page" },
        "snapshot": {
            "content_html": "<article><p>Just the page.</p></article>",
            "content_text": "Just the page.",
            "content_hash": "hash-whole",
            "partial": false
        },
        "media": []
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/captures")
                .header(
                    header::CONTENT_TYPE,
                    format!("multipart/form-data; boundary={BOUNDARY}"),
                )
                .body(multipart_capture(&payload))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let created = body_json(response).await;
    assert!(created["highlight"].is_null());
    assert_eq!(created["page"]["highlight_count"], 0);
}
