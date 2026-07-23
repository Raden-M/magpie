//! JSON wire shapes for the API, kept separate from the storage models so the
//! two can evolve independently.

use serde::Serialize;

use crate::db::models::{Highlight, Page, Snapshot};

#[derive(Debug, Serialize)]
pub struct PageJson {
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

impl From<Page> for PageJson {
    fn from(p: Page) -> Self {
        PageJson {
            id: p.id,
            canonical_url: p.canonical_url,
            original_url: p.original_url,
            title: p.title,
            site_name: p.site_name,
            lang: p.lang,
            favicon_path: p.favicon_path,
            first_captured_at: p.first_captured_at,
            last_captured_at: p.last_captured_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SnapshotJson {
    pub id: String,
    pub version: i64,
    pub byline: Option<String>,
    pub excerpt: Option<String>,
    pub word_count: Option<i64>,
    pub reading_time: Option<i64>,
    pub partial: bool,
    pub captured_at: String,
}

impl From<Snapshot> for SnapshotJson {
    fn from(s: Snapshot) -> Self {
        SnapshotJson {
            id: s.id,
            version: s.version,
            byline: s.byline,
            excerpt: s.excerpt,
            word_count: s.word_count,
            reading_time: s.reading_time,
            partial: s.partial,
            captured_at: s.captured_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct HighlightJson {
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

impl From<Highlight> for HighlightJson {
    fn from(h: Highlight) -> Self {
        HighlightJson {
            id: h.id,
            page_id: h.page_id,
            snapshot_id: h.snapshot_id,
            quote_exact: h.quote_exact,
            quote_prefix: h.quote_prefix,
            quote_suffix: h.quote_suffix,
            pos_start: h.pos_start,
            pos_end: h.pos_end,
            section_anchor: h.section_anchor,
            color: h.color,
            note: h.note,
            created_at: h.created_at,
            updated_at: h.updated_at,
        }
    }
}
