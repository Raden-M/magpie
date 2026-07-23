// Text normalization - the shared coordinate system.
//
// This MUST stay byte-for-byte identical to the backend's
// `backend/src/reader/normalize.rs`. Positions (`pos_start`/`pos_end`) and
// quotes computed here at capture are re-derived by the backend at render; any
// divergence puts highlights in the wrong place. See docs/04-anchoring.md.
//
// The algorithm, over plain text in document order:
//   1. collapse every run of Unicode whitespace to a single space, and trim;
//   2. fold smart quotes, dashes, and non-breaking spaces to ASCII (matching
//      only - displayed text keeps its originals).
// No full Unicode NFC is applied, so the two implementations agree exactly and
// the per-character source map stays exact.

export interface CharSource {
  /** Index of the source text piece. */
  piece: number;
  /** Character (code-point) offset within that piece. */
  offset: number;
}

export interface Normalized {
  text: string;
  /** One entry per character in `text`. */
  map: CharSource[];
}

// Unicode `White_Space` code points - exactly the set Rust's
// `char::is_whitespace` recognizes. Note U+200B and U+FEFF are NOT whitespace.
const WHITESPACE = new Set<number>([
  0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x20, 0x85, 0xa0, 0x1680, 0x2000, 0x2001, 0x2002,
  0x2003, 0x2004, 0x2005, 0x2006, 0x2007, 0x2008, 0x2009, 0x200a, 0x2028, 0x2029,
  0x202f, 0x205f, 0x3000,
]);

function isWhitespace(codePoint: number): boolean {
  return WHITESPACE.has(codePoint);
}

// One-to-one fold of a single code point to its ASCII match-equivalent.
function fold(ch: string): string {
  switch (ch.codePointAt(0)) {
    case 0x2018:
    case 0x2019:
    case 0x201a:
    case 0x201b:
    case 0x2032:
    case 0x0060:
    case 0x00b4:
      return "'";
    case 0x201c:
    case 0x201d:
    case 0x201e:
    case 0x201f:
    case 0x2033:
      return '"';
    case 0x2010:
    case 0x2011:
    case 0x2012:
    case 0x2013:
    case 0x2014:
    case 0x2015:
    case 0x2212:
      return "-";
    default:
      return ch;
  }
}

/** Normalize an ordered list of text pieces into one coordinate system. */
export function normalizePieces(pieces: string[]): Normalized {
  let text = "";
  const map: CharSource[] = [];
  let pendingWs: CharSource | null = null;

  for (let piece = 0; piece < pieces.length; piece++) {
    let offset = 0;
    // `for...of` iterates by code point, matching Rust's `char` iteration.
    for (const ch of pieces[piece]) {
      const cp = ch.codePointAt(0)!;
      if (isWhitespace(cp)) {
        if (text.length > 0 && pendingWs === null) {
          pendingWs = { piece, offset };
        }
        offset++;
        continue;
      }
      if (pendingWs !== null) {
        text += " ";
        map.push(pendingWs);
        pendingWs = null;
      }
      text += fold(ch);
      map.push({ piece, offset });
      offset++;
    }
  }

  return { text, map };
}

/** Normalize a standalone string to the same form. */
export function normalizeStr(s: string): string {
  return normalizePieces([s]).text;
}
