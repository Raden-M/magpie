# Single-user and relocatable, with one bearer token

Date: 2026-07-23
Status: accepted

## Context

The requirement is a relocatable single-user backend hosted wherever the owner chooses - not
multi-device sync, not accounts.

## Decision

One user per instance. Auth is a single shared bearer token, compared in constant time. Disabled
when bound to loopback with no token; required on any non-loopback bind, which refuses to start
without one.

## Alternatives rejected

Accounts and sessions: unnecessary complexity for one user, and it would make the data directory
less portable.

## Consequences

No multi-device story and no per-device revocation. Browsers cannot send a bearer header on
navigation, so remote *human* access to the web UI must be protected at the reverse proxy; the
token guards the extension's API path.
