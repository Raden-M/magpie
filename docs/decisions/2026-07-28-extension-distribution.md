# Self-hosted, unlisted-signed extension distribution

Date: 2026-07-28
Status: accepted

## Context

Release Firefox refuses unsigned extensions, so `about:debugging` installs vanish on restart -
the extension cannot stay installed. A store listing is slow to change and gated on review.

## Decision

Sign the Firefox build through AMO on the **unlisted** channel and host the resulting `.xpi` on
GitHub Releases, with an update manifest so it auto-updates. No public listing. Chromium
distribution is deliberately deferred.

## Alternatives rejected

A listed AMO or Chrome Web Store release: better install UX, but it introduces a review queue that
cannot be hot-fixed, and it must not happen before the permission model is settled - adding a
required host permission after publication disables the extension for every installed user until
each re-accepts it.

## Consequences

Firefox is fully solved; Chromium users are left with "load unpacked", with a developer-mode
warning and no auto-update, until a Chrome Web Store listing happens. Declaring data collection to
AMO also forces `strict_min_version` from 115 to 140, dropping ESR 115 and ESR 128 users.
