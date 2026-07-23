//! Re-location: given a snapshot's normalized text and a highlight's selectors,
//! find the character range. The ladder (docs/04-anchoring.md), first match wins:
//!
//!   1. position fast-path - the offsets captured against this snapshot;
//!   2. exact quote search, disambiguated by prefix/suffix and nearest offset;
//!   3. context search - locate prefix + suffix, take the text between;
//!   4. fuzzy quote - Levenshtein within ~5% of the quote length.
//!
//! No match within tolerance leaves the highlight orphaned (still shown, just
//! not painted on the snapshot).

use crate::reader::normalize::normalize_str;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnchorMethod {
    Position,
    Exact,
    Context,
    Fuzzy,
}

#[derive(Debug, Clone, Copy)]
pub struct Located {
    /// Inclusive start / exclusive end, as character indices into the text.
    pub start: usize,
    pub end: usize,
    pub method: AnchorMethod,
}

pub struct Selectors<'a> {
    pub quote_exact: &'a str,
    pub quote_prefix: &'a str,
    pub quote_suffix: &'a str,
    pub pos_start: Option<i64>,
    pub pos_end: Option<i64>,
}

fn to_chars(s: &str) -> Vec<char> {
    normalize_str(s).chars().collect()
}

fn window_eq(hay: &[char], at: usize, needle: &[char]) -> bool {
    at + needle.len() <= hay.len() && hay[at..at + needle.len()] == *needle
}

fn find_all(hay: &[char], needle: &[char]) -> Vec<usize> {
    if needle.is_empty() || needle.len() > hay.len() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for start in 0..=(hay.len() - needle.len()) {
        if hay[start..start + needle.len()] == *needle {
            out.push(start);
        }
    }
    out
}

pub fn locate(hay: &[char], sel: &Selectors) -> Option<Located> {
    let quote = to_chars(sel.quote_exact);
    if quote.is_empty() {
        return None;
    }
    let prefix = to_chars(sel.quote_prefix);
    let suffix = to_chars(sel.quote_suffix);

    // 1. Position fast-path.
    if let (Some(s), Some(e)) = (sel.pos_start, sel.pos_end)
        && s >= 0
        && e >= s
    {
        let (s, e) = (s as usize, e as usize);
        if e <= hay.len() && hay[s..e] == quote[..] {
            return Some(Located { start: s, end: e, method: AnchorMethod::Position });
        }
    }

    // 2. Exact quote search.
    let occurrences = find_all(hay, &quote);
    if occurrences.len() == 1 {
        let s = occurrences[0];
        return Some(Located { start: s, end: s + quote.len(), method: AnchorMethod::Exact });
    }
    if occurrences.len() > 1 {
        let best = occurrences
            .iter()
            .copied()
            .max_by_key(|&s| {
                let mut score = 0i64;
                if !prefix.is_empty() && s >= prefix.len() && window_eq(hay, s - prefix.len(), &prefix)
                {
                    score += 2;
                }
                if !suffix.is_empty() && window_eq(hay, s + quote.len(), &suffix) {
                    score += 2;
                }
                // Tie-break: closeness to the recorded start (higher is better).
                let distance = sel.pos_start.map_or(0, |p| (s as i64 - p).abs());
                score * 1_000_000 - distance
            })
            .expect("non-empty");
        return Some(Located { start: best, end: best + quote.len(), method: AnchorMethod::Exact });
    }

    // 3. Context search: prefix ... suffix, take the span between.
    if !prefix.is_empty() && !suffix.is_empty() {
        let prefix_ends: Vec<usize> = find_all(hay, &prefix).into_iter().map(|s| s + prefix.len()).collect();
        let suffix_starts = find_all(hay, &suffix);
        let mut best: Option<(usize, usize)> = None;
        for &pe in &prefix_ends {
            if let Some(&ss) = suffix_starts.iter().filter(|&&ss| ss >= pe).min() {
                let candidate = (pe, ss);
                let candidate_len = ss - pe;
                let target = quote.len();
                let score = candidate_len.abs_diff(target);
                if best.is_none_or(|(bpe, bss)| score < (bss - bpe).abs_diff(target)) {
                    best = Some(candidate);
                }
            }
        }
        if let Some((s, e)) = best {
            return Some(Located { start: s, end: e, method: AnchorMethod::Context });
        }
    }

    // 4. Fuzzy quote within ~5% Levenshtein tolerance.
    let tolerance = (quote.len() / 20).max(1);
    let quote_str: String = quote.iter().collect();
    let mut best: Option<(usize, usize, usize)> = None; // (distance, start, end)
    let min_len = quote.len().saturating_sub(tolerance).max(1);
    let max_len = (quote.len() + tolerance).min(hay.len());
    for len in min_len..=max_len {
        if len > hay.len() {
            break;
        }
        for start in 0..=(hay.len() - len) {
            let window: String = hay[start..start + len].iter().collect();
            let distance = strsim::levenshtein(&window, &quote_str);
            if distance <= tolerance && best.is_none_or(|(bd, ..)| distance < bd) {
                best = Some((distance, start, start + len));
                if distance == 0 {
                    break;
                }
            }
        }
    }
    if let Some((_, s, e)) = best {
        return Some(Located { start: s, end: e, method: AnchorMethod::Fuzzy });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chars(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    #[test]
    fn position_fast_path() {
        let hay = chars("the quick brown fox");
        let sel = Selectors {
            quote_exact: "quick",
            quote_prefix: "",
            quote_suffix: "",
            pos_start: Some(4),
            pos_end: Some(9),
        };
        let got = locate(&hay, &sel).unwrap();
        assert_eq!((got.start, got.end), (4, 9));
        assert_eq!(got.method, AnchorMethod::Position);
    }

    #[test]
    fn exact_when_position_wrong() {
        let hay = chars("the quick brown fox");
        let sel = Selectors {
            quote_exact: "brown",
            quote_prefix: "",
            quote_suffix: "",
            pos_start: Some(999),
            pos_end: Some(1004),
        };
        let got = locate(&hay, &sel).unwrap();
        assert_eq!(&"the quick brown fox"[got.start..got.end], "brown");
        assert_eq!(got.method, AnchorMethod::Exact);
    }

    #[test]
    fn disambiguates_duplicates_by_context() {
        let hay = chars("bank of the river and the bank of money");
        let sel = Selectors {
            quote_exact: "bank",
            quote_prefix: "the ",
            quote_suffix: " of money",
            pos_start: None,
            pos_end: None,
        };
        let got = locate(&hay, &sel).unwrap();
        assert_eq!(got.start, 26); // the second "bank"
    }

    #[test]
    fn fuzzy_when_quote_lightly_edited() {
        let hay = chars("the quick brown fox jumps");
        let sel = Selectors {
            quote_exact: "quick browne", // one extra char
            quote_prefix: "",
            quote_suffix: "",
            pos_start: None,
            pos_end: None,
        };
        let got = locate(&hay, &sel).unwrap();
        assert_eq!(got.method, AnchorMethod::Fuzzy);
        assert!(&"the quick brown fox jumps"[got.start..got.end].contains("quick brown"));
    }

    #[test]
    fn orphaned_when_nothing_matches() {
        let hay = chars("completely unrelated content here");
        let sel = Selectors {
            quote_exact: "xyzzy plugh frobnicate",
            quote_prefix: "",
            quote_suffix: "",
            pos_start: None,
            pos_end: None,
        };
        assert!(locate(&hay, &sel).is_none());
    }
}
