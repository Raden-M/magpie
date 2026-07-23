//! Snapshot storage and the versioning rule: reuse the latest snapshot when the
//! content is unchanged, otherwise store a new version (docs/03-data-model.md).

use rusqlite::{Connection, OptionalExtension, params};

use crate::db::models::{NewSnapshot, Snapshot};
use crate::util::{new_id, now_iso, reading_minutes};

/// Outcome of storing a capture's content against a page.
pub struct SnapshotDecision {
    pub snapshot: Snapshot,
    /// True when a new version row was created (the caller must write the HTML
    /// file and record media references); false when the latest was reused.
    pub created: bool,
}

pub fn latest_snapshot(conn: &Connection, page_id: &str) -> rusqlite::Result<Option<Snapshot>> {
    conn.query_row(
        "SELECT * FROM snapshot WHERE page_id = ?1 ORDER BY version DESC LIMIT 1",
        params![page_id],
        Snapshot::from_row,
    )
    .optional()
}

pub fn get_snapshot(conn: &Connection, id: &str) -> rusqlite::Result<Option<Snapshot>> {
    conn.query_row("SELECT * FROM snapshot WHERE id = ?1", params![id], Snapshot::from_row)
        .optional()
}

pub fn snapshot_by_version(
    conn: &Connection,
    page_id: &str,
    version: i64,
) -> rusqlite::Result<Option<Snapshot>> {
    conn.query_row(
        "SELECT * FROM snapshot WHERE page_id = ?1 AND version = ?2",
        params![page_id, version],
        Snapshot::from_row,
    )
    .optional()
}

/// Apply the versioning rule. If the newest snapshot's content hash matches,
/// reuse it. Otherwise create the next version. The returned snapshot's
/// `content_path` is where the caller must write the HTML when `created`.
pub fn store_or_reuse(
    conn: &Connection,
    page_id: &str,
    content_path_for: impl Fn(&str) -> String,
    input: &NewSnapshot,
) -> rusqlite::Result<SnapshotDecision> {
    if let Some(latest) = latest_snapshot(conn, page_id)? {
        if latest.content_hash == input.content_hash {
            return Ok(SnapshotDecision { snapshot: latest, created: false });
        }
    }

    let previous_version: Option<i64> = conn.query_row(
        "SELECT max(version) FROM snapshot WHERE page_id = ?1",
        params![page_id],
        |r| r.get(0),
    )?;
    let version = previous_version.unwrap_or(0) + 1;

    let id = new_id();
    let content_path = content_path_for(&id);
    let reading_time = input.word_count.map(reading_minutes);
    let now = now_iso();

    conn.execute(
        "INSERT INTO snapshot
           (id, page_id, version, content_path, content_hash, byline, excerpt,
            word_count, reading_time, partial, captured_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            id,
            page_id,
            version,
            content_path,
            input.content_hash,
            input.byline,
            input.excerpt,
            input.word_count,
            reading_time,
            input.partial as i64,
            now,
        ],
    )?;

    let snapshot = get_snapshot(conn, &id)?.expect("snapshot just inserted");
    Ok(SnapshotDecision { snapshot, created: true })
}

/// Index a snapshot's normalized plain text into the contentless FTS table.
pub fn index_fts(
    conn: &Connection,
    snapshot: &Snapshot,
    content_text: &str,
    title: &str,
) -> rusqlite::Result<()> {
    let rowid: i64 = conn.query_row(
        "SELECT rowid FROM snapshot WHERE id = ?1",
        params![snapshot.id],
        |r| r.get(0),
    )?;
    conn.execute(
        "INSERT INTO snapshot_fts(rowid, content_text, title) VALUES (?1, ?2, ?3)",
        params![rowid, content_text, title],
    )?;
    Ok(())
}

/// Record which media hashes a snapshot references (for orphan GC).
pub fn link_media(conn: &Connection, snapshot_id: &str, hash: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO snapshot_media(snapshot_id, media_hash) VALUES (?1, ?2)",
        params![snapshot_id, hash],
    )?;
    Ok(())
}
