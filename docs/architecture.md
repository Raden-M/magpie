# Architecture

Magpie is two codebases with an HTTP API between them.

## Browser extension (TypeScript, Manifest V3)

Runs in the user's browser, in their authenticated session. It owns **everything that needs the
live page**.

- **Content script** (`src/content/`) - reads `window.getSelection()`, computes anchoring
  selectors, runs reader-view extraction (`Readability.js` + `DOMPurify`), collects images, and
  renders the in-page capture pill.
- **Service worker** (`src/background/`) - owns the context-menu item, the keyboard command, and
  all backend communication. MV3 service workers are ephemeral, so no long-lived in-memory state.
- **Options page** (`src/options/`) - backend URL, token, default highlight shade.

`manifest.json` is the canonical Chromium manifest; the Firefox variant is derived from it at
build time by `build.mjs`.

## Backend (Rust)

A single static binary. It owns **storage, search and serving**, and nothing that needs a browser.
It never re-fetches pages and needs no outbound network.

- **HTTP API** (`axum`, `src/api/`) - receives captures; serves pages, highlights and search.
- **SQLite + FTS5** (`rusqlite`, bundled) - all structured data and full-text search.
- **Blobs on disk** - snapshot HTML and content-hashed media, referenced from the database.
- **Web UI** (`maud`, `src/web/`) - server-rendered collection, reader view and search.

## Capture flow

```
 USER                EXTENSION (browser)                     BACKEND (Rust)
  │                        │                                      │
  │ select text +          │                                      │
  │ "Keep in Magpie" ────▶ │                                      │
  │                        │ 1. read selection                    │
  │                        │    → TextQuoteSelector (quote,       │
  │                        │      prefix, suffix) + position      │
  │                        │ 2. Readability.js on a DOM clone     │
  │                        │ 3. DOMPurify sanitize                │
  │                        │ 4. collect + fetch images as blobs   │
  │                        │ 5. POST /api/captures ──────────────▶│
  │                        │                                      │ 6. upsert Page by
  │                        │                                      │    canonical_url
  │                        │                                      │ 7. version Snapshot,
  │                        │                                      │    write media to disk
  │                        │                                      │ 8. insert Highlight
  │                        │                                      │ 9. index into FTS5
  │                        │ ◀──────────────── 201 {page,highlight}
  │ ◀── "Kept. N shiny     │                                      │
  │     things on page"    │                                      │
```

## Trust boundaries

- **Content flows one way**, browser to the user's own backend. The backend never calls the origin
  site, so it has no SSRF surface and can run airgapped.
- **The backend is the source of truth.** The extension holds only configuration and transient
  capture state.
- **Sanitization happens twice on purpose**: `DOMPurify` at capture time in the extension, and a
  strict `Content-Security-Policy` on every served snapshot. The backend does not trust that the
  client sanitized correctly.
- **Auth** is a single bearer token, compared in constant time. Disabled only when bound to
  loopback with no token set; a non-loopback bind without a token refuses to start.

## Technology

| Layer | Choice | Notes |
| --- | --- | --- |
| Extension | TypeScript, MV3 | `Readability.js` (Apache-2.0), `DOMPurify` (Apache-2.0/MPL) |
| Backend | Rust + `axum` | Single static binary |
| DB / search | SQLite + FTS5 via `rusqlite` (bundled) | BM25 ranking; no second daemon |
| Blobs | Files on disk | Snapshots and content-hashed media |
| Web UI | Server-rendered `maud` | Reader view is static content plus a small JS island |
| Packaging | Single binary + distroless container | Reverse proxy terminates TLS |

## Known gap

The API is **unversioned** - routes are bare `/api/...` with no version segment, header, or
compatibility check. That is fine while both halves are built together, and becomes a problem the
day the extension auto-updates independently of the backend.
