<h1 align="center">Magpie</h1>

<p align="center"><em>In search of shiny things.</em></p>

<p align="center">
  Highlight a sentence anywhere on the web and keep it - the words, the source,
  and a clean reader-view snapshot of the page around it - on a server you own.
</p>

<p align="center">
  <a href="#get-started">Get started</a> ·
  <a href="#why">Why</a> ·
  <a href="docs/usage.md">Docs</a> ·
  <a href="CONTRIBUTING.md">Contributing</a> ·
  <a href="LICENSE">MIT</a>
</p>

---

<p align="center">
  <video src="https://github.com/user-attachments/assets/b4af4cdb-2b7f-4231-be9a-591b0751ab30" controls muted loop playsinline width="100%"></video>
</p>

<p align="center"><em>Highlight, keep, and find it again - the whole loop in under a minute.</em></p>

## Get started

You'll need [Rust](https://rustup.rs) and [Node 23.6+](https://nodejs.org).

```sh
git clone https://github.com/Raden-M/magpie
cd magpie
make dev
```

Then load the extension into Firefox, once:

1. Copy `about:debugging#/runtime/this-firefox` into the address bar
2. **Load Temporary Add-on** → pick any file inside `extension/dist-firefox/`

**Done.** Select text on any page and a pill appears - hit **Keep**. Your collection is at
<http://127.0.0.1:8787>.

There's nothing to configure. The extension ships pointed at your own machine, and a local Magpie
needs no token, so there are no settings to fill in and no pairing step.

Already have somewhere to host things? The extension will point at any Magpie you run - see
[on your own server](#other-ways-to-run-it) below.

> Firefox drops a temporary add-on when it restarts, so you re-load it after a restart. Permanent,
> auto-updating installs arrive with the first signed release.

## Why

The internet quietly deletes itself. The paragraph that changed your mind is behind a paywall now,
or 404, or "archived" into a redirect to a homepage. Bookmarks save an address, not a thought;
read-later apps save an article you will not reread; notes apps save a quote with no way back to
where it came from.

Magpie saves the sentence **and** the page around it, on hardware you control, in a format that
outlives the site. Single-user, local-first, one binary and one folder of data - no account, no
cloud, no telemetry.

## Other ways to run it

<details>
<summary><b>Without Rust</b> - run the backend in Docker</summary>

```sh
make docker-up
```

Starts the backend in a container, builds the extension, generates a token and prints a connect
string. Load the extension as above, then click the **Magpie icon** in the Firefox toolbar and paste
that string into **Backend URL**. `make connect` prints it again if you lose it.

A container is reachable from the Docker network rather than just your machine, which is why this
path has a token and the from-source one doesn't.

</details>

<details>
<summary><b>Always running</b> - so it survives a reboot</summary>

`make dev` holds a terminal open. For a permanent local install, see
[`packaging/`](packaging/README.md) for a systemd user unit, or leave the Docker instance up.

</details>

<details>
<summary><b>On your own server</b> - point the extension at a backend you host</summary>

Nothing ties the extension to `127.0.0.1`. Run the backend on a VPS, a NAS, or whatever is already
humming in the cupboard, put it behind a reverse proxy that terminates TLS, and paste the connect
string it prints into the extension's **Backend URL**. Every browser you use then keeps into the
same collection, reachable from anywhere you are; you sign in to the web UI once per browser.

The extension asks for permission to reach the one address you name and nothing else - not every
site you visit - so this costs a single prompt when you save it.
[`docs/usage.md`](docs/usage.md#deploying-remotely) has the proxy config, the token, and an honest
account of the security posture.

</details>

<details>
<summary><b>Chromium</b> - works, but isn't the shipping target</summary>

```sh
make extension-chromium
```

Then `chrome://extensions` → **Developer mode** → **Load unpacked** → `extension/dist`. There's no
Chrome Web Store listing yet, and
[a decision record](docs/decisions/2026-08-05-chrome-web-store-deferred.md) explains why.

</details>

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

Send the pull request - no need to open an issue and wait first.
[`CONTRIBUTING.md`](CONTRIBUTING.md) covers the setup and the one thing that gets a change turned
down (scope). Security problems are the exception and have [their own process](SECURITY.md).

## License

[MIT](LICENSE). Third-party components keep their own licenses - see
[`THIRD-PARTY.md`](THIRD-PARTY.md).
