//! Server-rendered web UI (maud): the reader view, and later the collection and
//! search pages. The reader is the centrepiece - a serif column with a
//! scroll-spy "shiny things" rail.

pub mod reader;

/// CSP for the full reader page: our own styles/scripts/images only, and never
/// any script the stored snapshot might carry (no `'unsafe-inline'` for scripts).
pub const READER_CSP: &str = "default-src 'none'; img-src 'self' data:; \
     style-src 'self' 'unsafe-inline'; script-src 'self'; font-src 'self'; \
     base-uri 'none'; form-action 'none'; frame-ancestors 'none'";

/// Stricter CSP for the raw snapshot (no scripts at all).
pub const SNAPSHOT_CSP: &str = "default-src 'none'; img-src 'self' data:; \
     style-src 'self' 'unsafe-inline'; base-uri 'none'; form-action 'none'; \
     frame-ancestors 'none'";
