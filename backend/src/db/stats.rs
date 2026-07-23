//! Aggregate counts for the web UI footer and `GET /api/stats`.

use rusqlite::Connection;

#[derive(Debug, Clone)]
pub struct Stats {
    pub pages: i64,
    pub highlights: i64,
    pub media: i64,
    pub media_bytes: i64,
}

pub fn stats(conn: &Connection) -> rusqlite::Result<Stats> {
    let pages = conn.query_row("SELECT count(*) FROM page", [], |r| r.get(0))?;
    let highlights = conn.query_row("SELECT count(*) FROM highlight", [], |r| r.get(0))?;
    let media = conn.query_row("SELECT count(*) FROM media", [], |r| r.get(0))?;
    let media_bytes = conn.query_row("SELECT coalesce(sum(byte_size), 0) FROM media", [], |r| {
        r.get(0)
    })?;
    Ok(Stats {
        pages,
        highlights,
        media,
        media_bytes,
    })
}
