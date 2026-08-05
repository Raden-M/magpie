<h1 align="center">Magpie</h1>

<p align="center"><em>In search of shiny things.</em></p>

<p align="center">
  Highlight a sentence anywhere on the web and keep it - the words, the source,
  and a clean reader-view snapshot of the page around it - on a server you own.
</p>

<p align="center">
  <a href="#quick-start">Quick start</a> ·
  <a href="#how-it-works">How it works</a> ·
  <a href="docs/usage.md">Full docs</a> ·
  <a href="CONTRIBUTING.md">Contributing</a> ·
  <a href="LICENSE">MIT</a>
</p>

---

## Why

The internet quietly deletes itself. The paragraph that changed your mind is behind a paywall now,
or 404, or "archived" into a redirect to a homepage. Bookmarks save an address, not a thought;
read-later apps save an article you will not reread; notes apps save a quote with no way back to
where it came from.

Magpie saves the sentence **and** the page around it, on hardware you control, in a format that
outlives the site.

> A magpie's blue-green-purple is *structural colour*: the feather holds no blue pigment. The colour
> is an interference effect that appears only when light catches the structure at the right angle,
> against a black base. The interface works the same way: everything is quiet and grey, and the only
> colour in the room is your highlights.

## What it does

- **Capture.** Select text on any page and a pill appears: pick a colour, add a note, hit **Keep**.
  Or use `Ctrl/Cmd+Shift+K` and skip the pill. You never leave the page.
- **Keep.** The article is saved with it, as clean readable text rather than a screenshot. Your
  highlight is still there after the page isn't.
- **Read.** Each highlight comes back with the passage around it, plus a side rail that follows you
  down the page and a link straight to the words on the live site. The full article is one click
  behind it.
- **Find.** Search every highlight, note, and saved article at once. It's fast, and it never leaves
  your machine.

Single-user, local-first, one binary and one folder of data. Runs on your laptop or on a box you
own - the same artifact either way, no account, no cloud, no telemetry.

## Quick start

Two steps: run the backend, then install the extension.

### 1. Run the backend

**With Docker** - no Rust toolchain needed:

```sh
make docker-up
```

Generates a token, starts Magpie on <http://127.0.0.1:8787>, and prints a connect string. Re-running
it keeps the token you already have, and `make connect` prints that string again whenever you need
it.

**From source** - if you have Rust:

```sh
make backend
```

Serves <http://127.0.0.1:8787>. On loopback with no token auth is off, so that URL is your
collection straight away - no token, no pairing string, nothing to paste.

### 2. Build and install the extension

Until the first signed release lands you build this yourself, so **Node >= 23.6 is required either
way** - including the Docker route. `make dev` does the backend and the extension together if you
want both from source.

```sh
make extension-firefox      # → extension/dist-firefox
```

In Firefox: `about:debugging` → **This Firefox** → **Load Temporary Add-on** → pick any file in
`extension/dist-firefox`.

Then **click the Magpie icon in the toolbar** - that opens its settings. Paste the connect string
into **Backend URL** (it fills the token too), or just `http://127.0.0.1:8787` if you're running
without a token, and hit **Save**. Saving asks permission to reach that one address, which is how
Magpie works with a backend hosted anywhere.

Highlight something.

### Notes

A temporary add-on is removed when Firefox restarts. Signed `.xpi` builds that install permanently
and auto-update will be published to [Releases](https://github.com/Raden-M/magpie/releases).

**Chromium** works but isn't a shipping target yet: `make extension`, then `chrome://extensions` →
**Developer mode** → **Load unpacked** → `extension/dist`. See
[the decision record](docs/decisions/2026-08-05-chrome-web-store-deferred.md).

To keep Magpie running rather than holding a terminal open, see
[`packaging/`](packaging/README.md) for the systemd user unit, or leave the Docker instance up.

## How it works

Two codebases with an HTTP API between them:

| | |
| --- | --- |
| **`backend/`** | A single static Rust binary (`axum` + SQLite/FTS5). Stores, searches, and serves the collection and the reader view. It never re-fetches pages, so it needs no outbound network at all. |
| **`extension/`** | A TypeScript Manifest V3 extension (Firefox and Chromium). Does everything that needs the live page: reading the selection, extracting the reader view (`Readability.js` + `DOMPurify`), collecting images, and posting finished captures to the backend. |

The split is the point: only the browser can see the page you are reading, and only your server
should see what you kept.

More in [`docs/architecture.md`](docs/architecture.md), and the reasoning behind the awkward calls
in [`docs/decisions/`](docs/decisions/).

## Contributing

Send the pull request - no need to open an issue and wait first. [`CONTRIBUTING.md`](CONTRIBUTING.md)
covers the setup and the one thing that gets a change turned down (scope). Security problems are the
exception and have [their own process](SECURITY.md).

## License

[MIT](LICENSE). Third-party components keep their own licenses - see
[`THIRD-PARTY.md`](THIRD-PARTY.md).
