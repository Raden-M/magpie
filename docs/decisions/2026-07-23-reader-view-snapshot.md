# Reader-view snapshot, not full-fidelity capture

Date: 2026-07-23
Status: accepted

## Context

The value of a kept page is the content worth re-reading, not its layout. Controlling the rendered
DOM also makes highlight anchoring dramatically easier.

## Decision

Store a cleaned **reader view** of each page, extracted with `Readability.js`. Not a pixel- or
layout-faithful copy.

## Alternatives rejected

Full-fidelity archival (SingleFile, ArchiveBox): heavier, layout-coupled, and it would leave
anchoring to fight the original page's DOM.

## Consequences

Exact visual reproduction is lost, and non-article pages (apps, forums) extract imperfectly -
mitigated by a selection-scoped fallback. Anyone needing a faithful visual archive should run a
full-fidelity archiver alongside Magpie.
