// Cross-browser extension API handle.
//
// Firefox exposes promise-based APIs under `browser.*`, while its `chrome.*`
// alias is callback-based; Chromium exposes promise-based `chrome.*`. Using
// `browser` when present (Firefox) and `chrome` otherwise (Chromium) gives us
// promise-returning APIs on both, so `await ext.runtime.sendMessage(...)` works
// everywhere. Types come from `chrome`, which the two are compatible with.

export const ext: typeof chrome = (globalThis as unknown as { browser?: typeof chrome }).browser
  ?? chrome;
