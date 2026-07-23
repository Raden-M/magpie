//! Text normalization - the shared coordinate system.
//!
//! Positions and quotes are only comparable if the extension (at capture) and
//! the backend (at render) normalize text **identically**. This module is the
//! canonical algorithm; `extension/src/shared/normalize.ts` mirrors it byte for
//! byte, and `tests/` covers the round-trip.
//!
//! The algorithm, applied to plain text in document order:
//!   1. collapse every run of Unicode whitespace to a single space, and trim;
//!   2. fold smart quotes, dashes, and non-breaking spaces to ASCII (for
//!      matching only - the *displayed* text keeps its originals).
//!
//! We deliberately do **not** apply full Unicode NFC. Whole-string NFC cannot be
//! reproduced identically across Rust and JS while also maintaining an exact
//! character-to-source-offset map (combining marks straddling text-node
//! boundaries would diverge). Every fold here is strictly one-scalar-to-one, so
//! the map stays exact and the two implementations are trivially in agreement.

/// Where a normalized character came from: which input piece, and the character
/// offset within that piece.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharSource {
    pub piece: usize,
    pub offset: usize,
}

/// Normalized text plus a per-character source map (one entry per `char`).
#[derive(Debug, Clone)]
pub struct Normalized {
    pub text: String,
    pub map: Vec<CharSource>,
}

/// True for characters treated as whitespace. This is Rust's Unicode
/// `White_Space` property; `normalize.ts` matches the same set explicitly.
fn is_ws(ch: char) -> bool {
    ch.is_whitespace()
}

/// Fold a single scalar to its ASCII match-equivalent, one-to-one. Non-folded
/// characters (including whitespace, handled separately) pass through.
fn fold(ch: char) -> char {
    match ch {
        '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{201B}' | '\u{2032}' | '\u{0060}'
        | '\u{00B4}' => '\'',
        '\u{201C}' | '\u{201D}' | '\u{201E}' | '\u{201F}' | '\u{2033}' => '"',
        '\u{2010}' | '\u{2011}' | '\u{2012}' | '\u{2013}' | '\u{2014}' | '\u{2015}'
        | '\u{2212}' => '-',
        other => other,
    }
}

/// Normalize an ordered list of text pieces into one coordinate system.
pub fn normalize_pieces(pieces: &[&str]) -> Normalized {
    let mut text = String::new();
    let mut map: Vec<CharSource> = Vec::new();
    let mut pending_ws: Option<CharSource> = None;

    for (pi, piece) in pieces.iter().enumerate() {
        for (ci, ch) in piece.chars().enumerate() {
            if is_ws(ch) {
                // Remember the first whitespace of a run as the source of the
                // single space we will emit - but never emit a leading space.
                if !text.is_empty() && pending_ws.is_none() {
                    pending_ws = Some(CharSource {
                        piece: pi,
                        offset: ci,
                    });
                }
                continue;
            }
            if let Some(src) = pending_ws.take() {
                text.push(' ');
                map.push(src);
            }
            text.push(fold(ch));
            map.push(CharSource {
                piece: pi,
                offset: ci,
            });
        }
    }
    // A trailing whitespace run is simply dropped (trim).

    Normalized { text, map }
}

/// Normalize a standalone string (e.g. a stored quote) to the same form, without
/// a source map.
pub fn normalize_str(s: &str) -> String {
    normalize_pieces(&[s]).text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapses_whitespace_and_trims() {
        let n = normalize_pieces(&["  the   quick\n\tbrown  "]);
        assert_eq!(n.text, "the quick brown");
        assert_eq!(n.map.len(), n.text.chars().count());
    }

    #[test]
    fn folds_smart_punctuation_one_to_one() {
        let n = normalize_pieces(&["\u{201C}don\u{2019}t\u{201D}\u{2014}really"]);
        assert_eq!(n.text, "\"don't\"-really");
    }

    #[test]
    fn joins_pieces_with_a_space_when_whitespace_between() {
        // Two text nodes: "foo " and "bar" -> "foo bar".
        let n = normalize_pieces(&["foo ", "bar"]);
        assert_eq!(n.text, "foo bar");
        // The 'b' maps to piece 1, offset 0.
        let b_index = n.text.chars().position(|c| c == 'b').unwrap();
        assert_eq!(
            n.map[b_index],
            CharSource {
                piece: 1,
                offset: 0
            }
        );
    }

    #[test]
    fn map_points_at_source_offsets() {
        let n = normalize_pieces(&["Hello  world"]);
        assert_eq!(n.text, "Hello world");
        // 'w' is raw offset 7 (after "Hello" + two spaces).
        let w_index = n.text.chars().position(|c| c == 'w').unwrap();
        assert_eq!(
            n.map[w_index],
            CharSource {
                piece: 0,
                offset: 7
            }
        );
    }
}
