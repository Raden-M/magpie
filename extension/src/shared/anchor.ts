// Anchoring computation at capture time.
//
// Positions must be offsets into the normalized plain text of the *extracted
// reader content* - not the live page (docs/05, step 1.3). So we locate the
// selected quote inside the extracted content's normalized text, which puts
// pos_start/pos_end in the same coordinate system the backend re-derives at
// render. The live selection only supplies the quote text and surrounding
// context (used to disambiguate duplicates).

import { normalizePieces, normalizeStr } from "./normalize.js";
import type { HighlightSelectors } from "./types.js";

const CONTEXT = 32;
const IGNORED_TAGS = new Set(["SCRIPT", "STYLE", "TEMPLATE", "HEAD", "NOSCRIPT"]);

export interface SelectionContext {
  /** The raw selected text. */
  quote: string;
  /** ~32 chars of live text immediately before the selection. */
  prefix: string;
  /** ~32 chars of live text immediately after the selection. */
  suffix: string;
}

export function collectTextNodes(root: Node): { nodes: Text[]; pieces: string[] } {
  const nodes: Text[] = [];
  const pieces: string[] = [];
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT, {
    acceptNode(node) {
      const parent = node.parentElement;
      if (parent && IGNORED_TAGS.has(parent.tagName)) return NodeFilter.FILTER_REJECT;
      return NodeFilter.FILTER_ACCEPT;
    },
  });
  let node = walker.nextNode();
  while (node) {
    nodes.push(node as Text);
    pieces.push((node as Text).data);
    node = walker.nextNode();
  }
  return { nodes, pieces };
}

/** The normalized plain text of an extracted-content root (for FTS + anchoring). */
export function normalizedTextOf(root: Node): string {
  return normalizePieces(collectTextNodes(root).pieces).text;
}

function findAll(hay: string[], needle: string[]): number[] {
  const out: number[] = [];
  if (needle.length === 0 || needle.length > hay.length) return out;
  outer: for (let i = 0; i <= hay.length - needle.length; i++) {
    for (let j = 0; j < needle.length; j++) {
      if (hay[i + j] !== needle[j]) continue outer;
    }
    out.push(i);
  }
  return out;
}

function windowEquals(hay: string[], at: number, needle: string[]): boolean {
  if (at < 0 || at + needle.length > hay.length) return false;
  for (let j = 0; j < needle.length; j++) {
    if (hay[at + j] !== needle[j]) return false;
  }
  return true;
}

function disambiguate(
  hay: string[],
  occurrences: number[],
  prefix: string[],
  suffix: string[],
  quoteLen: number,
): number {
  let best = occurrences[0];
  let bestScore = -1;
  for (const start of occurrences) {
    let score = 0;
    if (prefix.length > 0 && windowEquals(hay, start - prefix.length, prefix)) score += 2;
    if (suffix.length > 0 && windowEquals(hay, start + quoteLen, suffix)) score += 2;
    if (score > bestScore) {
      bestScore = score;
      best = start;
    }
  }
  return best;
}

function nearestMagId(node: Node | null): string | null {
  let el = node instanceof Element ? node : node?.parentElement ?? null;
  while (el) {
    const id = el.getAttribute("data-mag-id");
    if (id) return id;
    el = el.parentElement;
  }
  return null;
}

/**
 * Compute the anchoring selectors for a selection against extracted content.
 * `root` is the (already sanitized, id-assigned) extracted content DOM.
 */
export function computeSelectors(
  root: Element,
  sel: SelectionContext,
  color: string,
  note: string | null,
): HighlightSelectors {
  const { nodes, pieces } = collectTextNodes(root);
  const normalized = normalizePieces(pieces);
  const hay = Array.from(normalized.text);

  const quoteChars = Array.from(normalizeStr(sel.quote));
  const prefixChars = Array.from(normalizeStr(sel.prefix));
  const suffixChars = Array.from(normalizeStr(sel.suffix));

  let posStart: number | null = null;
  const occurrences = findAll(hay, quoteChars);
  if (occurrences.length === 1) {
    posStart = occurrences[0];
  } else if (occurrences.length > 1) {
    posStart = disambiguate(hay, occurrences, prefixChars, suffixChars, quoteChars.length);
  }
  const posEnd = posStart === null ? null : posStart + quoteChars.length;

  let quotePrefix: string;
  let quoteSuffix: string;
  let sectionAnchor: string | null = null;
  if (posStart !== null && posEnd !== null) {
    quotePrefix = hay.slice(Math.max(0, posStart - CONTEXT), posStart).join("");
    quoteSuffix = hay.slice(posEnd, posEnd + CONTEXT).join("");
    const source = normalized.map[posStart];
    if (source) sectionAnchor = nearestMagId(nodes[source.piece] ?? null);
  } else {
    // Couldn't locate in extracted content; keep the live context so the
    // backend still has robust selectors to try.
    quotePrefix = prefixChars.slice(-CONTEXT).join("");
    quoteSuffix = suffixChars.slice(0, CONTEXT).join("");
  }

  return {
    quote_exact: sel.quote.trim(),
    quote_prefix: quotePrefix,
    quote_suffix: quoteSuffix,
    pos_start: posStart,
    pos_end: posEnd,
    section_anchor: sectionAnchor,
    color,
    note,
  };
}
