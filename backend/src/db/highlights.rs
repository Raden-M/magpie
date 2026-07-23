//! Highlight CRUD. User data (note, colour) is editable; anchoring selectors
//! are immutable once captured.

use rusqlite::types::Value;
use rusqlite::{Connection, OptionalExtension, params, params_from_iter};

use crate::db::models::{Highlight, NewHighlight};
use crate::util::{new_id, now_iso};

pub struct HighlightListQuery {
    pub page_id: Option<String>,
    pub q: Option<String>,
    pub limit: usize,
    pub after: Option<(String, String)>,
}

/// One highlight plus its sort key for cursor building.
pub struct HighlightListRow {
    pub highlight: Highlight,
    pub cursor_value: String,
}

/// Cross-page highlight listing, newest first, keyset-paginated.
pub fn list_highlights(
    conn: &Connection,
    query: &HighlightListQuery,
) -> rusqlite::Result<Vec<HighlightListRow>> {
    let mut sql = String::from("SELECT * FROM highlight WHERE 1 = 1");
    let mut vals: Vec<Value> = Vec::new();

    if let Some(page_id) = &query.page_id {
        sql.push_str(" AND page_id = ?");
        vals.push(Value::Text(page_id.clone()));
    }
    if let Some(q) = query.q.as_ref().filter(|q| !q.trim().is_empty()) {
        sql.push_str(" AND (quote_exact LIKE ? OR note LIKE ?)");
        vals.push(Value::Text(format!("%{q}%")));
        vals.push(Value::Text(format!("%{q}%")));
    }
    if let Some((value, id)) = &query.after {
        sql.push_str(" AND (created_at < ? OR (created_at = ? AND id < ?))");
        vals.push(Value::Text(value.clone()));
        vals.push(Value::Text(value.clone()));
        vals.push(Value::Text(id.clone()));
    }
    sql.push_str(" ORDER BY created_at DESC, id DESC LIMIT ?");
    vals.push(Value::Integer(query.limit as i64));

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(vals), |row| {
        let highlight = Highlight::from_row(row)?;
        Ok(HighlightListRow { cursor_value: highlight.created_at.clone(), highlight })
    })?;
    rows.collect()
}

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
