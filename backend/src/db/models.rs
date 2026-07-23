//! Row structs mirroring the schema, plus the input structs the query layer
//! accepts. Serialization lives in the API modules, not here, so the storage
//! shape stays independent of the wire shape.

use rusqlite::Row;

#[derive(Debug, Clone)]
pub struct Page {
    pub id: String,
    pub canonical_url: String,
    pub original_url: String,
    pub title: String,
    pub site_name: Option<String>,
    pub lang: Option<String>,
    pub favicon_path: Option<String>,
    pub first_captured_at: String,
    pub last_captured_at: String,
}

impl Page {
    pub fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Page {
            id: row.get("id")?,
            canonical_url: row.get("canonical_url")?,
            original_url: row.get("original_url")?,
            title: row.get("title")?,
            site_name: row.get("site_name")?,
            lang: row.get("lang")?,
            favicon_path: row.get("favicon_path")?,
            first_captured_at: row.get("first_captured_at")?,
            last_captured_at: row.get("last_captured_at")?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub id: String,
    pub page_id: String,
    pub version: i64,
    pub content_path: String,
    pub content_hash: String,
    pub byline: Option<String>,
    pub excerpt: Option<String>,
    pub word_count: Option<i64>,
    pub reading_time: Option<i64>,
    pub partial: bool,
    pub captured_at: String,
}

impl Snapshot {
    pub fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Snapshot {
            id: row.get("id")?,
            page_id: row.get("page_id")?,
            version: row.get("version")?,
            content_path: row.get("content_path")?,
            content_hash: row.get("content_hash")?,
            byline: row.get("byline")?,
            excerpt: row.get("excerpt")?,
            word_count: row.get("word_count")?,
            reading_time: row.get("reading_time")?,
            partial: row.get::<_, i64>("partial")? != 0,
            captured_at: row.get("captured_at")?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Highlight {
    pub id: String,
    pub page_id: String,
    pub snapshot_id: Option<String>,
    pub quote_exact: String,
    pub quote_prefix: String,
    pub quote_suffix: String,
    pub pos_start: Option<i64>,
    pub pos_end: Option<i64>,
    pub section_anchor: Option<String>,
    pub color: String,
    pub note: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl Highlight {
    pub fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Highlight {
            id: row.get("id")?,
            page_id: row.get("page_id")?,
            snapshot_id: row.get("snapshot_id")?,
            quote_exact: row.get("quote_exact")?,
            quote_prefix: row.get("quote_prefix")?,
            quote_suffix: row.get("quote_suffix")?,
            pos_start: row.get("pos_start")?,
            pos_end: row.get("pos_end")?,
            section_anchor: row.get("section_anchor")?,
            color: row.get("color")?,
            note: row.get("note")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Media {
    pub hash: String,
    pub mime: String,
    pub byte_size: i64,
    pub created_at: String,
}

/// Fields needed to upsert a page. `id`/`first_captured_at` are only used when
/// the page is new; an existing page keeps its original values for those.
#[derive(Debug, Clone)]
pub struct NewPage {
    pub canonical_url: String,
    pub original_url: String,
    pub title: String,
    pub site_name: Option<String>,
    pub lang: Option<String>,
    pub favicon_path: Option<String>,
}

/// Content for a would-be snapshot version. The query layer decides whether this
/// becomes a new version or reuses the latest, per the versioning rule.
#[derive(Debug, Clone)]
pub struct NewSnapshot {
    pub content_hash: String,
    pub byline: Option<String>,
    pub excerpt: Option<String>,
    pub word_count: Option<i64>,
    pub partial: bool,
}

#[derive(Debug, Clone)]
pub struct NewHighlight {
    pub page_id: String,
    pub snapshot_id: Option<String>,
    pub quote_exact: String,
    pub quote_prefix: String,
    pub quote_suffix: String,
    pub pos_start: Option<i64>,
    pub pos_end: Option<i64>,
    pub section_anchor: Option<String>,
    pub color: String,
    pub note: Option<String>,
}
