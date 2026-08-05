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

To keep it running across reboots, see [`packaging/`](../packaging/README.md).

### Configuration

All configuration is environment variables (or flags of the same name); nothing is required for a
local run.

| Variable | Default | Meaning |
| --- | --- | --- |
| `MAGPIE_BIND` | `127.0.0.1:8787` | Listen address. A non-loopback address **requires** a token. |
| `MAGPIE_DATA_DIR` | platform data dir | Where the database, snapshots, and media live. |
| `MAGPIE_TOKEN` | *(unset)* | Shared secret. Unset **and** loopback ⇒ auth disabled. |
| `MAGPIE_ALLOWED_HOSTS` | *(none)* | Extra hostnames to answer to, comma separated. Needed behind a reverse proxy. |
| `MAGPIE_ALLOWED_ORIGINS` | *(none)* | Extension origins allowed via CORS. Rarely needed - the extension asks for host permission instead. |
| `MAGPIE_MAX_IMAGE_BYTES` | `5000000` | Per-image ceiling accepted at capture. |
| `MAGPIE_LOG` | `info` | Log level (`error`/`warn`/`info`/`debug`/`trace`). |

`MAGPIE_DATA_DIR` defaults to your platform's data directory - `~/.local/share/magpie` on Linux,
`~/Library/Application Support/magpie` on macOS, `%APPDATA%\magpie` on Windows. Run
`magpie --help` to see the resolved path on your machine.

> **Upgrading from a build before this default existed?** The data directory used to be `./data`,
> resolved against the working directory. If your collection looks empty after upgrading, it is
> still where you left it:
>
> ```sh
> mkdir -p ~/.local/share/magpie
> mv backend/data/* ~/.local/share/magpie/
> ```

### The data directory

Everything mutable is one directory - copy it to relocate the instance:

```
magpie.db            # SQLite (+ -wal, -shm)
snapshots/<id>.html  # sanitized reader HTML per snapshot version
media/<hash>         # content-addressed images
session.key          # browser session key, created on first run (owner-only)
```

Deleting `session.key` signs every browser out; it is recreated on the next start.

Startup is idempotent: it creates the schema and directories if missing and migrates forward if
present, never clobbering existing data. It refuses to start if the data dir isn't writable, or if
you bind to a non-loopback address without a token.

Migrations are **forward-only**. A newer Magpie will upgrade an older collection on first start; an
older Magpie meeting a newer collection refuses to start rather than touching it. Keep the version
you upgraded to.

> **The data directory must be on local storage.** SQLite runs in WAL mode, and WAL locking is
> unreliable enough on NFS and SMB to corrupt the database. Back up *from* the directory - don't
> run *on* a network mount.

## Installing the extension

```sh
cd extension
npm install
npm run build:firefox   # → extension/dist-firefox   (or `npm run build` for Chromium)
```

In Firefox: `about:debugging` → **This Firefox** → **Load Temporary Add-on** → pick any file in
`extension/dist-firefox`. (A temporary add-on is removed on restart; permanent installs come from
the signed `.xpi` on Releases.) In a Chromium browser: **Extensions → Developer mode → Load
unpacked → `extension/dist`**.

Open the extension's **Options** and set:

- **Backend URL** - `http://127.0.0.1:8787` for a local backend. You can paste the whole connect
  string (`http://host:port#token`) and both fields are filled. Magpie prints it at startup when a
  token is set, and `magpie --connect` prints it any time - or `make connect` against a running
  container.
- **Token** - leave empty for a local loopback backend; required for a remote one.
- **Default shade** - the highlight colour used by the keyboard command / context menu.

**Save** asks for permission to reach that backend, and nothing is stored if you decline. Magpie
requests access to the one address you name rather than to every site, which is why saving is what
grants it. Use **Test connection** afterwards to confirm.

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
2. Set `MAGPIE_ALLOWED_HOSTS` to the public hostname, e.g. `magpie.example.com`. Without it Magpie
   refuses requests carrying that name with `421 Misdirected Request`.
3. Terminate TLS at Caddy/nginx/Traefik and proxy to `127.0.0.1:8787`, forwarding
   `X-Forwarded-Proto` so the session cookie is marked `Secure`.
4. Point the extension's **Backend URL** at `https://magpie.example.com` and set the **Token**.
5. Open the same URL in a browser and sign in with the token once.

### Docker

```sh
make docker-up      # generates a token, starts it, prints the connect string
make connect        # print that string again later
```

Or by hand, with `MAGPIE_TOKEN` set in a `.env` (see [`.env.example`](../.env.example)):

```sh
docker compose up -d
```

The image is distroless and serves a mounted `./data` volume. Inside a container there is no
loopback worth binding to, so a token is always required; the published port is limited to
`127.0.0.1`.

## Security posture

- **Auth** is a single shared secret in two shapes. The extension sends it as a bearer token on
  every API call. Browsers can't attach a bearer header to ordinary navigation, so they exchange it
  once at `/login` for a session cookie (`HttpOnly`, `SameSite=Strict`, and `Secure` when the
  request arrived over HTTPS). The cookie carries a key kept in `session.key`, never the token
  itself, so what sits in the browser can't be replayed against the API. That key is owner-only and
  lives beside the collection - anyone who can read it can already read every highlight next to it.
  Both credentials are compared in constant time. Everything is behind auth except `/healthz`,
  `/login` and static assets.
- **Host allowlist.** Binding to loopback doesn't make Magpie unreachable: every page you visit can
  issue requests to `127.0.0.1`, and DNS rebinding defeats CORS outright by making the response
  same-origin. Requests must carry a `Host` we answer to - loopback, the bind address, or
  `MAGPIE_ALLOWED_HOSTS` - and anything else gets `421`.
- **Known residual risk.** A page you visit can still fire a cross-origin `POST /api/captures` at a
  genuinely local address. `multipart/form-data` is a CORS-safelisted content type, so it is a
  simple request needing no preflight, and it carries a legitimate `Host`. The response stays
  hidden, so this is write-only nuisance rather than disclosure - but a hostile page can put junk in
  your collection on an instance with auth off. Setting a token closes it.
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
