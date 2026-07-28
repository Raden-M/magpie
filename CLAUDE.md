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
make docker           # build the container image
```

The backend serves <http://127.0.0.1:8787>. On loopback with no token, auth is disabled.

## Conventions

- The backend never makes outbound requests. Anything needing the live page belongs in the
  extension; anything that can live in the backend does, because shipping extension updates is
  slow and serving a backend is not.
- The extension sanitizes at capture time *and* the backend serves snapshots under a strict CSP.
  Both are load-bearing - do not drop one because the other exists.
- The design is mobile-first throughout: base CSS targets the phone, `min-width` queries layer up.
  Never a wide-first layout with `max-width` overrides.

## Gotchas

- `MAGPIE_DATA_DIR` defaults to `./data`, which is **CWD-relative** - running the binary from a
  different directory silently starts an empty collection.
- The HTTP API is **not versioned**, despite older docs claiming otherwise: routes are bare
  `/api/...` with no version segment and no compatibility check between the two halves.

## Reference

- `docs/architecture.md` - how the two halves fit together, and the capture flow.
- `docs/usage.md` - running, configuring, deploying, and the security posture.
- `docs/decisions/` - one decision record per file, `YYYY-MM-DD-slug.md`.
- `.claude/rules/` - instructions scoped to `backend/` and `extension/`, loaded on demand.
