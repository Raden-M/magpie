---
paths:
  - "extension/**"
---

# Extension

- `manifest.json` is the canonical **Chromium** manifest. The Firefox variant is generated from it
  by `build.mjs:toFirefox()` - single source of truth. Never hand-edit `dist/` or `dist-firefox/`.
- Tests run `node --test` directly against `.ts` files, relying on native type stripping (**Node >=
  23.6**, enforced by `engines` and `make check-node`). Node does **not** rewrite `.js` specifiers
  to `.ts`, so a module a test imports must not import anything with a `.js` extension - that is why
  the pure pairing helpers live in `shared/pairing.ts`, away from `shared/config.ts` and its `ext`
  import.
- Lint with `make lint-firefox`, which passes `--self-hosted`. Without that flag `update_url` is
  reported as an **error**: addons-linter can't tell an unlisted build from a listed one, and listed
  add-ons may not self-update.
- Expect **0 errors and 4 `UNSAFE_VAR_ASSIGNMENT` warnings**. Three are inside vendored Readability
  and DOMPurify; the fourth is `sanitize.ts` assigning DOMPurify's own output. A fifth means
  something new. Builds are never minified, so those line numbers point at readable code.
- Only the **default local backend** (`127.0.0.1:8787`, `localhost:8787`) is a required
  `host_permissions`, so an install pointed at a local Magpie works with no configuration at all.
  Every other backend is an **optional** permission requested at runtime for that one origin
  (`options.ts`). Never *widen* the required list after publication: a new required host permission
  disables the extension for every installed user until each re-accepts it.
- MV3 service workers are ephemeral. Keep no long-lived in-memory state in `src/background/`;
  persist to `chrome.storage`.
- The worker **cannot read the active tab's URL**: that needs the `tabs` permission, whose install
  warning ("read your browsing history") is not worth a menu title. So the *page* announces itself
  (`PAGE_STATE` in `shared/messages.ts`) whenever it becomes visible, and the worker only reacts.
  Anything in `src/background/` that wants to know where the user is has to go through that.
- The token lives in `chrome.storage.local`, never `sync`, so it is not uploaded to the browser
  vendor.
