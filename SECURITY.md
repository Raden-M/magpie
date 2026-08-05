# Security

## Reporting a vulnerability

Everywhere else in this project, [just send the pull request](CONTRIBUTING.md). This is the one
exception, and only because of how git works: a public patch explains the hole to everyone the
moment it's pushed, while every installed copy is still running the old code.

So report it privately first, through
[GitHub's private vulnerability reporting](https://github.com/Raden-M/magpie/security/advisories/new).
If that isn't available to you, email **raden.dev@pm.me** with `magpie` in the subject.

**Fixes are very welcome.** Private reporting isn't a handover - once an advisory is open, GitHub
gives it a private fork you can push branches to and open a pull request against, visible only to
the two of us. Same review, same discussion, nothing public until the fix ships. If you'd rather
just attach a patch or a diff to the report, that works too.

Include what you found, how to reproduce it, and what an attacker gets. I'll acknowledge within a
week. Magpie is a spare-time project maintained by one person, so please don't expect a same-day
response - but a real vulnerability will be taken seriously and fixed properly rather than quickly.

Once a fix is released the advisory gets published with credit, unless you'd rather not be named.
Please give me a reasonable window to get there before disclosing publicly.

## What's in scope

Magpie is single-user and self-hosted, which shapes what counts. In scope:

- Anything that lets a **web page you visit** read from or write to your Magpie instance. The
  backend is a local HTTP service, so this is the interesting attack surface.
- Anything that gets **stored snapshot content to execute script** in the reader view, or escape the
  Content-Security-Policy the snapshot is served under.
- **Authentication bypass**: reaching a token-protected instance without the token, or forging a
  session cookie.
- **Path traversal** through media hashes, snapshot ids, or any other path component.
- Anything the **extension** leaks to the page it runs on - the token above all.

## What isn't

- **Anyone with access to the machine can read the collection.** The data directory is plain SQLite
  and HTML, deliberately: it's your data, on your disk, in a format you can open with other tools.
  There is no at-rest encryption and none is planned.
- **The token is a single shared secret** with no rotation, expiry, or rate limiting. That's the
  design for a single-user tool. Guessing a 32-byte random token is not a finding.
- **Running Magpie on a public interface with a weak token**, or without a reverse proxy doing TLS.
  The startup guard refuses a non-loopback bind with no token at all; beyond that, configuration is
  yours.
- **Denial of service against your own instance.** It's your machine.
- **A cross-origin `POST /api/captures` from a page you visit** to an instance with auth *off*. This
  is [documented as a known residual risk](docs/usage.md#security-posture): `multipart/form-data` is
  CORS-safelisted, so the write needs no preflight. The response stays hidden, so it's write-only
  nuisance, and setting a token closes it. A way to *read* the collection this way would very much
  be a finding.

## Supported versions

The latest release only. Magpie is pre-1.0 and migrations are forward-only, so the fix for a
security issue is to upgrade.
