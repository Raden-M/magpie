# W3C TextQuoteSelector as the primary anchor

Date: 2026-07-23
Status: accepted

## Context

A highlight must survive the page text shifting underneath it. Offsets and XPath break on any edit.

## Decision

Store multiple selectors per highlight: TextQuoteSelector (quote plus ~32 characters of
prefix/suffix) as primary, TextPositionSelector as a hint, and a `section_anchor` block id.
Re-locate via position fast-path, then exact quote, then context-fuzzy, then quote-fuzzy at about
5% Levenshtein - the Hypothesis-style ladder.

## Alternatives rejected

Reimplementing the full Hypothesis stack. Unnecessary here: rendering against a frozen snapshot
means the position fast-path essentially always hits.

## Consequences

Fuzzy matching fails on heavily rewritten text. Acceptable - the snapshot is the fallback, and
orphaned highlights still show their words. Live-page re-anchoring is a bonus, not load-bearing.
