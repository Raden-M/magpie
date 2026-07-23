-- Magpie initial schema: pages, versioned snapshots, highlights, media,
-- plus FTS5 indexes kept in sync by triggers. See docs/03-data-model.md.

-- Pages: one row per source web page, grouped by canonical URL.
CREATE TABLE page (
  id                TEXT PRIMARY KEY,          -- uuidv7 (time-ordered)
  canonical_url     TEXT NOT NULL UNIQUE,      -- normalized; the grouping key
  original_url      TEXT NOT NULL,             -- the URL actually captured from
  title             TEXT NOT NULL,
  site_name         TEXT,
  lang              TEXT,
  favicon_path      TEXT,                      -- media path, nullable
  first_captured_at TEXT NOT NULL,             -- ISO-8601 UTC
  last_captured_at  TEXT NOT NULL
);

-- Snapshots: versioned reader-view content for a page.
CREATE TABLE snapshot (
  id            TEXT PRIMARY KEY,              -- uuidv7
  page_id       TEXT NOT NULL REFERENCES page(id) ON DELETE CASCADE,
  version       INTEGER NOT NULL,              -- 1, 2, ... per page
  content_path  TEXT NOT NULL,                 -- path to sanitized reader HTML on disk
  content_hash  TEXT NOT NULL,                 -- sha256 of normalized content; dedupe/versioning
  byline        TEXT,
  excerpt       TEXT,
  word_count    INTEGER,
  reading_time  INTEGER,                       -- minutes, derived
  partial       INTEGER NOT NULL DEFAULT 0,    -- 1 if Readability fell back
  captured_at   TEXT NOT NULL,
  UNIQUE (page_id, version)
);

-- Highlights: one kept selection.
CREATE TABLE highlight (
  id             TEXT PRIMARY KEY,             -- uuidv7
  page_id        TEXT NOT NULL REFERENCES page(id) ON DELETE CASCADE,
  snapshot_id    TEXT REFERENCES snapshot(id) ON DELETE SET NULL,
  quote_exact    TEXT NOT NULL,                -- the selected text, normalized
  quote_prefix   TEXT NOT NULL DEFAULT '',     -- ~32 chars before
  quote_suffix   TEXT NOT NULL DEFAULT '',     -- ~32 chars after
  pos_start      INTEGER,                      -- char offset into snapshot normalized text
  pos_end        INTEGER,
  section_anchor TEXT,                         -- id of the containing heading/block
  color          TEXT NOT NULL DEFAULT 'iridescent',
  note           TEXT,
  created_at     TEXT NOT NULL,
  updated_at     TEXT NOT NULL
);

CREATE INDEX idx_highlight_page ON highlight(page_id, created_at);
CREATE INDEX idx_snapshot_page  ON snapshot(page_id, version DESC);

-- Content-addressed media (images, favicons) extracted from pages.
CREATE TABLE media (
  hash        TEXT PRIMARY KEY,   -- sha256 of bytes; also the on-disk filename
  mime        TEXT NOT NULL,
  byte_size   INTEGER NOT NULL,
  created_at  TEXT NOT NULL
);

-- Which media a given snapshot references (parsed from its manifest at capture
-- time). This is what makes orphan-media garbage collection exact: a media row
-- is collectable once no snapshot references it and no page uses it as a favicon.
CREATE TABLE snapshot_media (
  snapshot_id TEXT NOT NULL REFERENCES snapshot(id) ON DELETE CASCADE,
  media_hash  TEXT NOT NULL REFERENCES media(hash) ON DELETE CASCADE,
  PRIMARY KEY (snapshot_id, media_hash)
);

-- Idempotency ledger: maps a client capture_id to the result it produced, so a
-- retried capture replays the original response instead of duplicating.
CREATE TABLE capture_log (
  capture_id   TEXT PRIMARY KEY,
  page_id      TEXT NOT NULL,
  snapshot_id  TEXT NOT NULL,
  highlight_id TEXT,
  created_at   TEXT NOT NULL
);

-- Full-text search over highlight quotes and notes (external-content FTS).
CREATE VIRTUAL TABLE highlight_fts USING fts5(
  quote_exact,
  note,
  content='highlight',
  content_rowid='rowid',
  tokenize = 'unicode61 remove_diacritics 2'
);

CREATE TRIGGER highlight_ai AFTER INSERT ON highlight BEGIN
  INSERT INTO highlight_fts(rowid, quote_exact, note)
  VALUES (new.rowid, new.quote_exact, new.note);
END;

CREATE TRIGGER highlight_ad AFTER DELETE ON highlight BEGIN
  INSERT INTO highlight_fts(highlight_fts, rowid, quote_exact, note)
  VALUES ('delete', old.rowid, old.quote_exact, old.note);
END;

CREATE TRIGGER highlight_au AFTER UPDATE ON highlight BEGIN
  INSERT INTO highlight_fts(highlight_fts, rowid, quote_exact, note)
  VALUES ('delete', old.rowid, old.quote_exact, old.note);
  INSERT INTO highlight_fts(rowid, quote_exact, note)
  VALUES (new.rowid, new.quote_exact, new.note);
END;

-- Full-text search over snapshot plain text. A regular (self-contained) FTS5
-- table: it stores the tokenized plain text so search can return highlighted
-- snippets. The rendered HTML still lives on disk; this holds only extracted
-- text for search. Rows are keyed by the snapshot's rowid.
CREATE VIRTUAL TABLE snapshot_fts USING fts5(
  content_text,
  title,
  tokenize = 'unicode61 remove_diacritics 2'
);

-- content_text is not a column on `snapshot`, so inserts into snapshot_fts
-- happen in application code at capture time. Deletion is by rowid, driven here.
CREATE TRIGGER snapshot_ad AFTER DELETE ON snapshot BEGIN
  DELETE FROM snapshot_fts WHERE rowid = old.rowid;
END;
