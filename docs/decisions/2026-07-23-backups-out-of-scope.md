# Backups are out of scope

Date: 2026-07-23
Status: accepted

## Context

Backup and restore is a substantial feature, and the data is already structurally easy to copy.

## Decision

No built-in backup or restore. All data lives in one directory, backed up with whatever tool the
operator already uses.

## Alternatives rejected

Building scheduled backups into the binary: scope the project does not need, and it would duplicate
tools users already run.

## Consequences

Users must set up their own backups, and a naive copy of a live WAL database can be torn - hence
the `VACUUM INTO` guidance in `docs/usage.md`.
