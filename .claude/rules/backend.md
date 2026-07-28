---
paths:
  - "backend/**"
---

# Backend

- Migrations are **forward-only** and embedded with `include_str!` (`src/db/mod.rs`). Never edit
  an applied migration; add a new one. There is no down path, so a binary cannot be rolled back
  across a schema change.
- SQLite is compiled in via `rusqlite`'s `bundled` feature. That is what makes the single-binary
  and distroless story work, and it means cross-compiling needs a **C toolchain** - plain
  `cargo build --target` will not do it.
- WAL mode (`src/db/pool.rs`) means the data directory holds three files, not one. They move
  together, and the directory must be writable before startup or the WAL cannot be created.
- SQLite on NFS or SMB is unsafe - locking is unreliable enough to corrupt a WAL database. The
  data directory must be local storage.
- Full-text search is FTS5 virtual tables written as plain SQL in the migration; no extra crate
  feature is involved.
- The web UI is server-rendered with `maud`. Assets in `src/assets/` are served from
  `src/web/assets.rs` - adding one means adding its route.
