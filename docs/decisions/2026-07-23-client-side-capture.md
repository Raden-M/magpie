# Capture and extraction happen client-side

Date: 2026-07-23
Status: accepted

## Context

Captures must work on pages behind logins and paywalls, on SPAs whose content only exists after
JavaScript runs, and on localhost or intranet hosts a server could never reach.

## Decision

The extension reads the selection, runs Readability and DOMPurify, collects images, and posts
finished content. The backend never re-fetches pages.

## Alternatives rejected

Server-side archival (Karakeep, Wallabag, `monolith`): a valid camp, better suited to bulk-archiving
public URLs, which is not the use case here. It would also require a headless browser and outbound
network access.

## Consequences

Extraction logic ships in the extension, so changing it carries store-review latency, and
re-processing an old capture server-side is impossible without revisiting the page. The backend
gains no SSRF surface and can run airgapped.
