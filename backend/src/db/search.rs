//! Unified full-text search across highlight quotes/notes and snapshot content,
//! ranked with BM25 and grouped by page (docs/06-api.md `GET /api/search`).

use rusqlite::{Connection, params};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitKind {
    Highlight,
    Content,
}

#[derive(Debug, Clone)]
pub struct SearchHit {
    pub kind: HitKind,
    /// Highlight id (for a highlight hit) or snapshot id (for a content hit).
    pub id: String,
    pub snippet: String,
    pub rank: f64,
}

#[derive(Debug, Clone)]
pub struct PageGroup {
    pub page_id: String,
    pub page_title: String,
    pub best_rank: f64,
    pub hits: Vec<SearchHit>,
}

/// Turn free user input into a safe FTS5 MATCH expression: each whitespace token
/// becomes a quoted string (implicitly AND-ed), so punctuation can't produce a
/// syntax error. Returns `None` for an empty query.
pub fn to_match_query(q: &str) -> Option<String> {
    let terms: Vec<String> = q
        .split_whitespace()
        .map(|t| format!("\"{}\"", t.replace('"', "\"\"")))
        .collect();
    if terms.is_empty() {
        None
    } else {
        Some(terms.join(" "))
    }
}

/// Run search and return page groups sorted best-first (lower BM25 is better).
/// `cap` bounds how many raw hits of each kind are considered.
pub fn search(
    conn: &Connection,
    match_query: &str,
    cap: usize,
) -> rusqlite::Result<Vec<PageGroup>> {
    let mut groups: Vec<PageGroup> = Vec::new();
    let mut index: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    let mut push = |page_id: String, page_title: String, hit: SearchHit| {
        let slot = *index.entry(page_id.clone()).or_insert_with(|| {
            groups.push(PageGroup {
                page_id: page_id.clone(),
                page_title,
                best_rank: hit.rank,
                hits: Vec::new(),
            });
            groups.len() - 1
        });
        let group = &mut groups[slot];
        if hit.rank < group.best_rank {
            group.best_rank = hit.rank;
        }
        group.hits.push(hit);
    };

    // Highlight quotes and notes.
    {
        let mut stmt = conn.prepare(
            "SELECT h.id, h.page_id, p.title,
                    snippet(highlight_fts, -1, '<b>', '</b>', '…', 12) AS snip,
                    bm25(highlight_fts) AS rank
             FROM highlight_fts
             JOIN highlight h ON h.rowid = highlight_fts.rowid
             JOIN page p ON p.id = h.page_id
             WHERE highlight_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![match_query, cap as i64], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, f64>(4)?,
            ))
        })?;
        for row in rows {
            let (id, page_id, title, snip, rank) = row?;
            push(
                page_id,
                title,
                SearchHit {
                    kind: HitKind::Highlight,
                    id,
                    snippet: snip,
                    rank,
                },
            );
        }
    }

    // Snapshot content.
    {
        let mut stmt = conn.prepare(
            "SELECT s.id, s.page_id, p.title,
                    snippet(snapshot_fts, 0, '<b>', '</b>', '…', 16) AS snip,
                    bm25(snapshot_fts) AS rank
             FROM snapshot_fts
             JOIN snapshot s ON s.rowid = snapshot_fts.rowid
             JOIN page p ON p.id = s.page_id
             WHERE snapshot_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![match_query, cap as i64], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, f64>(4)?,
            ))
        })?;
        for row in rows {
            let (id, page_id, title, snip, rank) = row?;
            push(
                page_id,
                title,
                SearchHit {
                    kind: HitKind::Content,
                    id,
                    snippet: snip,
                    rank,
                },
            );
        }
    }

    for group in &mut groups {
        group.hits.sort_by(|a, b| a.rank.total_cmp(&b.rank));
    }
    groups.sort_by(|a, b| a.best_rank.total_cmp(&b.best_rank));
    Ok(groups)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        crate::db::migrations().to_latest(&mut conn).unwrap();

        conn.execute(
            "INSERT INTO page (id, canonical_url, original_url, title, first_captured_at, last_captured_at)
             VALUES ('p1','https://e.com/a','https://e.com/a','On Structural Colour','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO snapshot (id, page_id, version, content_path, content_hash, captured_at)
             VALUES ('s1','p1',1,'snapshots/s1.html','h','2026-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
        let rowid: i64 = conn
            .query_row("SELECT rowid FROM snapshot WHERE id='s1'", [], |r| r.get(0))
            .unwrap();
        conn.execute(
            "INSERT INTO snapshot_fts(rowid, content_text, title)
             VALUES (?1, 'the colour is an interference effect in the feather structure', 'On Structural Colour')",
            [rowid],
        ).unwrap();
        conn.execute(
            "INSERT INTO highlight (id, page_id, snapshot_id, quote_exact, created_at, updated_at)
             VALUES ('h1','p1','s1','structural colour is not pigment','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
            [],
        ).unwrap();
        conn
    }

    #[test]
    fn groups_hits_by_page() {
        let conn = setup();
        let mq = to_match_query("colour").unwrap();
        let groups = search(&conn, &mq, 100).unwrap();
        assert_eq!(groups.len(), 1);
        let group = &groups[0];
        assert_eq!(group.page_id, "p1");
        // Both a highlight hit and a content hit match "colour".
        assert!(group.hits.iter().any(|h| h.kind == HitKind::Highlight));
        assert!(group.hits.iter().any(|h| h.kind == HitKind::Content));
        assert!(group.hits.iter().any(|h| h.snippet.contains("<b>")));
    }

    #[test]
    fn content_only_term_matches_snapshot() {
        let conn = setup();
        let mq = to_match_query("interference").unwrap();
        let groups = search(&conn, &mq, 100).unwrap();
        assert_eq!(groups.len(), 1);
        assert!(groups[0].hits.iter().all(|h| h.kind == HitKind::Content));
    }

    #[test]
    fn punctuation_in_query_does_not_error() {
        let conn = setup();
        let mq = to_match_query("colour \"quoted\" (paren)").unwrap();
        assert!(search(&conn, &mq, 100).is_ok());
    }
}
