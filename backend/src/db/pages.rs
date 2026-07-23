//! Page CRUD: upsert by canonical URL (the grouping key), fetch, list, delete.

use rusqlite::types::Value;
use rusqlite::{Connection, OptionalExtension, params, params_from_iter};

use crate::db::models::{NewPage, Page};
use crate::util::{new_id, now_iso};

#[derive(Debug, Clone, Copy)]
pub enum PageSort {
    Recent,
    Title,
}

/// One row of the collection list: a page plus its highlight count and a
/// one-line preview (its most recent quote).
pub struct PageListRow {
    pub page: Page,
    pub highlight_count: i64,
    pub top_highlight: Option<String>,
    /// The value of this row's sort key, used to build the next cursor.
    pub cursor_value: String,
}

pub struct PageListQuery {
    pub q: Option<String>,
    pub sort: PageSort,
    pub limit: usize,
    pub after: Option<(String, String)>,
}

/// List pages for the collection, newest first by default, keyset-paginated.
pub fn list_pages(conn: &Connection, query: &PageListQuery) -> rusqlite::Result<Vec<PageListRow>> {
    let mut sql = String::from(
        "SELECT p.id, p.canonical_url, p.original_url, p.title, p.site_name, p.lang,
                p.favicon_path, p.first_captured_at, p.last_captured_at,
                (SELECT count(*) FROM highlight h WHERE h.page_id = p.id) AS hl_count,
                (SELECT h.quote_exact FROM highlight h WHERE h.page_id = p.id
                 ORDER BY h.created_at DESC LIMIT 1) AS top_hl
         FROM page p WHERE 1 = 1",
    );
    let mut vals: Vec<Value> = Vec::new();

    if let Some(q) = query.q.as_ref().filter(|q| !q.trim().is_empty()) {
        sql.push_str(" AND p.title LIKE ?");
        vals.push(Value::Text(format!("%{q}%")));
    }

    if let Some((value, id)) = &query.after {
        match query.sort {
            PageSort::Recent => {
                sql.push_str(" AND (p.last_captured_at < ? OR (p.last_captured_at = ? AND p.id < ?))")
            }
            PageSort::Title => {
                sql.push_str(" AND (p.title > ? OR (p.title = ? AND p.id > ?))")
            }
        }
        vals.push(Value::Text(value.clone()));
        vals.push(Value::Text(value.clone()));
        vals.push(Value::Text(id.clone()));
    }

    match query.sort {
        PageSort::Recent => sql.push_str(" ORDER BY p.last_captured_at DESC, p.id DESC"),
        PageSort::Title => sql.push_str(" ORDER BY p.title ASC, p.id ASC"),
    }
    sql.push_str(" LIMIT ?");
    vals.push(Value::Integer(query.limit as i64));

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(vals), |row| {
        let page = Page::from_row(row)?;
        let cursor_value = match query.sort {
            PageSort::Recent => page.last_captured_at.clone(),
            PageSort::Title => page.title.clone(),
        };
        Ok(PageListRow {
            highlight_count: row.get("hl_count")?,
            top_highlight: row.get("top_hl")?,
            cursor_value,
            page,
        })
    })?;
    rows.collect()
}

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
