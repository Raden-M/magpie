# Magpie

**A personal, self-hosted tool for capturing highlighted text from the web.**

You highlight a passage in your browser; Magpie keeps the text, the source, and a clean
reader-view snapshot of the page - so your catch survives link rot - and groups everything under
the page it came from. Single-user, local-first, one binary and one folder of data.

> A magpie's blue-green-purple is *structural colour*: the feather holds no blue pigment; the
> colour is an interference effect that only appears when light catches the structure at the right
> angle, against a black base. That is the design thesis - the interface is quiet monochrome, and
> iridescence is spent in exactly one place: your highlights.

## What it does

- **Capture** a text selection from any page in one action, with its source.
- **Keep** a reader-view snapshot (content, cleanly presented - not a pixel screenshot) that
  outlives the original page.
- **Read** it back as snippets - each highlight in its surrounding passage - with a scroll-spy
  "shiny things" rail, a one-click deep link to the exact words on the live page, and the full
  snapshot kept behind it as a backup.
- **Find** anything you kept with full-text search across highlights, notes, and snapshot content.

Everything runs locally, or on a remote host you choose - the same artifact either way.

## Architecture

Two codebases with a versioned HTTP API between them:

- **`backend/`** - a single static Rust binary (`axum` + SQLite/FTS5). Stores, searches, and serves
  the collection and reader view. Never re-fetches pages; needs no outbound network.
- **`extension/`** - a TypeScript Manifest V3 browser extension. Does everything that needs the live
  page: reading the selection, extracting the reader view (`Readability.js` + `DOMPurify`),
  collecting images, and posting finished captures to the backend.

## Quick start

### Backend

```sh
cd backend
cargo run
# serves http://127.0.0.1:8787 (auth disabled on loopback)
```

Open <http://127.0.0.1:8787> for the collection.

### Extension

```sh
cd extension
npm install
npm run build       # → extension/dist
```

Load `extension/dist` as an unpacked extension in a Chromium browser, then set the backend URL in
the options page.

See [`docs/usage.md`](docs/usage.md) for full run, deploy, and security notes.

## License

[MIT](LICENSE).
