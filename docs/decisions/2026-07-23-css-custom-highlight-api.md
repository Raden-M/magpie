# CSS Custom Highlight API for rendering highlights

Date: 2026-07-23
Status: accepted

## Context

Painting highlights by wrapping ranges in spans mutates the DOM, forces reflow, and can break the
host page's own JavaScript.

## Decision

Render highlights on live pages with the CSS Custom Highlight API. In the served reader view, use
baked `<mark>` spans for the MVP, with the Highlight API as progressive enhancement.

## Alternatives rejected

Span-wrapping on live pages: roughly 5x slower, mutates the DOM, and is awkward to clean up.

## Consequences

`highlightsFromPoint` hit-testing is not available everywhere yet, so interactions needing it
require a fallback. Baked spans keep the served reader view working without JavaScript.
