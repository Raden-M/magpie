# Rust backend, single static binary

Date: 2026-07-23
Status: accepted

## Context

The backend must be relocatable and hostable anywhere - a laptop, a Pi, a VPS - with no runtime to
install. At single-user scale, raw performance is irrelevant; deployment ergonomics are not.

## Decision

Backend in Rust (`axum`), shipped as one static binary plus a container image.

## Alternatives rejected

An all-TypeScript backend (Hono + `better-sqlite3`) would share one language with the extension, at
the cost of the single-binary deploy. Revisit only if the TS/Rust split becomes a real maintenance
burden.

## Consequences

Anchoring and extraction logic lives in TypeScript and is not shared with Rust. Because the reader
view means the backend stores selector strings as opaque data and never runs matching, there is
effectively nothing duplicated.
