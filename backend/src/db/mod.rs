//! Database layer: schema migrations, connection pool, and the typed query
//! functions each API module builds on.

use rusqlite_migration::{M, Migrations};

/// The forward-only, embedded migration set. Applied idempotently at startup and
/// tracked via SQLite's `PRAGMA user_version`, so re-running never clobbers data.
pub fn migrations() -> Migrations<'static> {
    Migrations::new(vec![M::up(include_str!("../../migrations/0001_init.sql"))])
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn migrations_are_valid() {
        migrations().validate().expect("migration set is valid");
    }

    #[test]
    fn migrations_apply_to_fresh_db() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrations().to_latest(&mut conn).expect("apply migrations");

        // Every expected table and virtual table exists.
        let count: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master
                 WHERE type='table' AND name IN
                   ('page','snapshot','highlight','media','capture_log','highlight_fts','snapshot_fts')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 7, "all core tables created");
    }

    #[test]
    fn highlight_fts_stays_in_sync() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations().to_latest(&mut conn).unwrap();

        conn.execute(
            "INSERT INTO page (id, canonical_url, original_url, title, first_captured_at, last_captured_at)
             VALUES ('p1','https://e.com/a','https://e.com/a','T','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO highlight (id, page_id, quote_exact, created_at, updated_at)
             VALUES ('h1','p1','the quick brown fox','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
            [],
        )
        .unwrap();

        let hits: i64 = conn
            .query_row("SELECT count(*) FROM highlight_fts WHERE highlight_fts MATCH 'brown'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(hits, 1, "insert trigger indexed the quote");

        conn.execute("DELETE FROM highlight WHERE id='h1'", []).unwrap();
        let hits: i64 = conn
            .query_row("SELECT count(*) FROM highlight_fts WHERE highlight_fts MATCH 'brown'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(hits, 0, "delete trigger removed the row");
    }
}
