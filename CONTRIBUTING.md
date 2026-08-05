# Contributing

Thanks for looking. **Send the pull request** - you don't need to ask first, and there's no need to
open an issue and wait for a reply before writing code.

The one thing worth reading before you start is [Scope](#scope) at the bottom. Magpie is
deliberately narrow, and that's the only reason a well-built change gets turned down.

## Getting set up

You need **Rust** (stable) and **Node >= 23.6**. The Node floor is real: the tests run TypeScript
directly through native type stripping, and an older Node fails with a confusing syntax error.
`make` checks both before doing anything.

```sh
make dev            # build the Chromium extension, then serve http://127.0.0.1:8787
make dev-firefox    # same, but the Firefox build
make test           # backend and extension suites
make lint-firefox   # Mozilla's own add-on linter, the one AMO runs
```

There is no configuration to do. On loopback with no token, auth is off and the collection is at
<http://127.0.0.1:8787>.

## Before you open a pull request

```sh
make test
cd backend && cargo fmt && cargo clippy --all-targets -- -D warnings
cd extension && npm run typecheck
make lint-firefox
```

CI runs exactly this. `make lint-firefox` should report **0 errors**; it reports four
`UNSAFE_VAR_ASSIGNMENT` warnings, three from vendored Readability and DOMPurify and one from
`sanitize.ts` where the DOMPurify call sits directly above it. Those four are expected - a fifth is
not.

## What the code expects of you

The conventions that actually get changes sent back:

- **The backend never makes outbound requests.** Anything needing the live page belongs in the
  extension; anything that can live in the backend does, because shipping extension updates is slow
  and serving a backend is not.
- **Sanitize at capture time *and* serve under a strict CSP.** Both are load-bearing. Don't drop one
  because the other exists.
- **Mobile-first CSS.** Base styles target the phone and `min-width` queries layer up. Never a
  wide-first layout with `max-width` overrides.
- **Migrations are forward-only** and embedded with `include_str!`. Never edit an applied migration;
  add a new one. There is no down path.
- **`manifest.json` is the canonical Chromium manifest.** The Firefox variant is derived from it by
  `build.mjs:toFirefox()`. Never hand-edit `dist/` or `dist-firefox/`.
- **Comments explain why, not what.** Self-documenting code first; a comment earns its place by
  capturing something the code can't say on its own.

Tests live beside what they test: Rust unit tests in-module, backend integration tests in
`backend/tests/capture_flow.rs` (which drives the real router in-process), and extension tests in
`extension/tests/`. Add the test that would have caught the bug.

## Commits and pull requests

Commit messages are conventional-commit style - `feat(backend):`, `fix(extension):`, `docs:` - with
a body explaining *why*, not a restatement of the diff. Keep a pull request to one coherent change.

If a change is awkward enough that you had to think hard about the trade-off, record it as a
decision record in `docs/decisions/` using [the template](docs/decisions/TEMPLATE.md). Name it
`YYYY-MM-DD-slug.md` and repeat that date inside. Never rename an existing record - supersede it.

## Reporting bugs

A pull request with a failing test, or a fix, is more welcome than a report - but a plain issue is
fine too. Include what you ran, what happened, and what you expected. For anything involving
capture, the page URL matters enormously: reader extraction and anchoring are where the interesting
failures live, and they're site-specific.

**Security problems are the one exception** - those go through [`SECURITY.md`](SECURITY.md), because
a public patch describes the hole to everyone before anyone can upgrade.

## Scope

Magpie is single-user, local-first, and stays that way. Multi-user, accounts, sharing, sync between
devices, and built-in backups are explicit non-goals - not "not yet", but design decisions that the
data model and auth deliberately don't accommodate. Features that need any of them won't be merged,
however well built.
