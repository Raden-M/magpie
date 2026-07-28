---
paths:
  - "extension/**"
---

# Extension

- `manifest.json` is the canonical **Chromium** manifest. The Firefox variant is generated from it
  by `build.mjs:toFirefox()` - single source of truth. Never hand-edit `dist/` or `dist-firefox/`.
- Tests run `node --test` directly against `.ts` files, relying on native type stripping. That
  needs **Node >= 23.6**; there is no `engines` field yet, so an older Node fails obscurely.
- Run `npx web-ext lint --source-dir=dist-firefox` before signing. It is the same linter AMO runs,
  so it turns a multi-day review round trip into a few seconds.
- Release builds are minified (`build.mjs`, `minify: !watch`). AMO requires source submission for
  minified code, and minification makes the four `innerHTML` lint warnings unreadable to a
  reviewer. Turning minify off is the intended fix.
- MV3 service workers are ephemeral. Keep no long-lived in-memory state in `src/background/`;
  persist to `chrome.storage`.
- The token lives in `chrome.storage.local`, never `sync`, so it is not uploaded to the browser
  vendor.
