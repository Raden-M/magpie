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
  <a href="LICENSE">MIT</a>
</p>

---

<!-- Replace the src below with the URL GitHub gives you when you drag the
     recording into the README editor (it becomes a github.com/user-attachments
     link), or commit the file to docs/media/ and keep this relative path. -->
<video src="docs/media/magpie-demo.mp4" controls muted loop playsinline width="100%"></video>

> Highlight, keep, and find it again - the whole loop in under a minute.

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

```sh
make dev
```

That builds the extension and starts the backend on <http://127.0.0.1:8787>. On loopback with no
token, auth is off - open that URL and the collection is yours.

Then load the extension: in a Chromium browser, `chrome://extensions` → **Developer mode** → **Load
unpacked** → pick `extension/dist`. Set the backend URL in the options page, and highlight
something.

<details>
<summary>Firefox, release builds, and the rest of <code>make</code></summary>

```sh
make dev-firefox    # build the Firefox extension (extension/dist-firefox), then run the backend
make build          # release binary + extension
make test           # backend and extension test suites
make docker         # build the release container image
```

</details>

For deploying to a remote host, the security posture, configuration, and backups, see
[`docs/usage.md`](docs/usage.md).

## How it works

Two codebases with a versioned HTTP API between them:

| | |
| --- | --- |
| **`backend/`** | A single static Rust binary (`axum` + SQLite/FTS5). Stores, searches, and serves the collection and the reader view. It never re-fetches pages, so it needs no outbound network at all. |
| **`extension/`** | A TypeScript Manifest V3 extension (Chromium and Firefox). Does everything that needs the live page: reading the selection, extracting the reader view (`Readability.js` + `DOMPurify`), collecting images, and posting finished captures to the backend. |

The split is the point: only the browser can see the page you are reading, and only your server
should see what you kept.

## License

[MIT](LICENSE).
