//! End-to-end capture -> read -> search, driven through the real router with
//! `oneshot`. Covers the normalization round-trip (a highlight captured with
//! position offsets must bake onto the exact quote at render) and idempotency.

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
    test_state_with(|_| {})
}

/// A state whose config can be adjusted before the app is built - auth and the
/// host allowlist are configured, so they can only be exercised this way.
fn test_state_with(tweak: impl FnOnce(&mut Config)) -> (AppState, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let mut config = Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        data_dir: dir.path().to_path_buf(),
        token: None,
        allowed_origins: vec![],
        allowed_hosts: vec![],
        public_url: None,
        max_image_bytes: 5_000_000,
        log: "warn".into(),
        connect: false,
    };
    tweak(&mut config);
    let db = Db::open(dir.path()).unwrap();
    (AppState::new(db, config), dir)
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
async fn collection_search_filters_live_and_renders_snippets() {
    let (state, _dir) = test_state();
    let app = magpie::app(state);
    let (payload, _quote) = capture_payload();

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

    // The full page carries the search box above the list.
    let response = app
        .clone()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let home = body_text(response).await;
    assert!(
        home.contains("class=\"collection-search\""),
        "collection page renders the search box"
    );
    assert!(
        home.contains("The Fox"),
        "an unfiltered collection lists the kept page"
    );

    // The live-search fragment: a content term matches and the snippet is bolded.
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/partials/collection?q=lazy")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let fragment = body_text(response).await;
    assert!(
        fragment.contains("collection-row") && fragment.contains("The Fox"),
        "a matching query surfaces the page as a row"
    );
    assert!(
        fragment.contains("<b>lazy</b>"),
        "the row preview bolds the matched term, got: {fragment}"
    );
    // The fragment is the list only - no page chrome.
    assert!(
        !fragment.contains("class=\"collection-search\""),
        "the fragment carries no search box of its own"
    );

    // A term that matches nothing yields the empty state, not a stale list.
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/partials/collection?q=zznomatch")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let empty = body_text(response).await;
    assert!(
        empty.contains("Nothing matches") && !empty.contains("collection-row"),
        "a non-matching query shows the empty state, got: {empty}"
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

#[tokio::test]
async fn ui_csp_allows_collection_search() {
    // Regression: the collection search box upgrades to live search by fetching
    // the `/partials/collection` fragment, and falls back to a plain `/` GET when
    // script is off. A CSP without `connect-src 'self'` blocks the fetch, and one
    // with `form-action 'none'` blocks the fallback submit - either way a typed
    // query never reaches the (working) backend. Both the page and the fragment
    // must permit same-origin connections and form submission.
    let (state, _dir) = test_state();
    let app = magpie::app(state);

    for uri in ["/", "/?q=anything", "/partials/collection?q=anything"] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{uri} renders");
        let csp = response
            .headers()
            .get(header::CONTENT_SECURITY_POLICY)
            .expect("UI page sets a CSP")
            .to_str()
            .unwrap();
        assert!(
            csp.contains("connect-src 'self'"),
            "{uri} CSP must allow the live-search fetch, got: {csp}"
        );
        assert!(
            csp.contains("form-action 'self'"),
            "{uri} CSP must allow the search form to fall back to a `/` submit, got: {csp}"
        );
        assert!(
            !csp.contains("form-action 'none'"),
            "{uri} CSP `form-action 'none'` blocks the search fallback"
        );
    }
}

#[tokio::test]
async fn reader_defaults_to_snippets_with_a_live_link_and_a_full_backup() {
    let (state, _dir) = test_state();
    let app = magpie::app(state);
    let (payload, _quote) = capture_payload();

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
    let page_id = body_json(response).await["page"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Default view: a snippet card, the note, and a Text-Fragment deep link back
    // to the live page at the highlighted words.
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
    let html = body_text(response).await;
    assert!(
        html.contains("class=\"snippet\""),
        "default view is snippets"
    );
    assert!(
        html.contains("a classic"),
        "the note rides along with the snippet"
    );
    assert!(
        html.contains("#:~:text=brown%20fox"),
        "the live link carries a text fragment for the exact quote"
    );
    assert!(
        html.contains("view=full"),
        "a toggle offers the full snapshot backup"
    );
    assert!(
        !html.contains("class=\"article-body\""),
        "the full snapshot body is not rendered by default"
    );

    // The backup: the whole captured article, marks baked in.
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/pages/{page_id}/reader?view=full"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let html = body_text(response).await;
    assert!(
        html.contains("class=\"article-body\""),
        "?view=full renders the full snapshot"
    );
    assert!(
        html.contains("over the lazy dog"),
        "full view keeps text outside the highlight's snippet"
    );
}

/// A locked instance: the token is what the extension would send.
fn locked_state() -> (AppState, tempfile::TempDir) {
    test_state_with(|c| c.token = Some("s3cret".into()))
}

#[tokio::test]
async fn foreign_host_is_refused_before_anything_else() {
    let (state, _dir) = test_state();
    let app = magpie::app(state);

    // The shape of a DNS-rebinding request: the attacker's name, our address.
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/")
                .header(header::HOST, "evil.example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.status(),
        StatusCode::MISDIRECTED_REQUEST,
        "a hostname we don't answer to is refused"
    );

    for host in ["127.0.0.1:8787", "localhost", "[::1]:8787"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header(header::HOST, host)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{host} reaches the app");
    }
}

#[tokio::test]
async fn a_proxy_hostname_is_reachable_once_allowed() {
    let (state, _dir) = test_state_with(|c| c.allowed_hosts = vec!["magpie.example.com".into()]);
    let app = magpie::app(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/")
                .header(header::HOST, "magpie.example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn locked_instance_redirects_browsers_but_keeps_json_for_the_api() {
    let (state, _dir) = locked_state();
    let app = magpie::app(state);

    // A browser navigating with no credentials is sent to the form, not handed
    // JSON it would render as text.
    let response = app
        .clone()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(response.headers()[header::LOCATION], "/login");

    // The extension's path keeps the JSON 401 its error handling reads.
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/pages")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(body_json(response).await["error"]["code"], "unauthorized");

    // The form itself must stay reachable, or there is no way in.
    let response = app
        .oneshot(
            Request::builder()
                .uri("/login")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn the_right_token_trades_for_a_session_that_opens_the_collection() {
    let (state, _dir) = locked_state();
    let app = magpie::app(state);

    let post_login = |body: &'static str| {
        app.clone().oneshot(
            Request::builder()
                .method("POST")
                .uri("/login")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(body))
                .unwrap(),
        )
    };

    let response = post_login("token=wrong").await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "a wrong token mints no session"
    );
    assert!(response.headers().get(header::SET_COOKIE).is_none());

    let response = post_login("token=s3cret").await.unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let cookie = response.headers()[header::SET_COOKIE].to_str().unwrap();
    assert!(cookie.contains("HttpOnly"), "not readable from script");
    assert!(cookie.contains("SameSite=Strict"), "not sent cross-site");
    assert!(
        !cookie.contains("s3cret"),
        "the token itself never reaches the browser"
    );

    // The session it just handed out opens the collection.
    let session = cookie.split(';').next().unwrap().to_string();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/")
                .header(header::COOKIE, session)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn a_lan_address_reaches_an_instance_that_named_no_hostname() {
    // Bound to everything, as a container or a LAN service is, with nothing in
    // the allowlist. Reaching it by address must not need configuration first.
    // A non-loopback bind forces auth on, so the extension's token comes too.
    let (state, _dir) = test_state_with(|c| {
        c.bind = "0.0.0.0:8787".parse().unwrap();
        c.token = Some("s3cret".into());
    });
    let app = magpie::app(state);

    for host in ["192.168.1.50:8787", "10.0.0.8", "[fe80::1]:8787"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header(header::HOST, host)
                    .header(header::AUTHORIZATION, "Bearer s3cret")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{host} reaches the app");
    }

    // A name still has to be vouched for, because a name is what gets rebound.
    let response = app
        .oneshot(
            Request::builder()
                .uri("/")
                .header(header::HOST, "evil.example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::MISDIRECTED_REQUEST);
}

#[tokio::test]
async fn the_public_url_host_is_allowed_without_repeating_it() {
    let (state, _dir) =
        test_state_with(|c| c.public_url = Some("https://magpie.example.com".into()));
    let app = magpie::app(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/")
                .header(header::HOST, "magpie.example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "naming the public URL is enough; the allowlist need not repeat it"
    );
}
