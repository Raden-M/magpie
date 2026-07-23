//! Page CRUD: upsert by canonical URL (the grouping key), fetch, and delete.

use rusqlite::{Connection, OptionalExtension, params};

use crate::db::models::{NewPage, Page};
use crate::util::{new_id, now_iso};

/// Insert a page, or update the existing one that shares this canonical URL.
/// `first_captured_at` is preserved on update; `last_captured_at` always bumps.
pub fn upsert_page(conn: &Connection, input: &NewPage) -> rusqlite::Result<Page> {
    let now = now_iso();
    let id = new_id();
    conn.query_row(
        "INSERT INTO page
           (id, canonical_url, original_url, title, site_name, lang, favicon_path,
            first_captured_at, last_captured_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)
         ON CONFLICT(canonical_url) DO UPDATE SET
           original_url     = excluded.original_url,
           title            = excluded.title,
           site_name        = excluded.site_name,
           lang             = excluded.lang,
           favicon_path     = coalesce(excluded.favicon_path, page.favicon_path),
           last_captured_at = excluded.last_captured_at
         RETURNING *",
        params![
            id,
            input.canonical_url,
            input.original_url,
            input.title,
            input.site_name,
            input.lang,
            input.favicon_path,
            now,
        ],
        Page::from_row,
    )
}

pub fn get_page(conn: &Connection, id: &str) -> rusqlite::Result<Option<Page>> {
    conn.query_row("SELECT * FROM page WHERE id = ?1", params![id], Page::from_row)
        .optional()
}

pub fn touch_last_captured(conn: &Connection, id: &str, when: &str) -> rusqlite::Result<()> {
    conn.execute("UPDATE page SET last_captured_at = ?2 WHERE id = ?1", params![id, when])?;
    Ok(())
}

/// Number of highlights currently attached to a page.
pub fn highlight_count(conn: &Connection, page_id: &str) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT count(*) FROM highlight WHERE page_id = ?1",
        params![page_id],
        |r| r.get(0),
    )
}

/// Delete a page (cascading to snapshots and highlights). Returns whether a row
/// was removed. Orphan-media GC is a separate step the caller runs after.
pub fn delete_page(conn: &Connection, id: &str) -> rusqlite::Result<bool> {
    let affected = conn.execute("DELETE FROM page WHERE id = ?1", params![id])?;
    Ok(affected > 0)
}
