//! Snippet extraction: the default reader view. Rather than replay the whole
//! stored article, we show only the passage around each highlight - the block
//! it sits in, plus one block of context on each side - and let the full
//! snapshot stand behind it as an optional backup.
//!
//! This reuses [`bake::mark_document`]: the highlights are anchored and wrapped
//! in `<mark>` spans exactly as the full view would, then we walk *up* from each
//! mark to its surrounding block(s) and serialize just those. Because we grow
//! outward from the highlight within already-cleaned reader content, page chrome
//! never enters a snippet, and a highlight in the very first block simply has
//! nothing above it - the same behaviour on any site.
//!
//! Highlights whose passages touch or overlap are merged into one snippet, so a
//! shared paragraph (and its `<mark>` ids) is emitted exactly once.

use kuchikiki::NodeRef;

use crate::reader::bake::{self, BakeHighlight};

/// One rendered passage: the ids of the highlights it contains (document order)
/// and the block HTML, `<mark>` spans included.
pub struct Snippet {
    pub hl_ids: Vec<String>,
    pub html: String,
}

/// Innermost ancestors we treat as a self-contained block to show.
const BLOCK_TAGS: &[&str] = &[
    "p",
    "blockquote",
    "pre",
    "figure",
    "figcaption",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "li",
    "dd",
    "dt",
    "td",
    "th",
    "caption",
    "summary",
];

/// A contiguous run of one parent's element children, with the highlights it
/// holds. Starts as a single highlight's block span; adjacent runs are merged.
struct Group {
    parent: NodeRef,
    core_start: usize,
    core_end: usize,
    /// Position of the earliest highlight in document order - the sort key that
    /// puts merged snippets back into reading order.
    order: usize,
    hl_ids: Vec<String>,
}

pub fn snippets(content_html: &str, highlights: &[BakeHighlight<'_>]) -> Vec<Snippet> {
    let marked = bake::mark_document(content_html, highlights);

    // Every `<mark>` in document order, grouped by highlight id. `mark_document`
    // gives every fragment `data-hl={id}`; descendants() is document order, so
    // marks[0] is the highlight's start and marks.last() its end.
    let mut marks_by_hl: Vec<(String, Vec<NodeRef>)> = Vec::new();
    for node in marked.document.descendants() {
        let Some(el) = node.as_element() else {
            continue;
        };
        if el.name.local.as_ref() != "mark" {
            continue;
        }
        let Some(hl_id) = el.attributes.borrow().get("data-hl").map(str::to_string) else {
            continue;
        };
        match marks_by_hl.iter_mut().find(|(id, _)| *id == hl_id) {
            Some((_, marks)) => marks.push(node.clone()),
            None => marks_by_hl.push((hl_id, vec![node.clone()])),
        }
    }
    let hl_order = |id: &str| {
        marks_by_hl
            .iter()
            .position(|(m, _)| m == id)
            .unwrap_or(usize::MAX)
    };

    // One group per highlight to start with.
    let mut groups: Vec<Group> = Vec::new();
    for (order, (hl_id, marks)) in marks_by_hl.iter().enumerate() {
        let first_block = snippet_block(&marks[0]);
        let last_block = snippet_block(marks.last().expect("at least one mark"));
        let Some(parent) = first_block.parent() else {
            continue;
        };
        let siblings = element_children(&parent);
        let Some(start) = index_of(&siblings, &first_block) else {
            continue;
        };
        // A highlight spanning blocks under one parent covers the whole run; one
        // that crosses a structural boundary degrades to its first block.
        let end = match last_block.parent() {
            Some(p) if p == parent => index_of(&siblings, &last_block).unwrap_or(start),
            _ => start,
        };
        groups.push(Group {
            parent,
            core_start: start.min(end),
            core_end: start.max(end),
            order,
            hl_ids: vec![hl_id.clone()],
        });
    }

    merge_adjacent(&mut groups);

    // Add one block of context each side, without letting an earlier group's
    // trailing context and a later group's leading context claim the same block.
    let mut snippets: Vec<SnippetWithOrder> = Vec::new();
    for parent in unique_parents(&groups) {
        let siblings = element_children(&parent);
        let n = siblings.len();
        let mut in_parent: Vec<&Group> = groups.iter().filter(|g| g.parent == parent).collect();
        in_parent.sort_by_key(|g| g.core_start);

        let mut prev_hi: Option<usize> = None;
        for g in in_parent {
            let mut lo = g.core_start.saturating_sub(1);
            if let Some(ph) = prev_hi {
                lo = lo.max(ph + 1);
            }
            let hi = (g.core_end + 1).min(n.saturating_sub(1));
            prev_hi = Some(hi);

            let html: String = siblings[lo..=hi].iter().map(|c| c.to_string()).collect();
            let mut hl_ids = g.hl_ids.clone();
            hl_ids.sort_by_key(|id| hl_order(id));
            snippets.push(SnippetWithOrder {
                order: g.order,
                snippet: Snippet { hl_ids, html },
            });
        }
    }

    snippets.sort_by_key(|s| s.order);
    snippets.into_iter().map(|s| s.snippet).collect()
}

struct SnippetWithOrder {
    order: usize,
    snippet: Snippet,
}

/// Repeatedly fold overlapping / adjacent same-parent groups together until no
/// further merge is possible (a fold can create a new adjacency).
fn merge_adjacent(groups: &mut Vec<Group>) {
    let mut changed = true;
    while changed {
        changed = false;
        let mut i = 0;
        'outer: while i < groups.len() {
            for j in (i + 1)..groups.len() {
                if groups[i].parent == groups[j].parent
                    && groups[i].core_start <= groups[j].core_end + 1
                    && groups[j].core_start <= groups[i].core_end + 1
                {
                    let other = groups.remove(j);
                    let g = &mut groups[i];
                    g.core_start = g.core_start.min(other.core_start);
                    g.core_end = g.core_end.max(other.core_end);
                    g.order = g.order.min(other.order);
                    g.hl_ids.extend(other.hl_ids);
                    changed = true;
                    continue 'outer;
                }
            }
            i += 1;
        }
    }
}

/// The distinct parent nodes across all groups, in first-seen order.
fn unique_parents(groups: &[Group]) -> Vec<NodeRef> {
    let mut out: Vec<NodeRef> = Vec::new();
    for g in groups {
        if !out.contains(&g.parent) {
            out.push(g.parent.clone());
        }
    }
    out
}

/// The block to show for a mark: its innermost block ancestor, lifted to the
/// enclosing list or table when the mark sits in an item or cell (so the snippet
/// stays valid, coherent HTML) and to a wrapping quote/figure when it frames the
/// paragraph.
fn snippet_block(mark: &NodeRef) -> NodeRef {
    let block = mark
        .inclusive_ancestors()
        .find(|a| tag_is(a, BLOCK_TAGS))
        .or_else(|| mark.parent())
        .unwrap_or_else(|| mark.clone());

    match element_tag(&block).as_deref() {
        Some("li" | "dd" | "dt") => {
            nearest_ancestor(&block, &["ul", "ol", "dl", "menu"]).unwrap_or(block)
        }
        Some("td" | "th" | "caption") => nearest_ancestor(&block, &["table"]).unwrap_or(block),
        Some("p") => match block.parent() {
            Some(p) if tag_is(&p, &["blockquote", "figure", "figcaption"]) => p,
            _ => block,
        },
        _ => block,
    }
}

fn nearest_ancestor(node: &NodeRef, tags: &[&str]) -> Option<NodeRef> {
    node.ancestors().find(|a| tag_is(a, tags))
}

fn element_children(parent: &NodeRef) -> Vec<NodeRef> {
    parent
        .children()
        .filter(|n| n.as_element().is_some())
        .collect()
}

fn index_of(children: &[NodeRef], node: &NodeRef) -> Option<usize> {
    children.iter().position(|c| c == node)
}

fn element_tag(node: &NodeRef) -> Option<String> {
    node.as_element()
        .map(|el| el.name.local.as_ref().to_string())
}

fn tag_is(node: &NodeRef, tags: &[&str]) -> bool {
    node.as_element()
        .is_some_and(|el| tags.contains(&el.name.local.as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::anchor::Selectors;

    fn hl<'a>(id: &'a str, quote: &'a str, prefix: &'a str, suffix: &'a str) -> BakeHighlight<'a> {
        BakeHighlight {
            id,
            color: "iridescent",
            selectors: Selectors {
                quote_exact: quote,
                quote_prefix: prefix,
                quote_suffix: suffix,
                pos_start: None,
                pos_end: None,
            },
        }
    }

    #[test]
    fn snippet_is_the_block_plus_one_each_side() {
        let html = "<article>\
            <p>Intro before.</p>\
            <p>Alpha.</p>\
            <p>The quick brown fox jumps.</p>\
            <p>Omega.</p>\
            <p>Tail after.</p></article>";
        let out = snippets(html, &[hl("h1", "brown fox", "quick ", " jumps")]);
        assert_eq!(out.len(), 1);
        let s = &out[0];
        assert_eq!(s.hl_ids, vec!["h1"]);
        assert!(s.html.contains("data-hl=\"h1\""));
        // The highlight's block, plus exactly one neighbour on each side.
        assert!(s.html.contains("Alpha."));
        assert!(s.html.contains("Omega."));
        // But not two away.
        assert!(!s.html.contains("Intro before."));
        assert!(!s.html.contains("Tail after."));
    }

    #[test]
    fn first_block_has_no_leading_context() {
        let html = "<article><p>The quick brown fox.</p><p>Second.</p><p>Third.</p></article>";
        let out = snippets(html, &[hl("h1", "quick brown", "The ", " fox")]);
        assert_eq!(out.len(), 1);
        assert!(out[0].html.contains("data-hl=\"h1\""));
        assert!(out[0].html.contains("Second."));
        assert!(!out[0].html.contains("Third."));
    }

    #[test]
    fn two_highlights_in_one_paragraph_merge_to_one_snippet() {
        let html = "<article><p>Before.</p><p>The quick brown fox jumps the lazy dog.</p>\
            <p>After.</p></article>";
        let out = snippets(
            html,
            &[
                hl("a", "quick brown", "The ", " fox"),
                hl("b", "lazy dog", "the ", "."),
            ],
        );
        assert_eq!(out.len(), 1, "same paragraph yields one snippet");
        let s = &out[0];
        assert_eq!(s.hl_ids, vec!["a", "b"]);
        // Each id appears exactly once - no duplicate ids across cards.
        assert_eq!(s.html.matches("id=\"hl-a\"").count(), 1);
        assert_eq!(s.html.matches("id=\"hl-b\"").count(), 1);
    }

    #[test]
    fn distant_highlights_yield_separate_snippets_in_order() {
        let mut paras = String::from("<article>");
        paras.push_str("<p>The alpha marker here.</p>");
        for _ in 0..6 {
            paras.push_str("<p>Filler paragraph text.</p>");
        }
        paras.push_str("<p>The omega marker here.</p></article>");
        let out = snippets(
            &paras,
            &[
                hl("z", "omega marker", "The ", " here"),
                hl("a", "alpha marker", "The ", " here"),
            ],
        );
        assert_eq!(out.len(), 2);
        // Document order, not the order highlights were passed.
        assert_eq!(out[0].hl_ids, vec!["a"]);
        assert_eq!(out[1].hl_ids, vec!["z"]);
    }

    #[test]
    fn list_item_snippet_keeps_the_list_wrapper() {
        let html = "<article><p>Lead.</p><ul><li>First item.</li>\
            <li>The quick brown fox item.</li><li>Third item.</li></ul>\
            <p>Trail.</p></article>";
        let out = snippets(html, &[hl("h1", "brown fox", "quick ", " item")]);
        assert_eq!(out.len(), 1);
        let s = &out[0];
        // Whole list is shown as a valid <ul>, with siblings for context.
        assert!(s.html.contains("<ul>"));
        assert!(s.html.contains("First item."));
        assert!(s.html.contains("Third item."));
        assert!(s.html.contains("Lead."));
        assert!(s.html.contains("Trail."));
    }

    #[test]
    fn orphaned_highlight_produces_no_snippet() {
        let html = "<article><p>Nothing to see here.</p></article>";
        let out = snippets(html, &[hl("h1", "xyzzy plugh", "", "")]);
        assert!(out.is_empty());
    }
}
