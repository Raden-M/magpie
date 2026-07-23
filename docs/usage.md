# Using Magpie

Magpie is two pieces: a **backend** (a single Rust binary that stores, searches, and serves your
collection) and a **browser extension** (that captures highlights from the pages you read). This
guide covers running both, deploying remotely, and the security posture.

## Running the backend

### From source

```sh
cd backend
cargo run            # serves http://127.0.0.1:8787
```

On loopback with no token, auth is disabled - open <http://127.0.0.1:8787> and you have your
collection. That's the whole local setup.

### As a binary

```sh
cd backend
cargo build --release
./target/release/magpie
```

### Configuration

All configuration is environment variables (or flags of the same name); nothing is required for a
local run.

| Variable | Default | Meaning |
| --- | --- | --- |
| `MAGPIE_BIND` | `127.0.0.1:8787` | Listen address. A non-loopback address **requires** a token. |
| `MAGPIE_DATA_DIR` | `./data` | Where the database, snapshots, and media live. |
| `MAGPIE_TOKEN` | *(unset)* | Bearer token. Unset **and** loopback ⇒ auth disabled. |
| `MAGPIE_ALLOWED_ORIGINS` | *(none)* | Comma-separated extension origins allowed via CORS. |
| `MAGPIE_MAX_IMAGE_BYTES` | `5000000` | Per-image ceiling accepted at capture. |
| `MAGPIE_LOG` | `info` | Log level (`error`/`warn`/`info`/`debug`/`trace`). |

### The data directory

Everything mutable is one directory - copy it to relocate the instance:

```
data/
  magpie.db            # SQLite (+ -wal, -shm)
  snapshots/<id>.html  # sanitized reader HTML per snapshot version
  media/<hash>         # content-addressed images
```

Startup is idempotent: it creates the schema and directories if missing and migrates forward if
present, never clobbering existing data. It refuses to start if the data dir isn't writable, or if
you bind to a non-loopback address without a token.

## Installing the extension

```sh
cd extension
npm install
npm run build        # → extension/dist
```

Then, in a Chromium browser: **Extensions → Developer mode → Load unpacked → select
`extension/dist`**. Open the extension's **Options** and set:

- **Backend URL** - `http://127.0.0.1:8787` for a local backend.
- **Token** - leave empty for a local loopback backend; required for a remote one.
- **Default shade** - the highlight colour used by the keyboard command / context menu.

Use **Test connection** to confirm it can reach the backend.

### Capturing

- **Select text** on any page → a pill appears → pick a shade, optionally **＋ Note**, then **Keep**.
- **Right-click a selection** → *Keep selection in Magpie*.
- **Keyboard**: `Ctrl/Cmd+Shift+K` keeps the current selection.

You'll see **"Kept. N shiny things on this page."** Open the web UI to read it back with your
highlights baked in, grouped by page, and full-text searchable.

A capture with no selection stores the page and its reader snapshot with zero highlights.

## Deploying remotely

Magpie speaks plain HTTP and expects to sit behind a reverse proxy that terminates TLS. It never
makes outbound requests, so it has no SSRF surface and can run airgapped apart from the extension's
calls.

```
Browser ⇢ (extension, token) ⇢ HTTPS ⇢ reverse proxy (TLS) ⇢ 127.0.0.1:8787 ⇢ magpie
```

1. Run `magpie` bound to loopback or a private interface, with `MAGPIE_TOKEN` set.
2. Set `MAGPIE_ALLOWED_ORIGINS` to your installed extension's origin, e.g.
   `chrome-extension://<your-extension-id>`.
3. Terminate TLS at Caddy/nginx/Traefik and proxy to `127.0.0.1:8787`.
4. Point the extension's **Backend URL** at `https://magpie.example.com` and set the **Token**.

### Docker

```sh
cd backend
docker build -t magpie:latest .
```

Or with the provided compose file from the repo root (set `MAGPIE_TOKEN` in a `.env` first):

```sh
MAGPIE_TOKEN=$(openssl rand -hex 32) docker compose up -d
```

The image is distroless and serves a mounted `./data` volume.

## Security posture

- **Auth**: a single bearer token, compared in constant time. Enforced on everything except
  `/healthz` and static assets whenever a token is set (or the bind is non-loopback). Browsers can't
  send a bearer header on navigation, so protect remote **human** access to the web UI at the reverse
  proxy (basic auth, VPN, or IP allowlist); the token guards the extension's API path.
- **CSP on served snapshots**: the reader view is served under a strict `Content-Security-Policy`
  (`default-src 'none'`, no inline scripts) even though captures are sanitized at capture time. A
  stored snapshot can never execute script or load remote resources. Defense in depth - the backend
  does not trust the client's sanitization.
- **Media** are served only from `/media/<hash>` on your own origin; snapshots reference nothing
  remote (all images are rewritten to local hashes at capture).
- **Input limits**: request body size, per-image bytes, images per capture, and quote/note lengths
  are all capped.
- **The token** is stored in `chrome.storage.local` (not synced), so it is never uploaded to the
  browser vendor's sync.

## Backups

There is **no built-in backup** (a deliberate non-goal). All your data is one directory, so back it
up with whatever you already use (`restic`, `rsync`, a filesystem snapshot). Because SQLite uses a
WAL, copy it safely rather than mid-write - either stop Magpie first, or use SQLite's
`VACUUM INTO '/path/backup.db'` against a running instance to get a clean, single-file copy.
