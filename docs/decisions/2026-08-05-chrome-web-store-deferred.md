# Chrome Web Store listing deferred

Date: 2026-08-05
Status: accepted

## Context

Firefox distribution is solved: AMO signs an unlisted `.xpi`, GitHub Releases hosts it, and
`update_url` keeps installed copies current, with no review queue in the path. Chromium has no
equivalent - self-hosted `.crx` installs are blocked outside enterprise policy - so the only route
to a permanently installed Chromium extension is a Chrome Web Store listing.

That listing is a bigger commitment than it looks, for reasons that mostly aren't technical:

- **A review queue you don't control** sits between a fix and its users. Firefox's unlisted channel
  has none, so the two halves of the project would ship at very different speeds.
- **Permissions can't be widened after publication.** Adding a *required* host permission disables
  the extension for every installed user until each re-accepts it. This is now much less of a trap
  than it was - the backend origin moved to `optional_host_permissions` requested at runtime, which
  it had to for self-hosting to work at all - but it means the permission model must be settled
  before listing, not after.
- **The 2026 policy updates took effect on 1 August 2026**, tightening the Limited Use policy so
  collected data must be "strictly necessary to the extension's disclosed single purpose", and
  requiring prominent disclosure of all collection regardless of relevance. Magpie transmits page
  content to a server, so it has a real disclosure to make and a privacy policy to host, even though
  the server is the user's own.
- **Broad host permissions trigger in-depth review.** A content script matching all URLs, plus host
  access to an arbitrary self-hosted backend, is the combination reviewers look hardest at.

None of this is prohibitive. It is simply work with an unpredictable schedule, and none of it blocks
the thing that actually matters right now: being installable at all.

## Decision

Ship Firefox. Defer the Chrome Web Store listing.

Chromium stays supported as a build target - `npm run build` produces a working unpacked extension,
CI builds it on every change, and releases carry a `.zip` of it - so Chromium users can run Magpie
today via **Load unpacked**, accepting the developer-mode warning and no auto-update.

Revisit when any of these is true:

- Someone other than the author actually wants it on Chromium.
- The permission model has been stable across a few releases.
- There is a privacy policy to point at and appetite for a review queue.

The recommended shape when that happens: CI uploads a **draft** and publishing stays a manual click,
so a bad tag can never auto-ship into a store that takes days to correct.

## Alternatives rejected

**List on the Chrome Web Store now.** Better install UX for Chromium users, and the $5 registration
is trivial. Rejected because it introduces a review queue before there is any evidence of a Chromium
user, and it would freeze the permission model at the exact moment the self-hosting design is
newest.

**Drop Chromium support entirely.** Simpler - one manifest, one target, no `toFirefox()` derivation.
Rejected because the divergence is genuinely small (a derived manifest and an event page instead of
a service worker), CI keeps it honest, and abandoning it would be hard to undo once the code drifts.

**Self-host a `.crx` with an update manifest**, as Firefox does. Rejected because Chromium blocks
it outside enterprise policy - it isn't an option, only an appealing-looking dead end.

## Consequences

Firefox users get a signed, auto-updating add-on. Chromium users get a working extension with a
worse install story: a developer-mode warning, and manual updates by re-downloading the zip. That
asymmetry is visible in the README, and it will be the most common complaint from anyone who finds
this project on Chromium.

The project keeps the ability to ship a hot-fix within minutes, on the browser it actually targets.
Nothing here is hard to reverse: the listing can happen whenever the trade turns, and the artifact
it needs is already built on every tag.
