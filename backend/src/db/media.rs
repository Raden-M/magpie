//! Content-addressed media. Blobs live on disk under `<data_dir>/media/<hash>`;
//! this table records metadata and drives dedupe and orphan collection.

use rusqlite::{Connection, OptionalExtension, params};

use crate::db::models::Media;
use crate::util::now_iso;

pub fn media_exists(conn: &Connection, hash: &str) -> rusqlite::Result<bool> {
    let found: Option<i64> = conn
        .query_row("SELECT 1 FROM media WHERE hash = ?1", params![hash], |r| r.get(0))
        .optional()?;
    Ok(found.is_some())
}

/// Insert media metadata, ignoring a hash we already have (content addressing).
pub fn insert_media(conn: &Connection, media: &Media) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO media(hash, mime, byte_size, created_at)
         VALUES (?1, ?2, ?3, ?4)",
        params![media.hash, media.mime, media.byte_size, now_iso()],
    )?;
    Ok(())
}

pub fn get_media(conn: &Connection, hash: &str) -> rusqlite::Result<Option<Media>> {
    conn.query_row(
        "SELECT hash, mime, byte_size, created_at FROM media WHERE hash = ?1",
        params![hash],
        |r| {
            Ok(Media {
                hash: r.get(0)?,
                mime: r.get(1)?,
                byte_size: r.get(2)?,
                created_at: r.get(3)?,
            })
        },
    )
    .optional()
}

/// Media hashes present in the table but referenced by no snapshot and used as
/// no page favicon. These are safe to delete from disk and DB.
pub fn orphan_hashes(conn: &Connection) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT hash FROM media
         WHERE hash NOT IN (SELECT media_hash FROM snapshot_media)
           AND hash NOT IN (
             SELECT replace(favicon_path, '/media/', '')
             FROM page WHERE favicon_path IS NOT NULL
           )",
    )?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
    rows.collect()
}

pub fn delete_media_row(conn: &Connection, hash: &str) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM media WHERE hash = ?1", params![hash])?;
    Ok(())
}
