# Magpie

A single-user, self-hosted highlight keeper: a browser extension captures a text selection plus a
reader-view snapshot of the page, and a Rust backend stores, searches and serves the collection.

Global conventions live in `~/.claude/` and load automatically; they are not restated here.

## Commands

```sh
make dev              # build the Chromium extension, then run the backend
make dev-firefox      # same, but build the Firefox variant into extension/dist-firefox
make test             # cargo test + npm test
make build            # release binary + extension
make docker-up        # run in Docker, generating a token on first start
make lint-firefox     # addons-linter over the Firefox build (--self-hosted)
make licenses         # regenerate THIRD-PARTY.md
```

`make help` lists the rest. The backend serves <http://127.0.0.1:8787>. On loopback with no token,
auth is disabled.

## Conventions

- The backend never makes outbound requests. Anything needing the live page belongs in the
  extension; anything that can live in the backend does, because shipping extension updates is
  slow and serving a backend is not.
- The extension sanitizes at capture time *and* the backend serves snapshots under a strict CSP.
  Both are load-bearing - do not drop one because the other exists.
- The design is mobile-first throughout: base CSS targets the phone, `min-width` queries layer up.
  Never a wide-first layout with `max-width` overrides.

## Gotchas

- The HTTP API is **not versioned**: routes are bare `/api/...` with no version segment and no
  compatibility check between the two halves. This gets sharper once the extension auto-updates
  independently of the backend.
- Auth has **two credential shapes for one secret**: the extension sends a bearer token, browsers
  trade it at `/login` for a session cookie. Anything touching `api/auth.rs` has to keep both, or
  one of the two halves silently loses access.
- **Loopback is not a security boundary.** Pages you visit can reach `127.0.0.1`, so the acceptable
  `Host` is pinned in `api/host_guard.rs`. A cross-origin `POST /api/captures` still gets through by
  design (`multipart/form-data` is CORS-safelisted) - documented, not forgotten.
- `MAGPIE_DATA_DIR` resolves to the **platform data directory**, not `./data`. Containers set it
  explicitly. Collections created before this change are still in `backend/data/`.

## Reference

- `docs/architecture.md` - how the two halves fit together, and the capture flow.
- `docs/usage.md` - running, configuring, deploying, and the security posture.
- `docs/decisions/` - one decision record per file, `YYYY-MM-DD-slug.md`.
- `packaging/README.md` - systemd unit and the always-running setups.
- `CONTRIBUTING.md` - what a contributor is held to; `SECURITY.md` for reports.
- `.claude/rules/` - instructions scoped to `backend/` and `extension/`, loaded on demand.
- `THIRD-PARTY.md` is **generated** (`make licenses`) - never hand-edit it.
