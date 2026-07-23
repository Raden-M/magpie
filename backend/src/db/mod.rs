//! Database layer: schema migrations, connection pool, and the typed query
//! functions each API module builds on.

pub mod highlights;
pub mod media;
pub mod models;
pub mod pages;
pub mod pool;
pub mod search;
pub mod snapshots;
pub mod stats;

pub use pool::Db;

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

    fn fresh() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrations().to_latest(&mut conn).unwrap();
        conn
    }

    #[test]
    fn upsert_page_is_idempotent_by_canonical_url() {
        use models::NewPage;
        let conn = fresh();
        let input = NewPage {
            canonical_url: "https://e.com/a".into(),
            original_url: "https://e.com/a?utm_source=x".into(),
            title: "First".into(),
            site_name: Some("E".into()),
            lang: Some("en".into()),
            favicon_path: None,
        };
        let first = pages::upsert_page(&conn, &input).unwrap();

        let mut second_input = input.clone();
        second_input.title = "Second".into();
        let second = pages::upsert_page(&conn, &second_input).unwrap();

        assert_eq!(first.id, second.id, "same canonical URL keeps one page");
        assert_eq!(second.title, "Second", "title updates on re-capture");
        assert_eq!(
            first.first_captured_at, second.first_captured_at,
            "first_captured_at is preserved"
        );

        let n: i64 = conn
            .query_row("SELECT count(*) FROM page", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1);
    }

    #[test]
    fn snapshot_versioning_reuses_or_bumps() {
        use models::{NewPage, NewSnapshot};
        let conn = fresh();
        let page = pages::upsert_page(
            &conn,
            &NewPage {
                canonical_url: "https://e.com/a".into(),
                original_url: "https://e.com/a".into(),
                title: "T".into(),
                site_name: None,
                lang: None,
                favicon_path: None,
            },
        )
        .unwrap();

        let path_for = |id: &str| format!("snapshots/{id}.html");
        let v1 = NewSnapshot {
            content_hash: "hash-a".into(),
            byline: None,
            excerpt: None,
            word_count: Some(400),
            partial: false,
        };
        let first = snapshots::store_or_reuse(&conn, &page.id, path_for, &v1).unwrap();
        assert!(first.created);
        assert_eq!(first.snapshot.version, 1);
        assert_eq!(first.snapshot.reading_time, Some(2), "400 words ~ 2 min");

        // Same hash -> reuse, no new version.
        let reused = snapshots::store_or_reuse(&conn, &page.id, path_for, &v1).unwrap();
        assert!(!reused.created);
        assert_eq!(reused.snapshot.id, first.snapshot.id);

        // Different hash -> new version.
        let v2 = NewSnapshot {
            content_hash: "hash-b".into(),
            ..v1.clone()
        };
        let bumped = snapshots::store_or_reuse(&conn, &page.id, path_for, &v2).unwrap();
        assert!(bumped.created);
        assert_eq!(bumped.snapshot.version, 2);
    }

    #[test]
    fn orphan_media_detected_after_snapshot_gone() {
        use models::{Media, NewPage, NewSnapshot};
        let conn = fresh();
        let page = pages::upsert_page(
            &conn,
            &NewPage {
                canonical_url: "https://e.com/a".into(),
                original_url: "https://e.com/a".into(),
                title: "T".into(),
                site_name: None,
                lang: None,
                favicon_path: None,
            },
        )
        .unwrap();
        let snap = snapshots::store_or_reuse(
            &conn,
            &page.id,
            |id| format!("snapshots/{id}.html"),
            &NewSnapshot {
                content_hash: "h".into(),
                byline: None,
                excerpt: None,
                word_count: None,
                partial: false,
            },
        )
        .unwrap()
        .snapshot;

        media::insert_media(
            &conn,
            &Media {
                hash: "abc".into(),
                mime: "image/png".into(),
                byte_size: 10,
                created_at: String::new(),
            },
        )
        .unwrap();
        snapshots::link_media(&conn, &snap.id, "abc").unwrap();

        assert!(
            media::orphan_hashes(&conn).unwrap().is_empty(),
            "referenced media not orphaned"
        );

        conn.execute("DELETE FROM page WHERE id = ?1", [&page.id])
            .unwrap();
        assert_eq!(
            media::orphan_hashes(&conn).unwrap(),
            vec!["abc".to_string()]
        );
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
            .query_row(
                "SELECT count(*) FROM highlight_fts WHERE highlight_fts MATCH 'brown'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(hits, 1, "insert trigger indexed the quote");

        conn.execute("DELETE FROM highlight WHERE id='h1'", [])
            .unwrap();
        let hits: i64 = conn
            .query_row(
                "SELECT count(*) FROM highlight_fts WHERE highlight_fts MATCH 'brown'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(hits, 0, "delete trigger removed the row");
    }
}
