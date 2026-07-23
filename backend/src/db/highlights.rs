//! Highlight CRUD. User data (note, colour) is editable; anchoring selectors
//! are immutable once captured.

use rusqlite::{Connection, OptionalExtension, params};

use crate::db::models::{Highlight, NewHighlight};
use crate::util::{new_id, now_iso};

pub fn insert_highlight(conn: &Connection, input: &NewHighlight) -> rusqlite::Result<Highlight> {
    let id = new_id();
    let now = now_iso();
    conn.execute(
        "INSERT INTO highlight
           (id, page_id, snapshot_id, quote_exact, quote_prefix, quote_suffix,
            pos_start, pos_end, section_anchor, color, note, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12)",
        params![
            id,
            input.page_id,
            input.snapshot_id,
            input.quote_exact,
            input.quote_prefix,
            input.quote_suffix,
            input.pos_start,
            input.pos_end,
            input.section_anchor,
            input.color,
            input.note,
            now,
        ],
    )?;
    Ok(get_highlight(conn, &id)?.expect("highlight just inserted"))
}

pub fn get_highlight(conn: &Connection, id: &str) -> rusqlite::Result<Option<Highlight>> {
    conn.query_row("SELECT * FROM highlight WHERE id = ?1", params![id], Highlight::from_row)
        .optional()
}

/// Highlights for a page, ordered by document position where anchored, else by
/// creation time. Drives both the reader rail and the page JSON view.
pub fn highlights_for_page(conn: &Connection, page_id: &str) -> rusqlite::Result<Vec<Highlight>> {
    let mut stmt = conn.prepare(
        "SELECT * FROM highlight WHERE page_id = ?1
         ORDER BY (pos_start IS NULL), pos_start, created_at",
    )?;
    let rows = stmt.query_map(params![page_id], Highlight::from_row)?;
    rows.collect()
}

pub fn update_highlight(
    conn: &Connection,
    id: &str,
    note: Option<Option<&str>>,
    color: Option<&str>,
) -> rusqlite::Result<Option<Highlight>> {
    if get_highlight(conn, id)?.is_none() {
        return Ok(None);
    }
    if let Some(note) = note {
        conn.execute(
            "UPDATE highlight SET note = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, note, now_iso()],
        )?;
    }
    if let Some(color) = color {
        conn.execute(
            "UPDATE highlight SET color = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, color, now_iso()],
        )?;
    }
    get_highlight(conn, id)
}

pub fn delete_highlight(conn: &Connection, id: &str) -> rusqlite::Result<bool> {
    let affected = conn.execute("DELETE FROM highlight WHERE id = ?1", params![id])?;
    Ok(affected > 0)
}
