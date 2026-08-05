## What and why

<!-- What changes, and the reason for it. The diff says what; this should say why. -->

## How it was verified

<!-- What you ran, and what you saw. If capture behaviour changed, say which pages you tried. -->

- [ ] `make test`
- [ ] `cargo fmt` + `cargo clippy --all-targets -- -D warnings`
- [ ] `npm run typecheck`
- [ ] `make lint-firefox` (0 errors; the four known innerHTML warnings are expected)

## Notes

<!-- Anything a reviewer would otherwise have to work out: trade-offs, things deliberately
     left out, follow-ups. If this changes a hard-to-reverse decision, link the record in
     docs/decisions/. -->
