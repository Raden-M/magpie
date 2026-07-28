# Group captures by canonical URL

Date: 2026-07-23
Status: accepted

## Context

Multiple highlights from one page must collect under that page, and page identity has to survive
tracking parameters and fragments.

## Decision

The Page, keyed by canonical URL, is the grouping unit; highlights hang off it. Canonicalization
prefers `<link rel="canonical">`, else normalizes the URL - strip tracking params and fragments,
drop default ports, sort the query, de-slash.

## Alternatives rejected

Grouping by exact URL: splits one article across several pages the moment a tracking parameter
differs.

## Consequences

Query-varying pages such as search results and paginated threads may over- or under-group.
Canonicalization is kept simple; revisit the tracking-parameter list if it misbehaves.
