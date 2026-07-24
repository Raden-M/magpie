//! Baking: parse a stored snapshot, locate each highlight in its normalized
//! text, and wrap the matching character ranges in `<mark>` spans - the MVP
//! reader-view rendering (ADR-006). Correctness-first; the reader is not hot.

use std::collections::{HashMap, HashSet};

use html5ever::{LocalName, QualName, namespace_url, ns};
use kuchikiki::traits::*;
use kuchikiki::{Attribute, ExpandedName, NodeRef};

use crate::reader::anchor::{self, Selectors};
use crate::reader::normalize::normalize_pieces;

/// A highlight to bake: its id, colour, and anchoring selectors.
pub struct BakeHighlight<'a> {
    pub id: &'a str,
    pub color: &'a str,
    pub selectors: Selectors<'a>,
}

pub struct BakeResult {
    /// The article's inner HTML with highlight `<mark>` spans applied.
    pub html: String,
    /// Ids of highlights that were successfully anchored (the rest are orphaned).
    pub located: HashSet<String>,
}

/// A parsed snapshot with every located highlight wrapped in `<mark>` spans.
/// The full-article [`bake`] serializes its body; the snippet extractor
/// ([`crate::reader::snippet`]) walks the same tree to pull out the blocks
/// around each highlight - both work from this one marking pass.
pub struct Marked {
    pub document: NodeRef,
    /// Ids of highlights that were successfully anchored (the rest are orphaned).
    pub located: HashSet<String>,
}

/// A run to wrap within a single text node: `[start, end)` in char offsets.
struct Segment {
    start: usize,
    end: usize,
    hl_id: String,
    color: String,
    /// Whether this run is the first / last of its highlight in document order.
    /// A quote broken by inline markup becomes several `<mark>`s, and only the
    /// outermost two may be drawn with an end (see `.hl-open` / `.hl-close`).
    opens: bool,
    closes: bool,
}

fn char_to_byte(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(b, _)| b)
        .unwrap_or(s.len())
}

fn make_mark(hl_id: &str, color: &str, opens: bool, closes: bool) -> NodeRef {
    let name = QualName::new(None, ns!(html), LocalName::from("mark"));
    let mut class = format!("hl hl-{color}");
    if opens {
        class.push_str(" hl-open");
    }
    if closes {
        class.push_str(" hl-close");
    }
    let mut attrs: Vec<(ExpandedName, Attribute)> = vec![
        (
            ExpandedName::new(ns!(), LocalName::from("class")),
            Attribute {
                prefix: None,
                value: class,
            },
        ),
        (
            ExpandedName::new(ns!(), LocalName::from("data-hl")),
            Attribute {
                prefix: None,
                value: hl_id.to_string(),
            },
        ),
    ];
    if opens {
        attrs.push((
            ExpandedName::new(ns!(), LocalName::from("id")),
            Attribute {
                prefix: None,
                value: format!("hl-{hl_id}"),
            },
        ));
    }
    NodeRef::new_element(name, attrs)
}

/// True if any ancestor is an element whose text we must not treat as content.
fn in_ignored_container(node: &NodeRef) -> bool {
    node.ancestors().any(|a| {
        a.as_element().is_some_and(|el| {
            let local = el.name.local.as_ref();
            local == "script" || local == "style" || local == "template" || local == "head"
        })
    })
}

pub fn bake(content_html: &str, highlights: &[BakeHighlight<'_>]) -> BakeResult {
    let marked = mark_document(content_html, highlights);

    // Serialize the body's inner HTML.
    let html = match marked.document.select_first("body") {
        Ok(body) => body.as_node().children().map(|c| c.to_string()).collect(),
        Err(_) => marked.document.to_string(),
    };

    BakeResult {
        html,
        located: marked.located,
    }
}

/// Parse a snapshot, locate every highlight in its normalized text, and wrap the
/// matching character ranges in `<mark>` spans - returning the live tree so a
/// caller can either serialize it whole ([`bake`]) or extract regions of it.
pub fn mark_document(content_html: &str, highlights: &[BakeHighlight<'_>]) -> Marked {
    let document = kuchikiki::parse_html().one(content_html);

    // 1. Collect text nodes in document order.
    let mut nodes: Vec<NodeRef> = Vec::new();
    let mut pieces: Vec<String> = Vec::new();
    for node in document.descendants() {
        if let Some(text_cell) = node.as_text() {
            if in_ignored_container(&node) {
                continue;
            }
            pieces.push(text_cell.borrow().clone());
            nodes.push(node.clone());
        }
    }

    // 2. Normalize into one coordinate system.
    let piece_refs: Vec<&str> = pieces.iter().map(|s| s.as_str()).collect();
    let normalized = normalize_pieces(&piece_refs);
    let hay: Vec<char> = normalized.text.chars().collect();

    // 3. Locate each highlight; collect per-piece segments.
    let mut per_piece: HashMap<usize, Vec<Segment>> = HashMap::new();
    let mut located: HashSet<String> = HashSet::new();

    for hl in highlights {
        let Some(range) = anchor::locate(&hay, &hl.selectors) else {
            continue;
        };
        located.insert(hl.id.to_string());
        tracing::trace!(highlight = %hl.id, method = ?range.method, "highlight anchored");
        for (piece, start, end) in piece_ranges(&normalized.map, range.start, range.end) {
            per_piece.entry(piece).or_default().push(Segment {
                start,
                end,
                hl_id: hl.id.to_string(),
                color: hl.color.to_string(),
                opens: false,
                closes: false,
            });
        }
    }

    // 4. Resolve overlaps first, so the edges are flagged on segments that
    //    survive: the opening one also carries the unique jump-to `id`.
    for segments in per_piece.values_mut() {
        clip_overlaps(segments);
    }
    per_piece.retain(|_, segments| !segments.is_empty());
    mark_edge_segments(&mut per_piece);

    // 5. Rewrite each touched text node.
    for (piece_idx, segments) in &per_piece {
        rewrite_text_node(&nodes[*piece_idx], &pieces[*piece_idx], segments);
    }

    Marked { document, located }
}

/// Map a normalized `[start, end)` range to `(piece, start_off, end_off)` chunks,
/// one per text node the range touches. Offsets are character offsets into the
/// piece; `end_off` is exclusive.
fn piece_ranges(
    map: &[crate::reader::normalize::CharSource],
    start: usize,
    end: usize,
) -> Vec<(usize, usize, usize)> {
    let mut out: Vec<(usize, usize, usize)> = Vec::new();
    for src in &map[start..end.min(map.len())] {
        match out.last_mut() {
            Some(last) if last.0 == src.piece => last.2 = src.offset + 1,
            _ => out.push((src.piece, src.offset, src.offset + 1)),
        }
    }
    out
}

/// Where a segment sits in the article: `(text node index, char offset)`.
type Pos = (usize, usize);

/// Flag the first and last segment of each highlight. Text nodes are collected
/// in document order, so `Pos` orders segments the way they read.
fn mark_edge_segments(per_piece: &mut HashMap<usize, Vec<Segment>>) {
    let mut bounds: HashMap<String, (Pos, Pos)> = HashMap::new();
    for (&piece, segs) in per_piece.iter() {
        for seg in segs {
            let pos = (piece, seg.start);
            let entry = bounds.entry(seg.hl_id.clone()).or_insert((pos, pos));
            entry.0 = entry.0.min(pos);
            entry.1 = entry.1.max(pos);
        }
    }
    for (&piece, segs) in per_piece.iter_mut() {
        for seg in segs.iter_mut() {
            let pos = (piece, seg.start);
            if let Some(&(first, last)) = bounds.get(&seg.hl_id) {
                seg.opens = pos == first;
                seg.closes = pos == last;
            }
        }
    }
}

/// Resolve overlaps within a single text node by clipping later segments so no
/// two `<mark>`s nest. Segments are consumed sorted by start.
fn clip_overlaps(segments: &mut Vec<Segment>) {
    segments.sort_by_key(|s| (s.start, s.end));
    let mut cursor = 0usize;
    segments.retain_mut(|seg| {
        if seg.start < cursor {
            seg.start = cursor;
        }
        if seg.start >= seg.end {
            return false;
        }
        cursor = seg.end;
        true
    });
}

fn rewrite_text_node(node: &NodeRef, text: &str, segments: &[Segment]) {
    let mut cursor = 0usize; // char offset
    let mut new_nodes: Vec<NodeRef> = Vec::new();

    for seg in segments {
        if seg.start > cursor {
            let slice = &text[char_to_byte(text, cursor)..char_to_byte(text, seg.start)];
            new_nodes.push(NodeRef::new_text(slice));
        }
        let slice = &text[char_to_byte(text, seg.start)..char_to_byte(text, seg.end)];
        let mark = make_mark(&seg.hl_id, &seg.color, seg.opens, seg.closes);
        mark.append(NodeRef::new_text(slice));
        new_nodes.push(mark);
        cursor = seg.end;
    }
    let total = text.chars().count();
    if cursor < total {
        let slice = &text[char_to_byte(text, cursor)..];
        new_nodes.push(NodeRef::new_text(slice));
    }

    for n in new_nodes {
        node.insert_before(n);
    }
    node.detach();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sel<'a>(quote: &'a str, prefix: &'a str, suffix: &'a str) -> Selectors<'a> {
        Selectors {
            quote_exact: quote,
            quote_prefix: prefix,
            quote_suffix: suffix,
            pos_start: None,
            pos_end: None,
        }
    }

    #[test]
    fn wraps_exact_quote_in_a_single_node() {
        let html = "<article><p>The quick brown fox jumps.</p></article>";
        let hls = vec![BakeHighlight {
            id: "h1",
            color: "iridescent",
            selectors: sel("brown fox", "quick ", " jumps"),
        }];
        let res = bake(html, &hls);
        assert!(res.located.contains("h1"));
        assert!(res.html.contains("data-hl=\"h1\""));
        assert!(res.html.contains("id=\"hl-h1\""));
        assert!(res.html.contains(">brown fox</mark>"));
        // Text outside the quote is left intact.
        assert!(res.html.contains("The quick "));
        assert!(res.html.contains(" jumps."));
    }

    #[test]
    fn wraps_across_an_inline_element_boundary() {
        // The quote spans the end of a link and the following text node.
        let html = "<article><p>The <a href=\"/x\">quick brown</a> fox jumps.</p></article>";
        let hls = vec![BakeHighlight {
            id: "h2",
            color: "iridescent",
            selectors: sel("brown fox", "quick ", " jumps"),
        }];
        let res = bake(html, &hls);
        assert!(res.located.contains("h2"));
        // Two marks, one per text node, sharing the highlight id.
        let mark_count = res.html.matches("data-hl=\"h2\"").count();
        assert_eq!(mark_count, 2, "one mark per spanned text node");
        // Only the first carries the jump-to id.
        assert_eq!(res.html.matches("id=\"hl-h2\"").count(), 1);
        assert!(res.html.contains("brown</mark>"));
        assert!(res.html.contains(">fox</mark>") || res.html.contains(" fox</mark>"));
        // Exactly one open and one close edge, so the pair draws as one run.
        assert_eq!(res.html.matches("hl-open").count(), 1);
        assert_eq!(res.html.matches("hl-close").count(), 1);
    }

    #[test]
    fn interior_fragments_carry_no_edges() {
        // Wikipedia's `700<span>&nbsp;</span>million`: the middle text node is a
        // fragment of the quote with neither of its ends.
        let html = "<article><p>at 700<span>\u{a0}</span>million and up.</p></article>";
        let hls = vec![BakeHighlight {
            id: "h4",
            color: "iridescent",
            selectors: sel("700 million", "at ", " and"),
        }];
        let res = bake(html, &hls);
        assert!(res.located.contains("h4"));
        assert_eq!(res.html.matches("data-hl=\"h4\"").count(), 3);
        assert_eq!(res.html.matches("hl-open").count(), 1);
        assert_eq!(res.html.matches("hl-close").count(), 1);
        // The nbsp fragment is a plain middle - no edge classes at all.
        assert!(
            res.html
                .contains("class=\"hl hl-iridescent\" data-hl=\"h4\">&nbsp;</mark>")
        );
    }

    #[test]
    fn orphaned_highlight_is_not_baked() {
        let html = "<article><p>Nothing to see here.</p></article>";
        let hls = vec![BakeHighlight {
            id: "h3",
            color: "iridescent",
            selectors: sel("xyzzy plugh", "", ""),
        }];
        let res = bake(html, &hls);
        assert!(!res.located.contains("h3"));
        assert!(!res.html.contains("<mark"));
    }
}
