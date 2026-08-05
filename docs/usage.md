# Using Magpie

Getting started is in the [README](../README.md). This is the reference behind it: configuring the
backend, the extension's settings, running it on a server, and what the security posture actually
buys you.

## Running the backend

```sh
cd backend
cargo run               # serves http://127.0.0.1:8787
cargo build --release   # or a binary at ./target/release/magpie
```

On loopback with no token, auth is disabled - open <http://127.0.0.1:8787> and the collection is
there. That's the whole local setup. To keep it running across reboots, see
[`packaging/`](../packaging/README.md).

### Configuration

Environment variables, or flags of the same name. Nothing is required for a local run.

| Variable | Default | Meaning |
| --- | --- | --- |
| `MAGPIE_BIND` | `127.0.0.1:8787` | Listen address. A non-loopback address **requires** a token. |
| `MAGPIE_DATA_DIR` | platform data dir | Where the database, snapshots, and media live. |
| `MAGPIE_TOKEN` | *(unset)* | Shared secret. Unset **and** loopback ⇒ auth disabled. |
| `MAGPIE_PUBLIC_URL` | *(none)* | The URL this is reached at from outside, e.g. `https://magpie.example.com`. Needed behind a reverse proxy. |
| `MAGPIE_ALLOWED_HOSTS` | *(none)* | Further hostnames to answer to, comma separated. Rarely needed. |
| `MAGPIE_ALLOWED_ORIGINS` | *(none)* | Extension origins allowed via CORS. Rarely needed - the extension asks for host permission instead. |
| `MAGPIE_MAX_IMAGE_BYTES` | `5000000` | Per-image ceiling accepted at capture. |
| `MAGPIE_LOG` | `info` | Log level (`error`/`warn`/`info`/`debug`/`trace`). |

`MAGPIE_DATA_DIR` defaults to your platform's data directory - `~/.local/share/magpie` on Linux,
`~/Library/Application Support/magpie` on macOS, `%APPDATA%\magpie` on Windows. `magpie --help`
prints the resolved path on your machine.

> **Collection looks empty after upgrading?** The default used to be `./data`, resolved against the
> working directory. Your data is still where you left it:
>
> ```sh
> mkdir -p ~/.local/share/magpie && mv backend/data/* ~/.local/share/magpie/
> ```

### The data directory

Everything mutable is one directory - copy it to relocate the instance:

```
magpie.db            # SQLite (+ -wal, -shm)
snapshots/<id>.html  # sanitized reader HTML per snapshot version
media/<hash>         # content-addressed images
session.key          # browser session key, created on first run (owner-only)
```

Startup is idempotent: it creates what is missing, migrates forward what is present, and never
clobbers existing data. It refuses to start if the directory isn't writable, or if you bind to a
non-loopback address without a token.

Migrations are **forward-only**. A newer Magpie upgrades an older collection on first start; an
older Magpie meeting a newer collection refuses to start rather than touching it. Keep the version
you upgraded to.

Deleting `session.key` signs every browser out, and it is recreated on the next start. Changing
`MAGPIE_TOKEN` does the same, because the cookie is derived from the key *and* the token - rotating
a leaked token is enough on its own to revoke every browser signed in with it.

> **Keep the directory on local storage.** SQLite runs in WAL mode, and WAL locking is unreliable
> enough on NFS and SMB to corrupt the database. Back up *from* the directory - don't run *on* a
> network mount.

## The extension

Installing it is in the [README](../README.md#get-started). **Click the Magpie icon in the toolbar**
to open its settings - the icon has no popup, so that click is what opens them (`about:addons` →
Magpie → Preferences works too):

- **Backend URL** - `http://127.0.0.1:8787` for a local backend. Pasting a whole connect string
  (`http://host:port#token`) fills both fields; `magpie --connect` prints one, or `make connect`
  against a running container.
- **Token** - empty for a local loopback backend, required for a remote one.
- **Default shade** - the highlight colour used by the keyboard command and context menu.

**Save** asks for permission to reach that backend and stores nothing if you decline - Magpie
requests the one address you name rather than every site, which is why saving is what grants it.
**Test connection** confirms it.

### Capturing

- **Select text** on any page → a pill appears → pick a shade, optionally **＋ Note**, then **Keep**.
- **Right-click a selection** → *Keep selection in Magpie*.
- **`Ctrl/Cmd+Shift+K`** keeps the current selection.

A capture with no selection stores the page and its reader snapshot with zero highlights.

## Deploying remotely

Magpie speaks plain HTTP and expects to sit behind a reverse proxy that terminates TLS.

```
Browser ⇢ (extension, token) ⇢ HTTPS ⇢ reverse proxy (TLS) ⇢ 127.0.0.1:8787 ⇢ magpie
```

1. Run `magpie` bound to loopback or a private interface, with `MAGPIE_TOKEN` set.
2. Set `MAGPIE_PUBLIC_URL` to the address you reach it at, scheme included - the scheme is never
   guessed, because a proxy speaking plain HTTP on a LAN is a real setup. That hostname is then
   allowed; without it, requests carrying that name get `421 Misdirected Request`.
3. Terminate TLS at Caddy/nginx/Traefik and proxy to `127.0.0.1:8787`, forwarding
   `X-Forwarded-Proto` so the session cookie is marked `Secure`.
4. Paste `magpie --connect` into the extension's **Backend URL**. It carries the public URL and the
   token together.
5. Open the same URL in a browser and sign in with the token once.

Reaching an instance by **IP address** needs none of this. `MAGPIE_BIND=0.0.0.0:8787` is directly
usable at `http://192.168.1.50:8787`, because an address cannot be DNS-rebound - see the security
posture below.

### Docker

```sh
make docker-up      # generates a token, starts it, prints the connect string
make connect        # print that string again later
```

Or by hand, with `MAGPIE_TOKEN` set in a `.env` (see [`.env.example`](../.env.example)):
`docker compose up -d`. The image is distroless and serves a mounted `./data` volume. Inside a
container there is no loopback worth binding to, so a token is always required; the published port
is limited to `127.0.0.1`.

## Security posture

**Auth is one secret in two shapes.** The extension sends it as a bearer token; browsers can't
attach that header to ordinary navigation, so they exchange it once at `/login` for a session cookie
(`HttpOnly`, `SameSite=Lax`, and `Secure` over HTTPS). The cookie carries `HMAC(session.key, token)`
rather than the token, so what sits in the browser can't be replayed against the API, and rotating
the token invalidates every outstanding cookie. Both credentials are compared in constant time,
wrong tokens at `/login` are throttled into uselessness, and everything
is behind auth except `/healthz`, `/login` and static assets.

**Loopback is not a boundary.** Every page you visit can issue requests to `127.0.0.1`, and DNS
rebinding defeats CORS outright by making the response same-origin. So requests must carry a `Host`
we answer to - `localhost`, any IP address, the host of `MAGPIE_PUBLIC_URL`, or
`MAGPIE_ALLOWED_HOSTS` - and anything else gets `421`. Bare addresses are fine because rebinding is
an attack on a *name*: a browser only sends one when it was pointed straight at it, and then the
origin is that address and ordinary CORS applies.

**One residual risk.** A page you visit can still fire a cross-origin `POST /api/captures` at a
genuinely local address: `multipart/form-data` is CORS-safelisted, so it needs no preflight and
carries a legitimate `Host`. The response stays hidden, making it write-only nuisance rather than
disclosure - but a hostile page can put junk in a collection running with auth off. A token closes
it.

**Snapshots can't execute.** They are sanitized with `DOMPurify` at capture time and served under a
strict `Content-Security-Policy` (`default-src 'none'`, no inline scripts) anyway - the backend does
not trust the client's sanitization. Images are rewritten to local hashes at capture and served only
from `/media/<hash>` on your own origin, so a snapshot references nothing remote.

**Limits.** Request body size, per-image bytes, images per capture, and quote/note lengths are all
capped. The token lives in `chrome.storage.local`, which is never uploaded to the browser vendor's
sync.

## Backups

There is **no built-in backup**, [deliberately](decisions/2026-07-23-backups-out-of-scope.md). All
your data is one directory, so back it up with whatever you already use (`restic`, `rsync`, a
filesystem snapshot). Because SQLite uses a WAL, copy it safely rather than mid-write - either stop
Magpie first, or use `VACUUM INTO '/path/backup.db'` against a running instance to get a clean,
single-file copy.
