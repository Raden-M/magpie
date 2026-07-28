# Permissive-compatible libraries for extraction

Date: 2026-07-23
Status: accepted

## Context

A copyleft dependency in the capture path would constrain the project's own license choice.

## Decision

Use `Readability.js` (Apache-2.0) and `DOMPurify` (Apache-2.0/MPL) for extraction and sanitizing.
Do not depend on AGPL SingleFile for core capture.

## Alternatives rejected

SingleFile: higher-fidelity capture, but AGPL and it needs a server-side headless browser, which
contradicts client-side capture.

## Consequences

Reader-view fidelity is lower than a faithful archiver's - already accepted when choosing the
reader-view snapshot.
