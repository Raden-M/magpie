//! Server-rendered web UI (maud): the reader view and the collection. The reader
//! is the centrepiece - a serif column with a scroll-spy "shiny things" rail; the
//! collection carries a live full-text search box above its list.

pub mod about;
pub mod assets;
pub mod collection;
pub mod layout;
pub mod reader;

/// CSP for server-rendered UI pages (collection, reader): our own
/// styles/scripts/images only, and never any script a stored snapshot might
/// carry (no `'unsafe-inline'` for scripts). `form-action 'self'` lets the
/// collection's search form fall back to a `/` GET when script is off;
/// `connect-src 'self'` lets `collection.js` fetch the live-search fragment.
pub const PAGE_CSP: &str = "default-src 'none'; img-src 'self' data:; \
     style-src 'self' 'unsafe-inline'; script-src 'self'; font-src 'self'; \
     connect-src 'self'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'";

/// Stricter CSP for the raw snapshot (no scripts at all).
pub const SNAPSHOT_CSP: &str = "default-src 'none'; img-src 'self' data:; \
     style-src 'self' 'unsafe-inline'; base-uri 'none'; form-action 'none'; \
     frame-ancestors 'none'";
